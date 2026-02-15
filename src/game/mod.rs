
mod logic;
mod level_0;
mod level_1;
mod level_2;
mod level_3;

use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::core_pipeline::Skybox;
use leafwing_input_manager::prelude::ActionState;
pub use logic::*;

use std::time::Duration;

use crate::assets::{ModelAssets, SkyboxAssets};
use crate::common::*;
use crate::player_spawning::spawn_player;

use bevy::asset::uuid::Uuid;
use bevy::ecs::world::CommandQueue;
use bevy_seedling::prelude::*;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::{
    scene::SceneInstanceReady,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LogicPlugin)

            .insert_resource(LevelList(default()))
            .insert_resource(LevelIndex(0))

            .add_plugins(level_0::LevelPlugin)
            .add_plugins(level_1::LevelPlugin)
            .add_plugins(level_2::LevelPlugin)
            .add_plugins(level_3::LevelPlugin)

            .insert_resource(BaseEntity(Entity::PLACEHOLDER, Transform::IDENTITY))

            .add_observer(observe_spawn_mesh)

            .add_systems(
                OnExit(ProgramState::New),
                ensure_levels
            )
            .add_systems(
                OnEnter(GameplayState::Setup),
                (
                    level_spawn_started,
                    spawn_level,
                ).chain()
            )
            .add_systems(
                OnExit(GameplayState::Setup),
                (
                    level_spawn_finished,
                ).chain()
            )
            .add_systems(
                Update,
                (
                    init_player_settings,
                    spawn_player_on_start,
                )
                .chain()
                .run_if(added_player_start)
                .run_if(in_state(GameplayState::Playing))
            )
            .add_systems(
                OnTransition{ exited: GameplayState::Playing, entered: GameplayState::Setup },
                despawn_level,
            )

            .add_systems(OnEnter(LevelState::LevelLoaded),
                (
                    start_skybox_setup,
                ).chain()
                    .run_if(in_state(ProgramState::InGame))
            )

            .add_systems(
                OnEnter(LevelState::Won),
                won_level,
            )
            .add_systems(
                OnEnter(LevelState::Lost),
                lost_level
            )

            .add_systems(
                OnEnter(LevelState::Advance),
                advance_level
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
                    check_won_level.run_if(in_state(LevelState::Won)),
                    check_lost_level.run_if(in_state(LevelState::Lost)),
                )
                    .run_if(not(is_in_menu))
                    .run_if(in_state(ProgramState::InGame))
                ,
            )
        ;
    }
}

/// This defines the list of levels.
#[derive(Resource, Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct LevelInfo {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) scene: Handle<Scene>,
}

/// This defines the list of levels.
#[derive(Resource, Reflect, Default, Debug)]
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
#[derive(Resource, Reflect, Debug, Deref)]
#[reflect(Resource)]
#[type_path = "game"]
pub(crate) struct CurrentLevel(pub(crate) LevelInfo);

/// The level index into [LevelList].
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct LevelIndex(pub usize);

/// The current score.
#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct CurrentScore {
    pub score: i32,
}

const END_LEVEL_DELAY_SECS: u64 = 3;

/// Countdown to next or same level.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct AutoEndLevelTimer(pub(crate) Timer);

// Map markers (in .glb)

/// Marker for the top level entity of a level (for searching metadata).
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct LevelRoot;

/// Place on LevelRoot for the camera mode of the level.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct PlayerCameraMode(pub PlayerMode);

/// Place on LevelRoot for the camera effects to apply.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub enum CameraEffects {
    Normal,
    Mode1,
    Mode2,
}

/// Place on LevelRoot for the skybox to apply.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub enum SkyboxSelection {
    Space,
    Farm,
}

/// Place on LevelRoot for the music track to use.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub enum MusicTrack {
    Song0,
    Song2,
}


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

/// Marker (in .glb) for the base box.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct BaseMarker;

/// Marker (in .glb) for a generator switch collider.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct GeneratorSwitchCollider;

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

// World state

/// Our "base" object and its initial transform.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub(crate) struct BaseEntity(pub Entity, pub Transform);

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

// Player state

/// Marker for an object (e.g. net) in the hand.
#[derive(Component)]
pub(crate) struct InHand;

