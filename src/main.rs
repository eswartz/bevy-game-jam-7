mod common;
mod menus;
mod assets;
mod audio;
mod player_spawning;
mod actions;
mod camera;
mod game;

use crate::assets::*;
use crate::audio::AudioPlugin;
use crate::camera::ensure_3d_camera;
use crate::game::CameraEffects;
use crate::game::GamePlugin;
use crate::game::LevelRoot;
use crate::menus::MenuPlugin;
use bevy::color::palettes::tailwind;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::render::view::ColorGrading;
use bevy::render::view::ColorGradingGlobal;
use bevy::render::view::ColorGradingSection;
use bevy_seedling::prelude::*;
use common::*;

use std::time::Duration;

use avian3d::prelude::*;
use bevy::camera::Exposure;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::ecs::message::MessageUpdateSystems;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapter;
use bevy::render::renderer::RenderDevice;
use bevy::render::view::Hdr;
use bevy::{
    asset::AssetMetaCheck,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    image::{ImageAddressMode, ImageSamplerDescriptor},
    winit::WinitSettings,
};
use bevy_asset_loader::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin};
use bevy_skein::SkeinPlugin;

/// This is registered to initiate a shutdown.
/// The process may take a few frames (e.g. waiting on network).
#[derive(Debug, Resource)]
pub struct ExitRequest;

fn main() -> AppExit {
    let res = find_runtime_base_directory_by_folder("assets");
    let base_dir = match res {
        Err(e) => {
            log::error!("startup failure: {e}");
            return AppExit::from_code(3);
        }
        Ok(base_dir) => base_dir,
    };

    let mut app = App::new();
    app
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f32(
                1.0 / 120.0,
            )),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f32(
                1.0 / 24.0,
            )),
        })
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics in web builds on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    watch_for_changes_override: Some(true),
                    file_path: base_dir.join("assets").display().to_string(),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        address_mode_w: ImageAddressMode::Repeat,
                        ..ImageSamplerDescriptor::linear()
                    },
                }),
            SkeinPlugin::default(),
            PhysicsPlugins::default(),
        ))
        .add_plugins(avian3d::debug_render::PhysicsDebugPlugin::default())

        .insert_gizmo_config(
            PhysicsGizmos::default(),
            GizmoConfig {
                // enabled: true,
                enabled: false,
                depth_bias: -0.1,
                ..default()
            },
        )
        ////////

        .add_plugins(EguiPlugin::default())
        .insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })

        ////////
        .insert_state(ProgramState::default())
        .insert_state(GameplayState::default())
        .insert_state(OverlayState::default())

        //////
        // Custom exit handling.
        .add_systems(
            First,
            (
                check_app_exit.in_set(MessageUpdateSystems),
                check_windows_closed.in_set(MessageUpdateSystems),
            )
                .chain(),
        )
        //////

        .add_loading_state(
            LoadingState::new(ProgramState::Initializing)
                .continue_to_state(ProgramState::New)
                .on_failure_continue_to_state(ProgramState::Error)
            ,
        )

        .add_plugins(ActionPlugin)
        .insert_resource(actions::default_input_map())

        .add_plugins(MenuPlugin)
        .add_plugins(LifecyclePlugin)
        // .add_plugins(LevelStatePlugin)
        .add_plugins(GuiPlugin)
        .add_plugins(WorldUiPlugin)
        .add_plugins(WorldStatePlugin)
        .add_plugins(AudioPlugin)
        .add_plugins(CrosshairPlugin)
        .add_plugins(EffectsPlugin)

        .add_plugins(PlayerCameraPlugin)
        .add_plugins(PlayerInputPlugin)
        .add_plugins(PlayerClientPlugin)
        .add_plugins(PlayerMovementPlugin)
        .add_plugins(PlayerControllerPlugin)

        .insert_resource(OurUser(default()))

        .insert_resource(PlayerMode::Space)
        .insert_resource(PlayerInputSettings::for_space())

        .add_loading_state(
            LoadingState::new(ProgramState::Initializing)
                .continue_to_state(ProgramState::New)
                .on_failure_continue_to_state(ProgramState::Error)
                .load_collection::<SkyboxAssets>()
                .load_collection::<MapAssets>()
                .load_collection::<ModelAssets>()
        )

        .add_systems(OnEnter(ProgramState::Initializing), on_enter_initializing)
        .add_systems(
            OnEnter(ProgramState::New),
            (on_enter_loading, init_perf_ui.run_if(show_dev_tools)).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::LaunchMenu),
            (on_enter_launch_menu,).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::InGame),
            (on_exit_launch_menu.run_if(is_in_menu),
                on_enter_in_game).chain(),
        )
        .add_systems(
            OnEnter(GameplayState::Playing),
            (ensure_3d_camera, show_3d_camera),
        )
        .add_systems(
            OnExit(GameplayState::Playing),
            hide_3d_camera, //.in_set(SimulationSystems),
        )
        .insert_resource(VideoSettings::default())
        ////////
        .init_state::<ProgramState>()
        .init_state::<GameplayState>()
        .init_state::<LevelState>()
        .insert_resource(ProductName(PRODUCT_NAME.to_string()))
        .insert_resource(PauseState::new(false))

        .add_systems(OnEnter(OverlayState::GameOverScreen),
            on_game_over_screen)
        .add_systems(OnExit(OverlayState::GameOverScreen),
            on_game_over_screen_finished)

        .add_systems(OnEnter(ProgramState::Error),
            on_enter_error)
        .add_systems(OnEnter(OverlayState::ErrorScreen),
            on_error_screen)
        .add_systems(OnExit(OverlayState::ErrorScreen),
            on_error_screen_finished)

        /////
        .add_plugins(GamePlugin);

    if show_dev_tools() {
        app
            .add_plugins(DebugPlugin)
            .add_systems(
                First,
                (
                    bevy::dev_tools::states::log_transitions::<ProgramState>,
                    bevy::dev_tools::states::log_transitions::<GameplayState>,
                    bevy::dev_tools::states::log_transitions::<OverlayState>,
                    bevy::dev_tools::states::log_transitions::<LevelState>,
                ),
            )
            .add_plugins(FpsOverlayPlugin::default())
        ;
    }

    app.run()
}

