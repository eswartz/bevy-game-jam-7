use crate::assets::*;
use crate::player_spawning::spawn_player;
use crate::common::*;

use bevy::asset::uuid::Uuid;
use bevy::audio::PlaybackSettings;
use bevy::ecs::world::CommandQueue;
use bevy_seedling::prelude::*;
use rand::RngExt;
use rand::seq::IndexedRandom;


use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::{
    gltf::GltfMeshName,
    scene::SceneInstanceReady,
};
use bevy_egui::input::egui_wants_any_keyboard_input;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Spawning(false))
            .insert_resource(Base(Entity::PLACEHOLDER, Transform::IDENTITY))
            .add_observer(observe_spawn_mesh)
            // .add_systems(
            //     OnEnter(GameplayState::AssetsLoaded), on_enter_initializing)
            .add_systems(
                OnEnter(GameplayState::Setup),
                spawn_level, // .in_set(SimulationSystems)
                             // .run_if(in_state(ProgramState::InGame)) // redundant
            )
            .add_systems(
                OnEnter(GameplayState::Playing),
                spawn_player_on_start,
                // .in_set(SimulationSystems)
                // .run_if(in_state(ProgramState::InGame)) // redundant
            )
            .add_systems(
                Update,
                (
                    check_ball_death,
                    check_ball_collisions,
                    // move_camera_around,
                    // aim_camera_around,
                )
                    .run_if(not(is_user_paused))
                    .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                Update,
                (shake_base, check_actions)
                    .run_if(not(is_paused))
                    .run_if(not(egui_wants_any_keyboard_input))
                    .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(Update, (spawn_ball,).run_if(in_state(LevelState::Playing)));
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
struct Generator;

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
struct Clone;

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
struct Spawned;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
struct Base(pub Entity, pub Transform);

#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
struct Spawning(pub bool);

#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
struct Shake(pub Vec3);

fn observe_spawn_mesh(
    ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    names: Query<&Name>,
    gltf_names: Query<&GltfMeshName>,
    meshes: Query<&Mesh3d>,
    parent: Query<&ChildOf>,
    xfrms: Query<&Transform>,
    mut commands: Commands,
) {
    for entity in children.iter_descendants(ready.entity) {
        if meshes.contains(entity) {
            let owner_name_is = |name_str| -> bool {
                let mut from = entity;
                loop {
                    if let Ok(name) = names.get(from)
                        && name.eq_ignore_ascii_case(name_str)
                    {
                        return true;
                    }
                    if let Ok(p) = parent.get(from) {
                        from = p.parent();
                    } else {
                        return false;
                    }
                }
            };

            commands.entity(entity).insert((
                MaxLinearSpeed(256.0),
                CollisionLayers::new(
                    GameLayer::World,
                    [
                        GameLayer::Default,
                        GameLayer::World,
                        GameLayer::Player,
                        GameLayer::Projectiles,
                    ],
                ),
            ));

            if owner_name_is("Base") || owner_name_is("Tube") {
                // dbg!(entity);
                commands
                    .entity(entity)
                    .insert(ColliderConstructor::TrimeshFromMesh);
            }

            if let Ok(gltf_name) = gltf_names.get(entity) {
                // dbg!(gltf_name);
                if gltf_name.0.eq_ignore_ascii_case("BaseX") {
                    commands.insert_resource(Base(entity, xfrms.get(entity).unwrap().clone()))
                }
            }
        }
    }
}

pub(crate) fn spawn_player_on_start(world: &mut World) {
    let ent = spawn_player(world, Uuid::default());

    let mut start_q = world.query_filtered::<&Transform, (With<PlayerStart>, Without<OurPlayer>)>();
    let Ok(xfrm) = start_q.single(world) else {
        log::error!("no PlayerStart or OurPlayer");
        return;
    };
    drop(start_q);
    let xfrm = xfrm.clone();

    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);

    // Put and orient the new Player where the PlayerStart is.
    commands.entity(ent).insert((
        PlayerLook { rotation: xfrm.rotation, .. default() },
        xfrm
    ));

    queue.apply(world);
}

