use std::time::Duration;

use crate::{assets::*};
use crate::game::*;
use crate::common::*;

use bevy::camera::primitives::Aabb;
use bevy_egui::input::egui_wants_any_input;
use bevy_seedling::sample::PlaybackSettings;
use bevy_seedling::prelude::*;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tweening::lens::{TransformPositionLens, TransformScaleLens};
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
                    check_spawn_toggle,
                )
                .run_if(resource_exists::<Spawning>)
                .run_if(not(is_user_paused))
                .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (
                    check_ball_catch,
                    check_ball_loss,
                    check_player_out_of_bounds,
                )
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(resource_exists::<CurrentScore>)
                .run_if(not(is_user_paused))
                .run_if(in_state(LevelState::Playing))
                .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (
                    shake_base.run_if(resource_exists::<BaseEntity>),
                    spawn_ball,
                )
                .run_if(not(is_paused))
                .run_if(not(is_in_menu))
                .run_if(in_state(LevelState::Playing))
                .run_if(in_state(ProgramState::InGame))
            )

            .add_observer(observe_in_hand_anim)

            .add_systems(
                FixedUpdate,
                (
                    check_actions,
                )
                    .run_if(not(is_in_menu))
                    .run_if(is_level_active)
                    .run_if(not(is_paused))
                    .run_if(not(egui_wants_any_input))
                    .run_if(in_state(ProgramState::InGame))
                ,
            )
        ;
    }
}

/// Toggle counts when the collision starts then ends.
pub(crate) fn check_spawn_toggle(
    mut reader: MessageReader<CollisionEnd>,
    mut commands: Commands,
    switch_q: Query<Entity, With<GeneratorSwitchCollider>>,
    player_q: Query<(), With<Player>>,
    // parent_q: Query<&ChildOf>,
    mut spawning: ResMut<Spawning>,
    fx: Res<FxAssets>,
) {
    let Some(switch) = switch_q.iter().next() else {
        return;
    };

    for event in reader.read() {
        if event.collider1 == switch || event.collider2 == switch {
            // Caught something...
            let not_switch = if event.collider1 == switch { event.collider2 } else { event.collider1 };

            // dbg!(not_switch);
            // for parent in parent_q.iter_ancestors(not_switch) {
            //     dbg!(parent);
            if player_q.get(not_switch).is_ok() {
                let new_state = !spawning.0;
                spawning.0 = new_state;

                let sample = if new_state {
                    fx.on.clone()
                } else {
                    fx.off.clone()
                };
                commands.spawn((
                    UiSfx,
                    SamplePlayer::new(sample),
                ));

                break;
            }
        }
    }
}

/// Spawn a ball from [Generator].
/// Assigns the [Scoreable] from [LevelRoot].
pub(crate) fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
    balls_q: Query<&Spawned>,
    scoreable_q: Single<&Scoreable, With<LevelRoot>>,
    world: Res<WorldMarkerEntity>,
    mut delay: ResMut<SpawnDelay>,
    time: Res<Time<Physics>>,
    spawning: Res<Spawning>,
    fx: Res<FxAssets>,
    models: Res<ModelAssets>,
    difficulty: Res<LevelDifficulty>,
    mut timer: ResMut<SpawnTimer>,
) {
    if !spawning.0 {
        return;
    }

    if balls_q.count() >= 100 {
        return;
    }

    if timer.duration().is_zero() {
        timer.0 = Timer::from_seconds(delay.0.as_secs_f32(), TimerMode::Repeating);
    }
    if !timer.tick(time.delta()).just_finished() {
        let (decay, min) = match difficulty.0 {
            Difficulty::Easy => (0.99999, 1.0),
            Difficulty::Normal => (0.9999, 0.5),
            Difficulty::Hard => (0.999, 0.25),
        };
        delay.0 = delay.0.mul_f32(decay).max(Duration::from_secs_f32(min));

        timer.set_duration(delay.0);
        return;
    }

    let mut rng = rand::rng();

    let spawn_chance = match difficulty.0 {
        Difficulty::Easy => 0.2,
        Difficulty::Normal => 0.5,
        Difficulty::Hard => 0.8,
    };
    for (_ent, xfrm) in generator_q.iter() {
        if rng.random_bool(spawn_chance) {
            commands.spawn((
                DespawnAfter(Duration::from_secs(30)),
                ChildOf(world.0),
                SceneRoot(models.gold_ball.clone()),
                xfrm.with_scale(Vec3::splat(time.elapsed_secs() % 1.0 + 0.5)),
                Spawned,
                **scoreable_q,
            ))
        } else {
            commands.spawn((
                DespawnAfter(Duration::from_secs(30)),
                ChildOf(world.0),
                SceneRoot(models.cyan_ball.clone()),
                xfrm.with_scale(Vec3::splat(time.elapsed_secs() % 1.0 + 0.5)),
                Spawned,
                // not scoreable
            ))
        }
        ;
        commands.spawn((
            ChildOf(world.0),
            Sfx,
            // Make into spatial sound.
            Transform::from_translation(xfrm.translation),
            sample_effects![SpatialBasicNode::default()],
            SamplePlayer::new(
                (*[&fx.belch_1, &fx.belch_2, &fx.belch_3]
                    .choose(&mut rng)
                    .unwrap())
                .clone(),
            ),
            PlaybackSettings {
                speed: rng.random_range(0.75..1.25),
                ..default()
            },
            VolumeNode::from_linear(rng.random_range(0.1..1.0)),
        ));
    }
}

