use std::time::Duration;

use crate::assets::*;
use crate::game::{Base, Generator, LevelInfo, LevelList, Spawned, is_in_level};
use crate::common::*;

use bevy::audio::PlaybackSettings;
use bevy::camera::visibility::RenderLayers;
use bevy_seedling::prelude::*;
use bevy_tweening::lens::{TransformPositionLens, TransformScaleLens};
use bevy_tweening::{AnimCompletedEvent, EaseMethod, Tween, TweenAnim, Tweenable};
use leafwing_input_manager::prelude::ActionState;
use rand::RngExt;
use rand::seq::IndexedRandom;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_egui::input::egui_wants_any_keyboard_input;

pub(crate) const ID: &str = "level0";
pub(crate) const NAME: &str = "Level 0";

pub struct Level0Plugin;

impl Plugin for Level0Plugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnExit(GameplayState::AssetsLoaded),
                on_assets_loaded
            )
            .add_systems(
                OnEnter(LevelState::Loaded),
                on_level_loaded
            )
            .add_systems(
                FixedUpdate,
                (
                    check_ball_catch,
                    check_ball_death,
                    // check_ball_collisions,
                )
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(is_in_level(ID))
                .run_if(not(is_user_paused))
                .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (
                    shake_base,
                    check_actions,
                    spawn_ball,
                )
                .run_if(is_in_level(ID))
                .run_if(not(is_paused))
                .run_if(not(is_in_menu))
                .run_if(not(egui_wants_any_keyboard_input))
                .run_if(in_state(LevelState::Playing))
                .run_if(in_state(ProgramState::InGame))
            )
        ;
    }
}


/// Is spawning active?
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct Spawning(pub bool);

/// Delay between spawns.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct SpawnDelay(pub(crate) Duration);

/// Apply shaking from user action.
#[derive(Resource)]
pub(crate) struct ShakeRequest(pub(crate) Vec3);

/// How long some kind of shaking is active.
#[derive(Resource)]
pub(crate) struct ShakeTime(pub(crate) Duration);

/// Set while shaking sound active.
#[derive(Component)]
pub(crate) struct ShakingSound;

fn on_assets_loaded(mut list: ResMut<LevelList>, maps: Res<MapAssets>) {
    list.0.push(LevelInfo {
        id: ID.to_string(),
        label: NAME.to_string(),
        scene: maps.level_test.clone()
    });
}

/// Marker (in .glb) for the collider.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct NetCollider;

fn on_level_loaded(
    mut commands: Commands,
    models: Res<ModelAssets>,
    camera_q: Single<Entity, (With<Camera3d>, With<ViewerCamera>)>,
) {
    let net = commands.spawn((
        Name::new("Net"),
        RenderLayers::layer(RENDER_LAYER_VIEW),
        SceneRoot(models.net.clone()),
        Transform::from_xyz(0.0, 0.0, -1.0).with_scale(Vec3::splat(2.0)),
        Visibility::Visible,
    )).id();
    commands.entity(*camera_q).add_child(net);

    commands.insert_resource(Spawning(false));
    commands.insert_resource(SpawnDelay(Duration::from_secs(1)));
    commands.insert_resource(ShakeRequest(Vec3::ZERO));
    commands.insert_resource(ShakeTime(Duration::ZERO));

    commands.set_state(LevelState::Playing);
}

fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
    listener_q: Query<&Transform, With<SpatialListener3D>>,
    delay: Res<SpawnDelay>,
    time: Res<Time<Physics>>,
    spawning: Res<Spawning>,
    fx: Res<FxAssets>,
    models: Res<ModelAssets>,
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