pub(crate) fn spawn_level(
    mut commands: Commands,
    map_assets: Res<MapAssets>,
    world: Single<Entity, With<WorldMarker>>,
) {
    commands.insert_resource(WorldSetup {
        waiting_skybox: true,
        waiting_reflections: false,
    });

    let level = commands
        .spawn((SceneRoot(map_assets.level_test.clone()),))
        .observe(|_ready: On<SceneInstanceReady>, mut commands: Commands| {
            commands.insert_resource(Spawning(true));
            commands.insert_resource(Shake(Vec3::ZERO));
        })
        .id();

    commands.entity(*world).add_child(level);
}

fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
    listener_q: Query<&Transform, With<SpatialListener3D>>,
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
        *timer = Timer::from_seconds(0.0125, TimerMode::Repeating);
        // *timer = Timer::from_seconds(1.0, TimerMode::Repeating);
    }
    if !timer.tick(time.delta()).just_finished() {
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
    keys: Res<ButtonInput<KeyCode>>,

    base: Res<Base>,
    time: Res<Time<Physics>>,
    shake: Option<Res<Shake>>,
    spawning: Res<Spawning>,
    overlay: Res<State<OverlayState>>,

    mut commands: Commands,
    mut forces: Query<Forces>,
) {
    if overlay.is_menu() {
        // Ignore here
        return;
    }

    if keys.just_released(KeyCode::Enter) {
        commands.insert_resource(Spawning(!spawning.0))
    }
    if keys.just_released(KeyCode::Space) {
        if let Ok(mut force) = forces.get_mut(base.0) {
            // dbg!(base.0);
            let mut x = 1000.0 * (time.elapsed_secs() % 5.0);
            if time.elapsed_secs() % 1.0 < 0.5 {
                x = -x;
            }
            force.apply_local_linear_impulse(Vec3::new(x, 0.0, -x));
        }
    }

    let mut force = if let Some(shake) = shake {
        shake.0
    } else {
        Vec3::ZERO
    };
    force.x = if keys.pressed(KeyCode::ArrowLeft) {
        -1.0
    } else {
        0.0
    } + if keys.pressed(KeyCode::ArrowRight) {
        1.0
    } else {
        0.0
    };
    force.z = if keys.pressed(KeyCode::ArrowUp) {
        -1.0
    } else {
        0.0
    } + if keys.pressed(KeyCode::ArrowDown) {
        1.0
    } else {
        0.0
    };
    if force.length() > 0.0 {
        commands.insert_resource(Shake(force * time.delta_secs()));
    }
}

fn shake_base(
    base: Res<Base>,
    shake: Option<Res<Shake>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    fx: Res<FxAssets>,
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
                commands.remove_resource::<Shake>();

                if !*shaking {
                    *shaking = true;
                    commands.spawn((UiSfx, SamplePlayer::new(fx.shake.clone())));
                }
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

// fn move_camera_around(
//     time: Res<Time<Physics>>,
//     camera_q: Single<(Entity, &Transform), With<Camera3d>>,
//     mut forces_q: Query<Forces>,
// ) {
//     if time.is_paused() {
//         return;
//     }

//     // Move some.
//     let mut rng = rand::rng();
//     let diff = Vec3::new(
//         rng.random_range(-1.0..=1.0),
//         rng.random_range(-1.5..=1.0),
//         rng.random_range(-2.0..=1.0),
//     );
//     let orig_pos = camera_q.1.translation;
//     let diff_rot = camera_q.1.rotation * diff;

//     let new_pos = orig_pos + diff_rot * 50.0 * time.delta_secs();

//     // camera_q.translation += diff_rot * 10.0 * time.delta_secs();
//     if let Ok(mut forces) = forces_q.get_mut(camera_q.0) {
//         forces.apply_linear_impulse((new_pos - orig_pos) * time.delta_secs());
//     }
// }

// fn aim_camera_around(
//     time: Res<Time<Physics>>,
//     mut camera_q: Single<(Entity, &mut Transform), With<Camera3d> /* With<OurCamera>, */>,
// ) {
//     if time.is_paused() {
//         return;
//     }

//     let target = camera_q.1.looking_at(Vec3::ZERO, Vec3::Y);
//     camera_q.1.rotation = camera_q.1.rotation.lerp(target.rotation, 0.5);
// }