pub(crate) fn shake_base(
    base: Res<BaseEntity>,
    shake: Option<Res<ShakeRequest>>,
    camera: Query<&GlobalTransform, (With<Camera3d>, With<WorldCamera>)>,
    shake_q: Query<Entity, With<ShakingSound>>,
    aabb_q: Query<&Aabb>,
    fx: Res<FxAssets>,
    mut commands: Commands,
    mut forces: Query<(&Transform, Forces)>,
) {
    if let Ok((xfrm, mut forces)) = forces.get_mut(base.0) {
        if let Some(shake) = shake {
            // Apply shake.
            if let Ok(xfrm) = camera.single() {
                if shake_q.single().is_err() {
                    // Start sound.
                    commands.spawn((
                        UiSfx,
                        ShakingSound,
                        SamplePlayer::new(fx.sloshing.clone()),
                    ));
                }

                let size = match aabb_q.get(base.0) {
                    Ok(base) => 100.0 * base.half_extents.x * base.half_extents.y * base.half_extents.z,
                    Err(_) => 10000.0
                };
                let force = shake.0 * size;
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

pub(crate) fn check_player_out_of_bounds(
    mut commands: Commands,
    parent_q: Query<&ChildOf>,
    player_q: Query<&Transform, With<Player>>,
    scene_q: Query<&SceneRoot>,
    sensor_q: Query<&CollidingEntities, With<DeathboxCollider>>,
    player_start_q: Query<&Transform, With<PlayerStart>>,
    fx: Res<FxAssets>,
) {
    let mut rng = rand::rng();
    for coll in sensor_q.iter() {
        for ent in coll.iter() {
            let mut parent = *ent;
            loop {
                if let Ok(xfrm) = player_q.get(parent) {
                    commands.spawn((
                        UiSfx,
                        SamplePlayer::new(
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
                        TransformPositionLens {
                            start: xfrm.translation,
                            end: player_start_q.single().unwrap().translation,
                        }
                    );
                    commands.entity(*ent).try_insert((
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

pub(crate) fn check_ball_loss(
    mut commands: Commands,
    parent_q: Query<&ChildOf>,
    spawned_q: Query<&Spawned, Without<Ignored>>,
    scoreable_q: Query<&Scoreable>,
    scene_q: Query<&SceneRoot>,
    sensor_q: Query<(&Transform, &CollidingEntities), (
        Or<(With<ConsumerCollider>, With<DeathboxCollider>)>,
        With<Sensor>,
    )>,
    mut score: ResMut<CurrentScore>,
    fx: Res<FxAssets>,
) {
    let mut rng = rand::rng();
    for (xfrm, coll) in sensor_q.iter() {
        for ent in coll.iter() {
            let mut parent = *ent;
            loop {
                if spawned_q.contains(parent) {

                    // One we care about?
                    if let Ok(scoreable) = scoreable_q.get(parent) {
                        score.score -= scoreable.lose as i32;

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
                    }

                    // Regardless of scoring, animate it to be removed.
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

/// Catch counts when the collision starts then ends.
pub(crate) fn check_ball_catch(
    mut reader: MessageReader<CollisionEnd>,
    mut commands: Commands,
    player_catching_q: Query<(), (With<Player>, With<Catching>)>,
    net_q: Query<Entity, With<NetCollider>>,
    spawned_q: Query<(Entity, &Transform, &GlobalTransform), (With<Spawned>, Without<Ignored>)>, // toplevel
    scoreable_q: Query<&Scoreable>,
    scene_q: Query<&SceneRoot>,
    parent_q: Query<&ChildOf>,
    mut score: ResMut<CurrentScore>,
    fx: Res<FxAssets>,
) {
    if player_catching_q.single().is_err() {
        // Don't catch!
        return
    }

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
                    // Remember which one, so we can animate/delete it.
                    ball = Some(ent);
                    ball_xfrm = Some(xfrm);
                    ball_gxfrm = Some(gxfrm);

                    // Was it one we care about?
                    if let Ok(scoreable) = scoreable_q.get(parent) {
                        score.score += scoreable.gain as i32;

                        commands.spawn((
                            UiSfx,
                            SamplePlayer::new(
                                // (*[&fx.belch_1, &fx.belch_2, &fx.belch_3]
                                (*[&fx.gain]
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
                    }

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
                    Ignored,    // don't double-count
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

fn observe_in_hand_anim(on: On<AnimCompletedEvent>, mut commands: Commands,
    mut in_hand_q: Query<Option<&ColliderDisabled>, With<InHand>>,
) {
    let ent = on.event_target();
    if let Ok(coll_dis) = in_hand_q.get_mut(ent) {
        if coll_dis.is_some() {
            commands.entity(ent).insert(Visibility::Hidden);
        } else {
            commands.entity(ent).insert(Visibility::Inherited);
        }
    }
}

fn check_actions(
    actions: Res<ActionState<UserAction>>,
    fx: Res<FxAssets>,
    time: Res<Time<Physics>>,
    shake_q: Query<Entity, With<ShakingSound>>,
    player_q: Query<Entity, With<Player>>,
    mut in_hand_q: Query<(Entity, &Transform), With<InHand>>,
    tween_anim_q: Query<Entity, (With<TweenAnim>, With<InHand>)>,
    // fire_pressed: Option<Res<FirePressed>>,
    // fire_released: Option<Res<FireReleased>>,
    mut fire_pressed: Local<bool>,
    mut fire_released: Local<bool>,
    mut shake_time: ResMut<ShakeTime>,
    // spawning: Res<Spawning>,
    mut commands: Commands,
) {
    // if actions.just_released(&UserAction::Interact) {
    //     let new_state = !spawning.0;
    //     let sample = if new_state {
    //         fx.on.clone()
    //     } else {
    //         fx.off.clone()
    //     };
    //     commands.spawn((
    //         UiSfx,
    //         SamplePlayer::new(sample),
    //     ));
    //     commands.insert_resource(Spawning(new_state))
    // }

    let show = if actions.just_pressed(&UserAction::Fire) {
        if tween_anim_q.iter().next().is_none() {
            // Nothing animated.
            true
        } else {
            // Wait for later.
            *fire_pressed = true;
            false
        }
    } else {
        // Was it pressed before?
        if *fire_pressed {
            *fire_pressed = false;
            true
        } else {
            false
        }
    };

    let hide = if actions.just_released(&UserAction::Fire) {
        if tween_anim_q.iter().next().is_none() {
            // Nothing animated.
            true
        } else {
            // Wait for later.
            *fire_released = true;
            false
        }
    } else if actions.released(&UserAction::Fire) {
        // Was it released before?
        if *fire_released {
            *fire_released = false;
            true
        } else {
            false
        }
    } else {
        false
    };

    if show || hide {
        // Only one player...
        let Ok(player) = player_q.single() else {
            log::error!("no single Player");
            return;
        };
        if show {
            commands.entity(player).insert(Catching);
        } else if hide {
            commands.entity(player).remove::<Catching>();
        }

        let mut any = false;
        for (ent, xfrm) in in_hand_q.iter_mut() {
            let out_xfrm = Transform::from_xyz(0.0, -1.0, 1.0)
                            .with_scale(xfrm.scale);
            let in_xfrm = Transform::from_xyz(0.0, -0.5, -1.0)
                            .with_scale(xfrm.scale);

            let xfrm_tween = if show {
                // Going to appear.
                commands.entity(ent).remove::<ColliderDisabled>();
                commands.entity(ent).insert(Visibility::Inherited);
                Tween::new(
                    EaseMethod::EaseFunction(EaseFunction::BackOut),
                    Duration::from_secs_f32(1.0),
                    TransformPositionRotationLens {
                        start: out_xfrm.with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                        end: in_xfrm,
                    }
                )
            } else {
                commands.entity(ent).insert(ColliderDisabled);
                Tween::new(
                    EaseMethod::EaseFunction(EaseFunction::BackOut),
                    Duration::from_secs_f32(1.0),
                    TransformPositionRotationLens {
                        start: in_xfrm.with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                        end: out_xfrm,
                    }
                )
            };

            // Trigger observe_in_hand_anim when done.
            commands.entity(ent).insert((
                TweenAnim::new(xfrm_tween).with_destroy_on_completed(true),
            ));

            any = true;
        }
        if any {
            commands.spawn((
                UiSfx,
                SamplePlayer::new(fx.swoosh.clone()),
            ));
        }
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

    if actions.just_released(&UserAction::ForceLose) {
        commands.set_state(LevelState::Lost);
    }
    if actions.just_released(&UserAction::ForceWin) {
        commands.set_state(LevelState::Won);
    }
}
