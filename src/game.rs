use std::time::Duration;

use crate::{assets::*, level0};
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

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(LevelList(default()))
            .insert_resource(Spawning(false))
            .insert_resource(Base(Entity::PLACEHOLDER, Transform::IDENTITY))
            .insert_resource(ShakeRequest(Vec3::ZERO))
            .insert_resource(ShakeTime(Duration::ZERO))
            .add_observer(observe_spawn_mesh)

            .add_plugins(level0::Level0Plugin)

            .add_systems(
                OnExit(GameplayState::AssetsLoaded),
                ensure_first_level
            )

            .add_systems(
                OnEnter(GameplayState::Setup),
                spawn_level
                // .run_if(in_state(ProgramState::InGame)) // redundant
            )
            .add_systems(
                OnEnter(GameplayState::Playing),
                spawn_player_on_start,
                // .run_if(in_state(ProgramState::InGame)) // redundant
            )
            .add_systems(
                OnExit(GameplayState::Playing),
                clear_level,
            )
        ;
    }
}

/// This defines the list of levels.
#[derive(Resource, Reflect, Default, Clone)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct LevelInfo {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) scene: Handle<Scene>,
}

/// This defines the list of levels.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct LevelList(pub(crate) Vec<LevelInfo>);

pub fn is_in_level(id: &str) -> impl Fn(Option<Res<CurrentLevel>>) -> bool {
    move |level: Option<Res<CurrentLevel>>| -> bool {
        level.is_some_and(|l| l.0.id == id)
    }
}

/// The current level.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct CurrentLevel(pub LevelInfo);

/// Marker for a thing that generates things.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct Generator;

/// Marker for things we spawned.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct Spawned;

/// Our "base" object and its initial transform.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub(crate) struct Base(pub Entity, pub Transform);

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

pub(crate) fn ensure_first_level(
    mut commands: Commands,
    list: Res<LevelList>,
) {
    let Some(first) = list.0.first() else {
        log::error!("no items in LevelList");
        commands.remove_resource::<CurrentLevel>();
        return;
    };

    commands.insert_resource(CurrentLevel(first.clone()));
}

pub(crate) fn spawn_level(
    mut commands: Commands,
    level: Res<CurrentLevel>,
    world: Single<Entity, With<WorldMarker>>,
) {
    log::info!("Entering level {}", level.0.label);

    commands.insert_resource(WorldSetup {
        waiting_skybox: true,
        waiting_reflections: false,
    });

    let level = commands
        .spawn((SceneRoot(level.0.scene.clone()),))
        .observe(|_ready: On<SceneInstanceReady>, mut commands: Commands| {
            commands.insert_resource(Spawning(false));
            commands.insert_resource(SpawnDelay(Duration::from_secs(1)));
        })
        .id();

    commands.entity(*world).add_child(level);
}

pub(crate) fn clear_level(
    mut commands: Commands,
    sounds_q: Query<Entity, With<SamplePlayer>>,
    spawned_q: Query<Entity, With<Spawned>>,
) {
    for ent in sounds_q.iter() {
        commands.entity(ent).try_despawn();
    }
    for ent in spawned_q.iter() {
        commands.entity(ent).try_despawn();
    }
}
