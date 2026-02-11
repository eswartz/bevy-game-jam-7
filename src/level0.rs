use std::time::Duration;

use crate::assets::*;
use crate::game::{Base, Generator, LevelInfo, LevelList, ShakeRequest, ShakeTime, ShakingSound, SpawnDelay, Spawned, Spawning, is_in_level};
use crate::player_spawning::spawn_player;
use crate::common::*;

use bevy::asset::uuid::Uuid;
use bevy::audio::PlaybackSettings;
use bevy::ecs::world::CommandQueue;
use bevy_seedling::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use rand::RngExt;
use rand::seq::IndexedRandom;


use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::{
    gltf::GltfMeshName,
    scene::SceneInstanceReady,
};
use bevy_egui::input::egui_wants_any_keyboard_input;

pub(crate) const ID: &str = "level0";
pub(crate) const NAME: &str = "Level 0";

pub struct Level0Plugin;

impl Plugin for Level0Plugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnExit(GameplayState::AssetsLoaded),
                register_level
            )
            .add_systems(
                Update,
                (
                    check_ball_death,
                    check_ball_collisions,
                )
                .run_if(is_in_level(ID))
                .run_if(not(is_user_paused))
                .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (shake_base, check_actions)
                    .run_if(is_in_level(ID))
                    .run_if(not(is_paused))
                    .run_if(not(is_in_menu))
                    .run_if(not(egui_wants_any_keyboard_input))
                    .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(FixedUpdate,
                (
                    spawn_ball,
                )
                .run_if(is_in_level(ID))
                .run_if(in_state(LevelState::Playing))
            )
        ;
    }
}

fn register_level(mut list: ResMut<LevelList>, maps: Res<MapAssets>) {
    list.0.push(LevelInfo {
        id: ID.to_string(),
        label: NAME.to_string(),
        scene: maps.level_test.clone()
    });
}

fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
    listener_q: Query<&Transform, With<SpatialListener3D>>,
    delay: Res<SpawnDelay>,
    time: Res<Time<Physics>>,
    assets: Res<AssetServer>,
    spawning: Res<Spawning>,
    fx: Res<FxAssets>,
    mut timer: Local<Timer>,
) {
    if !spawning.0 {
        return;
    }

    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(delay.0.as_secs_f32(), TimerMode::Repeating);
    }
    if !timer.tick(time.delta()).just_finished() {
        timer.set_duration(delay.0); // in case it changed
        return;
    }

    // Fetch the spatializer location to avoid miscalculation.
    // To avoid https://github.com/CorvusPrudens/bevy_seedling/issues/87
    let spat_xfrm_opt = listener_q.iter().next();

    let mut rng = rand::rng();

    for (_ent, xfrm) in generator_q.iter() {
        commands.spawn((
            SceneRoot(assets.load(
                GltfAssetLabel::Scene(0).from_asset("sphere.glb"),
                )),
            xfrm.with_scale(Vec3::splat(time.elapsed_secs() % 1.0 + 0.5)),
            Spawned,
        ))
        // .observe(observe_spawn_mesh)
        ;
        commands.spawn((
            Sfx,
            // Make into spatial sound.
            Transform::from_translation(xfrm.translation),
            // To avoid https://github.com/CorvusPrudens/bevy_seedling/issues/87
            sample_effects![SpatialBasicNode {
                offset: (if let Some(spat_xfrm) = spat_xfrm_opt {
                    spat_xfrm.translation - xfrm.translation
                } else {
                    Vec3::new(10.0, 10.0, 10.0)
                })
                .into(),
                ..default()
            }],
            // Another workaround: choose one of a similar sound
            // else the pool tends to get stuck and not play anything.
            SamplePlayer::new(
                (*[&fx.belch_1, &fx.belch_2, &fx.belch_3]
                    .choose(&mut rng)
                    .unwrap())
                .clone(),
            ),
            // SamplePlayer::new(fx.tone.clone()),
            PlaybackSettings {
                speed: rng.random_range(0.75..1.25),
                ..default()
            },
            VolumeNode::from_linear(rng.random_range(0.1..1.0)),
        ));
    }
}