/// [Player] marker for currently catching.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub(crate) struct Catching;


/////

fn observe_spawn_mesh(
    ready: On<SceneInstanceReady>,
    children: Query<&Children>,
    meshes: Query<&Mesh3d>,
    mut commands: Commands,
) {
    for entity in children.iter_descendants(ready.entity) {
        if meshes.contains(entity) {
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
        }
    }
}

pub(crate) fn ensure_levels(mut level_list: ResMut<LevelList>) {
    level_list.0.sort_by(|a, b| a.id.cmp(&b.id));
}

pub(crate) fn level_spawn_started(mut commands: Commands, mut pause: ResMut<PauseState>) {
    commands.set_state(LevelState::Initializing);
    commands.set_state(OverlayState::Loading);

    // Prevent moving/interacting while loading UI is up.
    pause.set_menu_paused(true);
}

pub(crate) fn level_spawn_finished(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
    sensable_q: Query<Entity, Or<(
        With<DeathboxCollider>,
        With<GeneratorSwitchCollider>,
        With<ConsumerCollider>,
    )>>,
    base_q: Query<(Entity, &Transform), With<BaseMarker>>,
    net_q: Query<Entity, With<NetCollider>>,
) {
    for ent in sensable_q.iter() {
        commands.entity(ent).insert((
            Sensor,
            CollisionEventsEnabled,
            CollidingEntities::default(),
        ));
    }
    if let Some((ent, xfrm)) = base_q.iter().next() {
        commands.insert_resource(BaseEntity(ent, xfrm.clone()));
    } else {
        commands.remove_resource::<BaseEntity>();
    }
    if let Some(ent) = net_q.iter().next() {
        commands.entity(ent).insert(ColliderDisabled);
    }

    commands.set_state(OverlayState::Hidden);
    commands.set_state(LevelState::LevelLoaded);

    // Go for it, user (unless they did set_user_paused)
    pause.set_menu_paused(false);
}

fn added_player_start(q: Query<&Transform, Added<PlayerStart>>) -> bool {
    let flag = q.iter().next().is_some();
    flag
}

pub(crate) fn spawn_player_on_start(world: &mut World) {
    // Make the player collision model and Player
    let player_ent = spawn_player(world, Uuid::default());

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

    queue.apply(world);
}

pub(crate) fn spawn_level(
    mut commands: Commands,
    level_list: Res<LevelList>,
    level_index: Res<LevelIndex>,
    world: Res<WorldMarkerEntity>,
    mut score_q: Query<&mut Text, (With<ScoreArea>, Without<GameStatusArea>)>,
    mut status_q: Query<&mut Text, (With<GameStatusArea>, Without<ScoreArea>)>,
) {
    let index = level_index.0;
    if index >= level_list.0.len() {
        log::error!("no items in LevelList");
        commands.remove_resource::<CurrentLevel>();
        commands.set_state(ProgramState::Error);
        return;
    }

    let level = &level_list.0[level_index.0];
    commands.insert_resource(CurrentLevel(level.clone()));

    log::info!("Entering level {}", level.label);

    commands
        .spawn((
            DespawnOnExit(GameplayState::Playing),
            SceneRoot(level.scene.clone()),
            ChildOf(world.0),
        ))
        .observe(|_event: On<SceneInstanceReady>, mut commands: Commands,| {
            commands.set_state(GameplayState::Playing);
        })
    ;
    commands.insert_resource(CurrentScore::default());

    score_q.single_mut().unwrap().clear();
    status_q.single_mut().unwrap().clear();
}

