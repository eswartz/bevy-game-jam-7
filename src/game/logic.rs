use std::time::Duration;

use crate::{assets::*};
use crate::player_spawning::spawn_player;
use crate::game::*;
use crate::common::*;

use bevy_seedling::sample::PlaybackSettings;
use bevy::asset::uuid::Uuid;
use bevy::ecs::world::CommandQueue;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;
use bevy_asset_loader::loading_state::config::{ConfigureLoadingState as _, LoadingStateConfig};
use bevy_egui::input::egui_wants_any_keyboard_input;
use bevy_seedling::prelude::*;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::{
    gltf::GltfMeshName,
    scene::SceneInstanceReady,
};
use bevy_tweening::lens::TransformScaleLens;
use bevy_tweening::*;
use rand::RngExt as _;
use rand::seq::IndexedRandom as _;

pub struct LogicPlugin;

impl Plugin for LogicPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                FixedUpdate,
                (
                    check_ball_catch,
                    check_ball_consumer,
                    check_ball_deathbox,
                )
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(not(is_user_paused))
                .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (
                    shake_base,
                    spawn_ball,
                )
                .run_if(not(is_paused))
                .run_if(not(is_in_menu))
                .run_if(not(egui_wants_any_keyboard_input))
                .run_if(in_state(LevelState::Playing))
                .run_if(in_state(ProgramState::InGame))
            )
        ;
    }
}

pub(crate) fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
    listener_q: Query<&Transform, With<SpatialListener3D>>,
    delay: Res<SpawnDelay>,
    time: Res<Time<Physics>>,
    spawning: Res<Spawning>,
    fx: Res<FxAssets>,
    models: Res<ModelAssets>,
    mut timer: ResMut<SpawnTimer>,
) {
    if !spawning.0 {
        return;
    }

    if timer.duration().is_zero() {
        timer.0 = Timer::from_seconds(delay.0.as_secs_f32(), TimerMode::Repeating);
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
            SceneRoot(models.sphere.clone()),
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


pub(crate) fn shake_base(
    base: Res<Base>,
    shake: Option<Res<ShakeRequest>>,
    camera: Query<&GlobalTransform, (With<Camera3d>, With<WorldCamera>)>,
    mut commands: Commands,
    mut forces: Query<(&Transform, Forces)>,
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
            let diff = xfrm.translation - base.1.translation;
            let vel = forces.linear_velocity();
            if vel.length() > 0.0001 {
                let force = -(vel + diff) * 1000.0;
                forces.apply_local_linear_impulse(force);
            }
        }
    }
}

pub(crate) fn check_ball_consumer(
    mut commands: Commands,
    parent_q: Query<&ChildOf>,
    spawned_q: Query<&Spawned, Without<Ignored>>,
    scene_q: Query<&SceneRoot>,
    sensor_q: Query<(&Transform, &CollidingEntities), (With<ConsumerCollider>, With<Sensor>)>,
    fx: Res<FxAssets>,
) {
    let mut rng = rand::rng();
    for (xfrm, coll) in sensor_q.iter() {
        for ent in coll.iter() {
            let mut parent = *ent;
            loop {
                if spawned_q.contains(parent) {
                    commands.spawn((
                        UiSfx,
                        SamplePlayer::new(
                            // (*[&fx.belch_1, &fx.belch_2, &fx.belch_3]
                            (*[&fx.loss]
                                .choose(&mut rng)
                                .unwrap())
                            .clone(),
                        ),
                        PlaybackSettings {
                            speed: rng.random_range(0.9..1.1),
                            ..default()
                        },
                        VolumeNode::from_linear(rng.random_range(0.85..1.0)),
                    ));

                    let xfrm_tween = Tween::new(
                        EaseMethod::EaseFunction(EaseFunction::BackOut),
                        Duration::from_secs_f32(1.0),
                        TransformScaleLens {
                            start: xfrm.scale,
                            end: Vec3::splat(0.001),
                        }
                    );
                    commands.entity(parent).try_insert((
                        Ignored,
                    ));
                    commands.entity(*ent).try_insert((
                        // Make static so it won't move by physics
                        RigidBody::Static,
                        DespawnAfter(xfrm_tween.cycle_duration()),
                        TweenAnim::new(xfrm_tween).with_destroy_on_completed(true),
                    ));

                    break;
                }
                if scene_q.contains(parent) {
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

pub(crate) fn check_ball_deathbox(
    mut commands: Commands,
    parent_q: Query<&ChildOf>,
    spawned_q: Query<&Spawned, Without<Ignored>>,
    sensor_q: Query<&CollidingEntities, (With<DeathboxCollider>, With<Sensor>)>,
) {
    for coll in sensor_q.iter() {
        for ent in coll.iter() {
            let mut parent = *ent;
            loop {
                if spawned_q.contains(parent) {
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

pub(crate) fn check_ball_catch(
    mut reader: MessageReader<CollisionEnd>,
    mut commands: Commands,
    net_q: Query<Entity, With<NetCollider>>,
    spawned_q: Query<(Entity, &Transform, &GlobalTransform), (With<Spawned>, Without<Ignored>)>, // toplevel
    scene_q: Query<&SceneRoot>,
    parent_q: Query<&ChildOf>,
    fx: Res<FxAssets>,
) {
    let Some(net) = net_q.iter().next() else {
        return;
    };

    let mut rng = rand::rng();
    for event in reader.read() {
        if event.collider1 == net || event.collider2 == net {
            // Caught something...
            let not_net = if event.collider1 == net { event.collider2 } else { event.collider1 };

            let mut ball = None;
            let mut ball_xfrm = None;
            let mut ball_gxfrm = None;
            for parent in parent_q.iter_ancestors(not_net) {
                if let Ok((ent, xfrm, gxfrm)) = spawned_q.get(parent) {
                    ball = Some(ent);
                    ball_xfrm = Some(xfrm);
                    ball_gxfrm = Some(gxfrm);
                    break;
                }
                if scene_q.contains(parent) {
                    break;
                }
            }

            if let Some(ball_gxfrm) = ball_gxfrm
            && let Some(ball_xfrm) = ball_xfrm
            && let Some(ball) = ball {
                // Animate "catching" the ball.

                // Leads to panics sometiems
                // commands.entity(not_net).try_remove::<RigidBody>();
                let xfrm_tween = Tween::new(
                    EaseMethod::EaseFunction(EaseFunction::BackOut),
                    Duration::from_secs_f32(1.0),
                    TransformScaleLens {
                        start: ball_xfrm.scale,
                        end: Vec3::splat(0.001),
                    }
                );
                commands.entity(not_net).try_insert((
                    Ignored,
                ));
                commands.entity(ball).try_insert((
                    // Make static so it won't move by physics
                    RigidBody::Static,
                    DespawnAfter(xfrm_tween.cycle_duration()),
                    TweenAnim::new(xfrm_tween).with_destroy_on_completed(true),
                    AimForCamera,
                ));

                commands.spawn((
                    Sfx,
                    // Transform::from_translation(ball_gxfrm.translation()),
                    ball_gxfrm.clone(),
                    SamplePlayer::new(
                        (*[
                            // &fx.action,
                            // &fx.action_rev,
                            &fx.swish,
                            // &fx.snap_2, &fx.snap_3
                            ]
                            .choose(&mut rng)
                            .unwrap())
                        .clone(),
                    ),

                    PlaybackSettings {
                        speed: rng.random_range(0.75..1.25),
                        ..default()
                    },
                    VolumeNode::from_linear(rng.random_range(0.5..1.0)),

                    sample_effects![SpatialBasicNode::default()],
                ));
            }
        }
    }
}