fn check_actions(
    // keys: Res<ButtonInput<KeyCode>>,
    actions: Res<ActionState<UserAction>>,
    fx: Res<FxAssets>,
    base: Res<Base>,
    time: Res<Time<Physics>>,
    shake_q: Query<Entity, With<ShakingSound>>,
    mut shake_time: ResMut<ShakeTime>,
    spawning: Res<Spawning>,
    overlay: Res<State<OverlayState>>,

    mut commands: Commands,
    mut forces: Query<Forces>,
) {
    if actions.just_released(&UserAction::Interact) {
        let new_state = !spawning.0;
        let sample = if new_state {
            fx.on.clone()
        } else {
            fx.off.clone()
        };
        commands.spawn((
            UiSfx,
            SamplePlayer::new(sample),
        ));
        commands.insert_resource(Spawning(new_state))
    }
    if actions.just_released(&UserAction::AlternateFire) {
        if let Ok(mut force) = forces.get_mut(base.0) {
            // dbg!(base.0);
            let mut x = 1000.0 * (time.elapsed_secs() % 5.0);
            if time.elapsed_secs() % 1.0 < 0.5 {
                x = -x;
            }
            force.apply_local_linear_impulse(Vec3::new(x, 0.0, -x));
        }
    }

    let mut new_shake = Vec3::ZERO;
    if let Some(move_lr) = actions.axis_data(&UserAction::MoveLeftRight2d) {
        new_shake.x = move_lr.value;
    }
    if let Some(move_ud) = actions.axis_data(&UserAction::MoveDownUp2d) {
        new_shake.y = move_ud.value;
    }
    if new_shake.length() > 0.0 {
        commands.insert_resource(ShakeRequest(new_shake * time.delta_secs()));

        if shake_q.single().is_err() {
            // Start sound.
            commands.spawn((
                UiSfx,
                ShakingSound,
                SamplePlayer::new(fx.sloshing.clone()),
            ));
        }
        shake_time.0 += time.delta();
    } else {
        // Remove sound after enough non-shaking.
        if !shake_time.0.is_zero() {
            shake_time.0 = shake_time.0.saturating_sub(time.delta());
            if shake_time.0.is_zero() {
                if let Ok(ent) = shake_q.single() {
                    commands.entity(ent).try_despawn();
                }
            }
        }
    }
}

fn shake_base(
    base: Res<Base>,
    shake: Option<Res<ShakeRequest>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut commands: Commands,
    mut forces: Query<(&Transform, Forces)>,
    mut shaking: Local<bool>,
) {
    if let Ok((xfrm, mut forces)) = forces.get_mut(base.0) {
        if let Some(shake) = shake {
            // Apply shake.
            if let Ok(xfrm) = camera.single() {
                let force = shake.0 * 10000.0;
                forces.apply_local_linear_impulse(xfrm.rotation() * force);
                commands.remove_resource::<ShakeRequest>();

            }
        } else {
            // Come to rest.
            *shaking = false;
            let diff = xfrm.translation - base.1.translation;
            let vel = forces.linear_velocity();
            if vel.length() > 0.0001 {
                let force = -(vel + diff) * 100.0;
                // dbg!(&force);
                // forces.apply_local_force(force);
                forces.apply_local_linear_impulse(force);
            }
        }
    }
}

fn check_ball_death(
    mut commands: Commands,
    parent_q: Query<&ChildOf>,
    spawned_q: Query<&Spawned>,
    // scene_q: Query<&SceneRoot>,
    sensor_q: Query<&CollidingEntities, With<Sensor>>,
) {
    for coll in sensor_q.iter() {
        for ent in coll.iter() {
            let mut parent = *ent;
            loop {
                if spawned_q.contains(parent) {
                    // dbg!(ent, parent);
                    commands.entity(parent).despawn();
                    break;
                }
                if let Ok(parent0) = parent_q.get(parent) {
                    parent = parent0.0;
                } else {
                    break;
                }
            }
        }
    }
}

fn check_ball_collisions(
    mut commands: Commands,
    coll_q: Query<(Entity, &Transform, &LinearVelocity), With<Collider>>,
    collisions: Collisions,
    spawned_q: Query<&Spawned>,
    scene_q: Query<&SceneRoot>,
    parent_q: Query<&ChildOf>,
    listener_q: Query<&Transform, With<SpatialListener3D>>,
    fx: Res<FxAssets>,
) {
    let mut xfrms = vec![];

    for (ent, xfrm, vel) in coll_q.iter() {
        if vel.length() < 1.0 {
            continue;
        }

        if let Some(pair) = collisions.collisions_with(ent).next() {
            if pair.total_normal_impulse_magnitude() > 10.0 {
                for parent in parent_q.iter_ancestors(ent) {
                    if spawned_q.contains(parent) {
                        xfrms.push(xfrm.translation);
                        if xfrms.len() >= 3 {
                            break;
                        }
                    }
                    if scene_q.contains(parent) {
                        break;
                    }
                }
            }
        }
    }

    if xfrms.is_empty() {
        return;
    }

    // Fetch the spatializer location to avoid miscalculation.
    // To avoid https://github.com/CorvusPrudens/bevy_seedling/issues/87
    let spat_xfrm_opt = listener_q.iter().next();

    let mut rng = rand::rng();
    for position in xfrms {
        commands.spawn((
            Sfx,
            Transform::from_translation(position),
            SamplePlayer::new(
                (*[&fx.snap_1, &fx.snap_2, &fx.snap_3]
                    .choose(&mut rng)
                    .unwrap())
                .clone(),
            ),
            PlaybackSettings {
                speed: rng.random_range(0.75..1.25),
                ..default()
            },
            VolumeNode::from_linear(rng.random_range(0.1..0.25)),
            sample_effects![SpatialBasicNode {
                offset: (if let Some(spat_xfrm) = spat_xfrm_opt {
                    spat_xfrm.translation - position
                } else {
                    Vec3::new(10.0, 10.0, 10.0)
                })
                .into(),
                ..default()
            }],
        ));
    }
}