pub(crate) fn despawn_level(
    mut commands: Commands,
    sounds_q: Query<Entity, With<SamplePlayer>>,
    spawned_q: Query<Entity, With<Spawned>>,
    player_q: Query<Entity, With<Player>>,
) {
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
//
fn init_player_settings(
    move_q: Query<&PlayerCameraMode, With<LevelRoot>>,
    mut commands: Commands,
    mut settings: ResMut<PlayerInputSettings>,
) {
    if let Ok(mode) = move_q.single() {
        match mode.0 {
            PlayerMode::Fps => *settings = PlayerInputSettings::for_fps(),
            PlayerMode::Space => *settings = PlayerInputSettings::for_space(),
        }
        commands.insert_resource(mode.0);
    } else {
        log::warn!("no PlayerCameraMode in LevelRoot");
    }
}

fn start_skybox_setup(
    mut commands: Commands,
    world_camera_q: Query<Entity, (With<Camera3d>, With<WorldCamera>)>,
    skybox_q: Query<&SkyboxSelection, With<LevelRoot>>,
    skyboxes: Res<SkyboxAssets>,
) {
    if let Ok(selection) = skybox_q.single() {
        let cam = world_camera_q.single().unwrap();

        let (brightness, skybox) = match selection {
            SkyboxSelection::Space => (100.0, skyboxes.star_map.clone()),
            SkyboxSelection::Farm => (light_consts::lux::CLEAR_SUNRISE, skyboxes.pure_sky.clone()),
        };
        let with_reflection_probe = Some((cam, 100.0));
        // let with_reflection_probe = None;
        commands.entity(cam).insert(SkyboxModel {
            skybox: Skybox {
                image: skybox,
                brightness,
                ..default()
            },
            xfrm: SkyboxTransform::From1_0_2f_3f_4_5,
            with_reflection_probe,
            enabled: true, //state.show_skybox,
        });


        commands.insert_resource(SkyboxSetup {
            waiting_skybox: true,
            waiting_reflections: false,
        });
        commands.set_state(LevelState::LoadingSkybox);
    } else {
        commands.set_state(LevelState::Playing);
        log::warn!("2");
    }
}

pub(crate) fn advance_level(
    mut commands: Commands,
    spawned_q: Query<Entity, With<Spawned>>,
) {
    for ent in spawned_q.iter() {
        commands.entity(ent).try_despawn();
    }
    commands.set_state(OverlayState::Loading);
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
        if *level_state == LevelState::LoadingSkybox {
            // This is allowable, but report once just in case.
            log::warn!("missing or too many LevelRoot + ScoreGoal");
        };
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
    text.0 = "Passed!".to_string();
    color.0 = Color::Srgba(tailwind::LIME_300);

    commands.insert_resource(AutoEndLevelTimer(Timer::new(Duration::from_secs(END_LEVEL_DELAY_SECS), TimerMode::Once)));
}

fn lost_level(
    mut commands: Commands,
    mut score_q: Single<(&mut Text, &mut TextColor), With<GameStatusArea>>,
) {
    let (ref mut text, ref mut color) = *score_q;
    text.0 = "Failed...\nTry again!".to_string();
    color.0 = Color::Srgba(tailwind::RED_700);

    commands.insert_resource(AutoEndLevelTimer(Timer::new(Duration::from_secs(END_LEVEL_DELAY_SECS), TimerMode::Once)));
}

fn check_won_level(
    mut commands: Commands,
    mut end_timer: ResMut<AutoEndLevelTimer>,
    time: Res<Time<Physics>>,
    level_index: ResMut<LevelIndex>,
    level_list: Res<LevelList>,
) {
    if !end_timer.0.tick(time.delta()).is_finished() {
        return;
    }

    let next_index = level_index.0 + 1;
    if next_index >= level_list.0.len() {
        commands.set_state(ProgramState::Completed);
        commands.set_state(LevelState::Initializing);
        commands.set_state(GameplayState::Done);
        commands.set_state(OverlayState::GameOverScreen);
    } else {
        commands.insert_resource(LevelIndex(next_index));
        commands.set_state(LevelState::Advance);
    }
}

fn check_lost_level(
    mut commands: Commands,
    mut end_timer: ResMut<AutoEndLevelTimer>,
    time: Res<Time<Physics>>,
) {
    if !end_timer.0.tick(time.delta()).is_finished() {
        return;
    }

    // Restarts level.
    commands.set_state(LevelState::Advance);
}

fn spawn_net(
    mut commands: Commands,
    models: Res<ModelAssets>,
    cam: Entity,
) {
    commands.spawn((
        Name::new("Net"),
        RenderLayers::layer(RENDER_LAYER_VIEW),
        SceneRoot(models.net.clone()),
        Transform::from_xyz(0.0, 0.0, -2.0).with_scale(Vec3::splat(2.0)),
        Visibility::Hidden,
        InHand,
        ChildOf(cam),
    ))
    ;
}
