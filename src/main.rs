mod common;
mod menus;
mod assets;
mod audio;
mod player_spawning;
mod game;
mod actions;

use crate::assets::*;
use crate::audio::AudioPlugin;
use crate::game::GamePlugin;
use crate::menus::MenuPlugin;
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

    let exit = App::new()
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
        .add_plugins(avian3d::debug_render::PhysicsDebugPlugin::default()) // show colliders
        ////////
        .insert_state(ProgramState::default())
        .insert_state(GameplayState::default())
        .insert_state(OverlayState::default())
        //////
        .add_systems(
            First,
            (
                bevy::dev_tools::states::log_transitions::<ProgramState>,
                bevy::dev_tools::states::log_transitions::<GameplayState>,
                bevy::dev_tools::states::log_transitions::<OverlayState>,
            ),
        )
        //////
        .add_plugins(EguiPlugin::default())
        .insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })
        .add_plugins(FpsOverlayPlugin::default())
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
            LoadingState::new(ProgramState::Initializing).continue_to_state(ProgramState::New),
        )

        .add_plugins(ActionPlugin)
        .insert_resource(actions::default_input_map())

        .add_plugins(MenuPlugin)
        .add_plugins(LifecyclePlugin)
        // .add_plugins(LevelStatePlugin)
        .add_plugins(GuiPlugin)
        .add_plugins(WorldUiPlugin)
        .add_plugins(WorldStatePlugin)
        .add_plugins(DebugPlugin)
        .add_plugins(AudioPlugin)
        .add_plugins(CrosshairPlugin)

        .add_plugins(PlayerCameraPlugin)
        .add_plugins(PlayerInputPlugin)
        .add_plugins(PlayerClientPlugin)
        .add_plugins(PlayerMovementPlugin)
        .add_plugins(PlayerControllerPlugin)

        .add_plugins(GamePlugin)

        .insert_resource(OurUser(default()))

        .insert_resource(PlayerInputSettings::for_space())

        // .add_loading_state(
        //         LoadingState::new(ProgramState::Initializing)
        //             .load_collection::<MusicAssets>()
        //             .load_collection::<FxAssets>()
        //     )

        .add_loading_state(
                LoadingState::new(GameplayState::AssetsLoading)
                    .continue_to_state(GameplayState::AssetsLoaded)
                    .load_collection::<SkyboxAssets>()
                    .load_collection::<MapAssets>()
            )

        .add_systems(OnEnter(ProgramState::Initializing), on_enter_initializing)
        .add_systems(
            OnEnter(ProgramState::New),
            (on_enter_loading, init_perf_ui).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::LaunchMenu),
            (on_enter_launch_menu,).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::InGame),
            (on_exit_launch_menu, on_enter_in_game).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::InGame),
            (ensure_3d_camera, show_3d_camera),
        )
        .add_systems(
            OnEnter(ProgramState::LaunchMenu),
            hide_3d_camera, //.in_set(SimulationSystems),
        )
        .insert_resource(VideoSettings::default())
        ////////
        .init_state::<ProgramState>()
        .init_state::<GameplayState>()
        .init_state::<LevelState>()
        .insert_resource(ProductName(PRODUCT_NAME.to_string()))
        .insert_resource(PauseState::new(false))
        .run();

    exit
}

fn ensure_3d_camera(
    mut commands: Commands,
    camera_q: Query<Entity, With<OurCamera>>,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
) {
    let use_clustered =
        bevy::pbr::decal::clustered::clustered_decals_are_usable(&render_device, &render_adapter);

    let ent = if let Ok(ent) = camera_q.single() {
        // Got one.
        ent
    } else {
        info!("Creating 3D camera");

        commands.spawn_empty().id()
    };

    // Force init.
    commands.insert_resource(VideoCameraSettingsChanged);
    commands.insert_resource(VideoEffectSettingsChanged);

    configure_3d_camera(commands.get_entity(ent).unwrap(), use_clustered);
}

fn configure_3d_camera(mut ent_commands: EntityCommands, use_clustered: bool) {
    info!("Setting up camera");
    ent_commands.insert((
        Name::new("Camera"),
        Camera3d::default(),
        Exposure { ev100: 10.0 },
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        Hdr,
        Projection::Perspective(PerspectiveProjection {
            // fov: std::f32::consts::PI / 5.0,
            fov: 75f32.to_radians(),
            ..default()
        }),
        OrderIndependentTransparencySettings::default(),
        Msaa::Off,

        DespawnOnExit(GameplayState::Playing),
        PlayerCamera(CameraMode::FirstPerson),
        OurCamera::default(),
        Transform::from_xyz(0., 1., 0.),

        // Audio is from the perspective of the camera.
        SpatialListener3D::default(),
    ));

    if !use_clustered {
        ent_commands.insert(DepthPrepass);
    }
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
            RenderLayers::from_layers(&[1]),
        ));
    }
}

fn on_enter_loading(mut commands: Commands) {
    commands.set_state(ProgramState::LaunchMenu);
}

fn on_enter_launch_menu(mut commands: Commands) {
    commands.set_state(OverlayState::MainMenu);
}

fn on_exit_launch_menu(state: Res<State<OverlayState>>, mut commands: Commands) {
    if state.get().is_menu() {
        commands.set_state(OverlayState::Hidden);
    }
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
