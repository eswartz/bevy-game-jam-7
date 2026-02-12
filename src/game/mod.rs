
mod logic;
mod level_0;
mod level_3;

use bevy::color::palettes::tailwind;
use leafwing_input_manager::prelude::ActionState;
pub use logic::*;

use std::time::Duration;

use crate::{assets::*};
use crate::player_spawning::spawn_player;
use crate::common::*;

use bevy::asset::uuid::Uuid;
use bevy::ecs::world::CommandQueue;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;
use bevy_asset_loader::loading_state::config::{ConfigureLoadingState as _, LoadingStateConfig};
use bevy_seedling::prelude::*;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::{
    gltf::GltfMeshName,
    scene::SceneInstanceReady,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LogicPlugin)

            .insert_resource(LevelList(default()))

            .add_plugins(level_0::Level0Plugin)
            .add_plugins(level_3::Level3Plugin)

            .insert_resource(Base(Entity::PLACEHOLDER, Transform::IDENTITY))

            .add_observer(observe_spawn_mesh)

            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<MapAssets>()
                    .load_collection::<ModelAssets>()
            )

            .add_systems(
                OnExit(GameplayState::AssetsLoaded),
                ensure_first_level
            )

            .add_systems(
                OnEnter(GameplayState::Setup),
                (
                    level_spawn_started,
                    spawn_level,
                ).chain()
                // .run_if(in_state(ProgramState::InGame)) // redundant
            )
            .add_systems(
                OnExit(GameplayState::Setup),
                (
                    level_spawn_finished,
                ).chain()
                // .run_if(in_state(ProgramState::InGame)) // redundant
            )
            .add_systems(
                Update,
                spawn_player_on_start
                    .run_if(added_player_start)
                    .run_if(in_state(GameplayState::Playing))
            )
            .add_systems(
                // OnExit(GameplayState::Playing),
                OnTransition{ exited: GameplayState::Playing, entered: GameplayState::Setup },
                despawn_level,
            )

            // .add_systems(
            //     OnEnter(LevelState::Loaded),
            //     (
            //         spawn_player_on_start,
            //     ).chain()
            // )
            .add_systems(
                OnEnter(LevelState::Won),
                won_level,
            )
            .add_systems(
                OnEnter(LevelState::Lost),
                lost_level,
            )

            .add_systems(
                OnEnter(LevelState::Advance),
                (
                    // despawn_level,
                    advance_level,
                ).chain()
            )

            .add_systems(
                Update,
                (
                    update_current_score,
                )
                    .run_if(not(is_in_menu))
                    .run_if(in_state(LevelState::Playing))
                    .run_if(in_state(ProgramState::InGame))
                ,
            )
            .add_systems(
                Update,
                (
                    check_actions,
                )
                    .run_if(not(is_in_menu))
                    .run_if(is_level_active)
                    .run_if(in_state(ProgramState::InGame))
                ,
            )
            .add_systems(
                Update,
                (
                    check_end_level,
                )
                    .run_if(not(is_in_menu))
                    .run_if(in_state(LevelState::Won).or(in_state(LevelState::Lost)))
                    .run_if(in_state(ProgramState::InGame))
                ,
            )
        ;
    }
}

/// This defines the list of levels.
#[derive(Resource, Reflect, Default, Clone, PartialEq)]
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
        level.is_some_and(|l| {
            l.0.id == id
        })
    }
}

/// The current level.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct CurrentLevel(pub LevelInfo);

/// The current score.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct CurrentScore {
    pub score: i32,
}

const END_LEVEL_DELAY_SECS: u64 = 1;

/// Countdown to next or same level.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct AutoEndLevelTimer(pub(crate) Timer);

/// Marker for the top level entity of a level (for searching metadata).
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct LevelRoot;

/// Marker for things that give a score.
/// The [LevelRoot] is expected to have this.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct ScoreGoal {
    /// Score to win.
    pub(crate) goal: u32,
    /// Score to lose.
    pub(crate) lose: i32,
}

