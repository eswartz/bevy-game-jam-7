use std::time::Duration;

use crate::assets::*;
use crate::game::*;
use crate::common::*;

use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::Skybox;
use leafwing_input_manager::prelude::ActionState;
use rand::RngExt;

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
                OnEnter(GameplayState::AssetsLoading),
                register_level
            )
            .add_systems(
                OnEnter(LevelState::LevelLoaded),
                on_level_loaded
                    .run_if(is_in_level(ID))
            )
            // .add_systems(
            //     OnEnter(LevelState::),
            //     on_level_init
            //         .run_if(is_in_level(ID))
            //         .run_if(in_state(ProgramState::InGame))
            //         .run_if(in_state(LevelState::Initializing))
            // )
            .add_systems(
                FixedUpdate,
                check_actions
                    .run_if(is_in_level(ID))
                    .run_if(not(is_paused))
                    .run_if(not(is_in_menu))
                    .run_if(not(egui_wants_any_keyboard_input))
                    .run_if(is_level_active)
                    .run_if(in_state(ProgramState::InGame))
            )
        ;
    }
}

fn register_level(mut list: ResMut<LevelList>, maps: Res<MapAssets>) {
    list.0.push(LevelInfo {
        id: ID.to_string(),
        label: NAME.to_string(),
        scene: maps.level_0.clone()
    });
}

fn on_level_loaded(
    mut commands: Commands,
    world_camera_q: Query<Entity, (With<Camera3d>, With<WorldCamera>)>,
    viewer_camera_q: Query<Entity, (With<Camera3d>, With<ViewerCamera>)>,
    models: Res<ModelAssets>,
    skyboxes: Res<SkyboxAssets>,
) {
    let cam = viewer_camera_q.single().unwrap();
    commands.spawn((
        Name::new("Net"),
        RenderLayers::layer(RENDER_LAYER_VIEW),
        SceneRoot(models.net.clone()),
        Transform::from_xyz(0.0, 0.0, -2.0).with_scale(Vec3::splat(2.0)),
        Visibility::Visible,
        ChildOf(cam),
    ));

    commands.insert_resource(Spawning(false));
    commands.insert_resource(SpawnDelay(Duration::from_secs(1)));
    commands.insert_resource(SpawnTimer(Timer::new(Duration::from_secs(1), TimerMode::Repeating)));
    commands.insert_resource(ShakeTime(Duration::ZERO));

    // commands.set_state(LevelState::Playing);

    let cam = world_camera_q.single().unwrap();

    let (brightness, skybox) = (100.0, skyboxes.star_map.clone());
    // let (brightness, skybox, transform) = (500.0, skyboxes.driving_school.clone());
    // let (brightness, skybox, transform) = (lux::CLEAR_SUNRISE, skyboxes.kloppenheim_sky_map.clone());
    // let (brightness, skybox, transform) = (lux::CLEAR_SUNRISE, skyboxes.pure_sky.clone());
    // let add_reflection_probe = Some(commands.spawn_empty().id());
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

    commands.set_state(LevelState::LoadingSkybox);
}

fn check_actions(
    actions: Res<ActionState<UserAction>>,
    time: Res<Time<Physics>>,
    shake_q: Query<Entity, With<ShakingSound>>,
    mut shake_time: ResMut<ShakeTime>,
    mut commands: Commands,
) {
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