fn check_actions(
    actions: Res<ActionState<UserAction>>,
    fx: Res<FxAssets>,
    time: Res<Time<Physics>>,
    shake_q: Query<Entity, With<ShakingSound>>,
    mut shake_time: ResMut<ShakeTime>,
    spawning: Res<Spawning>,
    mut commands: Commands,
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

    let mut rng = rand::rng();

    // Shake the base with left/right/up/down.
    let mut new_shake = Vec3::ZERO;
    if let Some(move_lr) = actions.axis_data(&UserAction::MoveLeftRight2d) {
        new_shake.x = move_lr.value;
    }
    if let Some(move_ud) = actions.axis_data(&UserAction::MoveDownUp2d) {
        new_shake.z = move_ud.value;
    }
    if new_shake.length() > 0. {
        new_shake.y = if rng.random_bool(0.5) { -1. } else { 1. };
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

// fn check_ball_collisions(
//     mut commands: Commands,
//     coll_q: Query<(Entity, &Transform, &LinearVelocity), With<Collider>>,
//     collisions: Collisions,
//     spawned_q: Query<&Spawned>,
//     scene_q: Query<&SceneRoot>,
//     parent_q: Query<&ChildOf>,
//     listener_q: Query<&Transform, With<SpatialListener3D>>,
//     fx: Res<FxAssets>,
// ) {
//     let mut xfrms = vec![];

//     for (ent, xfrm, vel) in coll_q.iter() {
//         if vel.length() < 1.0 {
//             continue;
//         }

//         if let Some(pair) = collisions.collisions_with(ent).next() {
//             if pair.total_normal_impulse_magnitude() > 10.0 {
//                 for parent in parent_q.iter_ancestors(ent) {
//                     if spawned_q.contains(parent) {
//                         xfrms.push(xfrm.translation);
//                         if xfrms.len() >= 3 {
//                             break;
//                         }
//                     }
//                     if scene_q.contains(parent) {
//                         break;
//                     }
//                 }
//             }
//         }
//     }

//     if xfrms.is_empty() {
//         return;
//     }

//     // Fetch the spatializer location to avoid miscalculation.
//     // To avoid https://github.com/CorvusPrudens/bevy_seedling/issues/87
//     let spat_xfrm_opt = listener_q.iter().next();

//     let mut rng = rand::rng();
//     for position in xfrms {
//         commands.spawn((
//             Sfx,
//             Transform::from_translation(position),
//             SamplePlayer::new(
//                 (*[&fx.snap_1, &fx.snap_2, &fx.snap_3]
//                     .choose(&mut rng)
//                     .unwrap())
//                 .clone(),
//             ),
//             PlaybackSettings {
//                 speed: rng.random_range(0.75..1.25),
//                 ..default()
//             },
//             VolumeNode::from_linear(rng.random_range(0.1..0.25)),
//             sample_effects![SpatialBasicNode {
//                 offset: (if let Some(spat_xfrm) = spat_xfrm_opt {
//                     spat_xfrm.translation - position
//                 } else {
//                     Vec3::new(10.0, 10.0, 10.0)
//                 })
//                 .into(),
//                 ..default()
//             }],
//         ));
//     }
// }

fn check_ball_catch(
    mut reader: MessageReader<CollisionEnd>,
    mut commands: Commands,
    net_q: Single<(Entity, &GlobalTransform), With<NetCollider>>,
    spawned_q: Query<(Entity, &Transform, &GlobalTransform), With<Spawned>>, // toplevel
    scene_q: Query<&SceneRoot>,
    parent_q: Query<&ChildOf>,
    listener_q: Query<&Transform, With<SpatialListener3D>>,
    ignored_q: Query<&Ignored>,
    camera_xfrm_q: Single<&GlobalTransform, (With<OurCamera>, With<WorldCamera>)>,
    fx: Res<FxAssets>,
) {
    // Fetch the spatializer location to avoid miscalculation.
    // To avoid https://github.com/CorvusPrudens/bevy_seedling/issues/87
    let spat_xfrm_opt = listener_q.iter().next();
    let mut rng = rand::rng();

    let (net, net_gxfrm) = *net_q;

    for event in reader.read() {
        if event.collider1 == net || event.collider2 == net {
            // Caught something...
            let not_net = if event.collider1 == net { event.collider2 } else { event.collider1 };
            if ignored_q.contains(not_net) {
                // Already handled.
                continue;
            }

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
                commands.entity(ball).try_insert((
                    // Make static so it won't move by physics
                    RigidBody::Static,
                    Ignored,
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

                    sample_effects![SpatialBasicNode {
                        offset: (if let Some(spat_xfrm) = spat_xfrm_opt {
                            spat_xfrm.translation - ball_gxfrm.translation()
                        } else {
                            Vec3::new(10.0, 10.0, 10.0)
                        })
                        .into(),
                        ..default()
                    }],
                ));
            }
        }
    }
}