/// Marker for things that give a score.
/// The [LevelRoot] is expected to have this.
#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct Scoreable {
    /// How many to add if the entity is gained.
    pub(crate) gain: u8,
    /// How many to add if the entity is lost.
    pub(crate) lose: u8,
}

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


/// Marker (in .glb) for the death box.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct DeathboxCollider;

/// Marker (in .glb) for the collider.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct NetCollider;

/// Marker (in .glb) for a consumer.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct ConsumerCollider;

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

#[derive(Resource, Reflect, Default, Deref, DerefMut)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct SpawnTimer(pub(crate) Timer);

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

pub(crate) fn level_spawn_started(mut commands: Commands, mut pause: ResMut<PauseState>) {
    log::warn!("level_spawn_started");
    commands.set_state(LevelState::Initializing);
    commands.set_state(OverlayState::Loading);
    pause.set_menu_paused(true);
}

pub(crate) fn level_spawn_finished(mut commands: Commands, mut pause: ResMut<PauseState>) {
    commands.set_state(OverlayState::Hidden);
    commands.set_state(LevelState::Loaded);
    pause.set_menu_paused(false);
    log::warn!("level_spawn_finished");
}

fn added_player_start(q: Query<&Transform, Added<PlayerStart>>) -> bool {
    let flag = q.iter().next().is_some();

    if flag {
        log::warn!("Saw PlayerStart");
    }
    flag
}

pub(crate) fn spawn_player_on_start(world: &mut World) {
    // Make the player collision model and Player
    let player_ent = spawn_player(world, Uuid::default());

    // let mut camera_q = world.query_filtered::<Entity, (With<Camera3d>, With<ViewerCamera>)>();
    // let Ok(cam_ent) = camera_q.single(world) else {
    //     log::error!("no single PlayerStart or OurPlayer");
    //     return;
    // };
    // drop(camera_q);

    // Move to start position/orientation.
    let mut start_q = world.query_filtered::<&Transform, With<PlayerStart>>();
    let Some(xfrm) = start_q.iter(world).next() else {
        log::error!("no PlayerStart");
        return;
    };
    drop(start_q);
    let xfrm = xfrm.clone();

    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);

    // Put and orient the new Player where the PlayerStart is.
    commands.entity(player_ent).insert((
        PlayerLook { rotation: xfrm.rotation, .. default() },
        xfrm
    ));

    // // Move view camera inside player.
    // commands.entity(cam_ent).insert(ChildOf(player_ent));


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
    world: Query<Entity, With<WorldMarker>>,
    mut score_q: Query<&mut Text, (With<ScoreArea>, Without<GameStatusArea>)>,
    mut status_q: Query<&mut Text, (With<GameStatusArea>, Without<ScoreArea>)>,
) {
    log::info!("Entering level {}", level.0.label);

    let level = commands
        .spawn((
            DespawnOnExit(GameplayState::Playing),
            SceneRoot(level.0.scene.clone()),
        ))
        .observe(|_event: On<SceneInstanceReady>, mut commands: Commands,| {
            commands.set_state(GameplayState::Playing);
        })
        .id();

    commands.insert_resource(CurrentScore::default());

    commands.entity(world.single().unwrap()).add_child(level);
    score_q.single_mut().unwrap().clear();
    status_q.single_mut().unwrap().clear();
}