fn check_app_exit(
    mut commands: Commands,
    exit: Option<Res<ExitRequest>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if exit.is_none() {
        return;
    }

    commands.remove_resource::<ExitRequest>();
    app_exit.write(AppExit::Success);
}

// It seems WindowClosed, WindowClosing, WindowDestroyed events don't make it for the primary window...?
fn check_windows_closed(windows: Query<&Window>, mut commands: Commands) {
    if windows.is_empty() {
        commands.insert_resource(ExitRequest);
    }
}

fn on_enter_initializing(mut commands: Commands, camera_q: Query<&Camera, With<Camera2d>>) {
    if camera_q.single().is_err() {
        commands.spawn((
            Camera2d,
            Camera {
                // Render before 3D.
                order: -1,
                clear_color: ClearColorConfig::Default,
                ..default()
            },
            RenderLayers::from_layers(&[RENDER_LAYER_UI]),
        ));
    }
}

fn on_enter_loading(mut commands: Commands) {
    commands.set_state(ProgramState::LaunchMenu);
}

fn on_enter_launch_menu(mut commands: Commands) {
    commands.set_state(OverlayState::MainMenu);
}

fn on_exit_launch_menu(mut commands: Commands) {
    commands.set_state(OverlayState::Hidden);
}

fn on_enter_in_game(mut time: ResMut<Time<Physics>>) {
    time.unpause();
}

fn init_perf_ui(mut commands: Commands) {
    commands.insert_resource(FpsOverlayConfig {
        text_config: TextFont::from_font_size(10.0),
        text_color: Color::WHITE.with_alpha(0.5),
        refresh_interval: Duration::from_secs_f32(1.0 / 10.0),
        ..default()
    });
}


#[derive(Component)]
pub(crate) struct GameOverScreen;

pub(crate) fn on_game_over_screen(
    mut commands: Commands,
    fonts: Option<Res<GuiAssets>>,
) {
    let ent_commands = commands.spawn((
        Name::new("GameOver"),
        GameOverScreen,
    ));
    setup_game_over_screen(ent_commands, fonts.as_deref());
}

pub(crate) fn on_game_over_screen_finished(
    mut commands: Commands,
    gui_q: Query<Entity, With<GameOverScreen>>,
) {
    for ent in gui_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub(crate) fn setup_game_over_screen(
    mut ent_commands: EntityCommands,
    fonts: Option<&GuiAssets>,
) -> Entity {
    ent_commands.insert((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            .. default()
        },
        BackgroundColor(tailwind::GREEN_800.with_alpha(0.5).into()),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    ))
    .with_children(|builder| {
        builder.spawn((
            Text::new(
                "You Won!",
            ),
            TextFont {
                font: fonts.map_or(default(), |fonts| fonts.std_ui.clone()),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
        builder.spawn((
            Text::new(
                "\u{a0}",
            ),
            TextFont {
                font: fonts.map_or(default(), |fonts| fonts.std_ui.clone()),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
        builder.spawn((
            Text::new(
                "Thanks for playing!",
            ),
            TextFont {
                font: fonts.map_or(default(), |fonts| fonts.std_ui.clone()),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
    })
    .id()
}

#[derive(Component)]
pub(crate) struct ErrorScreen;

pub(crate) fn on_enter_error(
    mut commands: Commands,
) {
    commands.set_state(OverlayState::ErrorScreen);
}

pub(crate) fn on_error_screen(
    mut commands: Commands,
    fonts: Option<Res<GuiAssets>>,
) {
    let ent_commands = commands.spawn((
        Name::new("Loading..."),
        ErrorScreen,
    ));
    setup_error_screen(ent_commands, fonts.as_deref());
}

pub(crate) fn on_error_screen_finished(
    mut commands: Commands,
    gui_q: Query<Entity, With<ErrorScreen>>,
) {
    for ent in gui_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub(crate) fn setup_error_screen(
    mut ent_commands: EntityCommands,
    fonts: Option<&GuiAssets>,
) -> Entity {
    ent_commands.insert((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            .. default()
        },
        BackgroundColor(tailwind::RED_800.with_alpha(0.75).into()),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    ))
    .with_children(|builder| {
        builder.spawn((
            Text::new(
                "There is an installation error.\nPlease gather stdout and stderr and report.",
            ),
            TextFont {
                font: fonts.map_or(default(), |fonts| fonts.std_ui.clone()),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
    })
    .id()
}