pub(crate) fn despawn_level(
    mut commands: Commands,
    sounds_q: Query<Entity, With<SamplePlayer>>,
    spawned_q: Query<Entity, With<Spawned>>,
    player_q: Query<Entity, With<Player>>,
) {
    log::warn!("despawn level");
    for ent in sounds_q.iter() {
        commands.entity(ent).try_despawn();
    }
    for ent in spawned_q.iter() {
        commands.entity(ent).try_despawn();
    }
    for ent in player_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub(crate) fn advance_level(
    mut commands: Commands,
    // gameplay_state: Res<State<GameplayState>>,
) {
    log::warn!("next level");
    // commands.set_state(LevelState::Playing);
    // commands.set_state(GameplayState::Setup);

    // commands.set_state(LevelState::Initializing);
    // if *gameplay_state.get() == GameplayState::AssetsLoaded {
    // commands.set_state(GameplayState::Setup);
    // }

    commands.set_state(OverlayState::Loading);
    // commands.set_state(ProgramState::InGame);
    // commands.set_state(GameplayState::AssetsLoading);

    commands.set_state(GameplayState::Setup);
}

fn update_current_score(
    mut commands: Commands,
    level_state: Res<State<LevelState>>,
    score: Option<Res<CurrentScore>>,
    mut score_q: Single<(&mut Text, &mut TextColor), With<ScoreArea>>,
    goal_q: Query<&ScoreGoal, With<LevelRoot>>,
) {
    let Ok(goal) = goal_q.single() else {
        log::error!("missing or too many LevelRoot + ScoreGoal");
        return;
    };

    let (ref mut text, ref mut color) = *score_q;
    if let Some(score) = score {
        if *level_state == LevelState::Playing {
            let won = score.score >= goal.goal as _;
            let lost = score.score <= goal.lose;

            text.0 = format!("Score: {} / {}", score.score, goal.goal);
            color.0 = Color::Srgba(if won {
                tailwind::LIME_300
            } else if lost {
                tailwind::RED_700
            } else {
                tailwind::GRAY_100
            });

            if won {
                commands.set_state(LevelState::Won);
            } else if lost {
                commands.set_state(LevelState::Lost);
            }
        }
    } else {
        text.0.clear();
    }
}

fn won_level(
    mut commands: Commands,
    mut score_q: Single<(&mut Text, &mut TextColor), With<GameStatusArea>>,
) {
    let (ref mut text, ref mut color) = *score_q;
    text.0 = "You Win!".to_string();
    color.0 = Color::Srgba(tailwind::LIME_300);

    commands.insert_resource(AutoEndLevelTimer(Timer::new(Duration::from_secs(END_LEVEL_DELAY_SECS), TimerMode::Once)));
}

fn lost_level(
    mut commands: Commands,
    mut score_q: Single<(&mut Text, &mut TextColor), With<GameStatusArea>>,
) {
    let (ref mut text, ref mut color) = *score_q;
    text.0 = "You Lost...".to_string();
    color.0 = Color::Srgba(tailwind::RED_700);

    commands.insert_resource(AutoEndLevelTimer(Timer::new(Duration::from_secs(END_LEVEL_DELAY_SECS), TimerMode::Once)));
}

fn check_end_level(
    mut commands: Commands,
    mut end_timer: ResMut<AutoEndLevelTimer>,
    time: Res<Time<Physics>>,
    level_info: Res<CurrentLevel>,
    level_list: Res<LevelList>,
) {
    if !end_timer.0.tick(time.delta()).is_finished() {
        return;
    }

    if let Some(current_index) = level_list.0.iter().position(|x| *x == level_info.0) {
        let next_index = (current_index + 1) % level_list.0.len();
        commands.insert_resource(CurrentLevel(level_list.0[next_index].clone()));
    } else {
        log::error!("current level not found!");
    };

    commands.set_state(LevelState::Advance);

    // commands.set_state(OverlayState::Loading);
    // commands.set_state(GameplayState::Setup);
    // commands.set_state(GameplayState::AssetsLoading);
}

fn check_actions(
    actions: Res<ActionState<UserAction>>,
    mut commands: Commands,
) {
    if actions.just_released(&UserAction::ForceLose) {
        commands.set_state(LevelState::Lost);
    }
    if actions.just_released(&UserAction::ForceWin) {
        commands.set_state(LevelState::Won);
    }
}
