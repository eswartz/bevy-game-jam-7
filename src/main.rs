mod common;
mod menus;
mod assets;
mod audio;
mod player_spawning;

use crate::assets::*;
use crate::audio::AudioPlugin;
use crate::menus::MenuPlugin;
use crate::player_spawning::spawn_player;
use bevy::asset::uuid::Uuid;
use bevy::audio::PlaybackSettings;
use bevy::ecs::world::CommandQueue;
use bevy_seedling::prelude::*;
use common::*;
use rand::RngExt;

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
    camera::visibility::NoFrustumCulling,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    gltf::GltfMeshName,
    image::{ImageAddressMode, ImageSamplerDescriptor},
    scene::SceneInstanceReady,
    winit::WinitSettings,
};
use bevy_asset_loader::prelude::*;
use bevy_egui::input::egui_wants_any_keyboard_input;
use bevy_egui::{EguiGlobalSettings, EguiPlugin};
use bevy_skein::SkeinPlugin;

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
        .add_plugins(MenuPlugin)
        .add_plugins(LifecyclePlugin)
        // .add_plugins(LevelStatePlugin)
        .add_plugins(GuiPlugin)
        .add_plugins(WorldUiPlugin)
        .add_plugins(WorldStatePlugin)
        .add_plugins(DebugPlugin)
        .add_plugins(AudioPlugin)

        .add_plugins(PlayerCameraPlugin)
        .add_plugins(PlayerInputPlugin)
        .add_plugins(PlayerClientPlugin)
        .add_plugins(PlayerMovementPlugin)
        .add_plugins(PlayerControllerPlugin)

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
                    // .load_collection::<IconAssets>()
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
        .insert_resource(Spawning(false))
        .insert_resource(Base(Entity::PLACEHOLDER, Transform::IDENTITY))
        .add_observer(observe_spawn_mesh)
        // .add_systems(
        //     OnEnter(GameplayState::AssetsLoaded), on_enter_initializing)
        .add_systems(
            PreUpdate,
            (setup_egui_style, ensure_egui_context)
                .chain()
                .run_if(egui_not_initialized)
                .run_if(in_state(GameplayState::Playing)),
        )

        .add_systems(OnEnter(GameplayState::Setup),
            spawn_level
            // .in_set(SimulationSystems)
            // .run_if(in_state(ProgramState::InGame)) // redundant
        )
        .add_systems(OnEnter(GameplayState::Playing),
            spawn_player_on_start,
            // .in_set(SimulationSystems)
            // .run_if(in_state(ProgramState::InGame)) // redundant
        )
        .add_systems(
            Update,
            (check_ball_death,
            // move_camera_around,
            // aim_camera_around,
            ).run_if(in_state(LevelState::Playing)),
        )
        .add_systems(
            Update,
            (shake_base, check_actions)
                .run_if(not(is_menu_paused))
                .run_if(not(egui_wants_any_keyboard_input))
                .run_if(in_state(LevelState::Playing))
            ,
        )
        .add_systems(
            Update,
            (spawn_ball,).run_if(in_state(LevelState::Playing)),
        )
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
        // OrderIndependentTransparencySettings::default(),
        // Msaa::Off,

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

            commands
                .entity(entity)
                .insert((
                    NoFrustumCulling,
                    MaxLinearSpeed(256.0),
                    CollisionLayers::new(
                        GameLayer::World,
                        [GameLayer::Default, GameLayer::World, GameLayer::Player, GameLayer::Projectiles]
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

    let mut start_q = world.query_filtered::<&Transform, (With<PlayerStart>,Without<OurPlayer>)>();
    let Ok(xfrm) = start_q.single(world) else { log::error!("no PlayerStart or OurPlayer"); return; };
    drop(start_q);
    let xfrm = xfrm.clone();

    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);

    // Put the new Player where the PlayerStart is.
    commands.entity(ent).insert(xfrm);

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

    let level = commands.spawn((
        SceneRoot(map_assets.level_test.clone()),
    ))
        .observe(|_ready: On<SceneInstanceReady>,
            mut commands: Commands| {
                commands.insert_resource(Spawning(true));
                commands.insert_resource(Shake(Vec3::ZERO));
        }).id();

    commands.entity(*world).add_child(level);
}

fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
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
                offset: Vec3::new(1000.0, 1000.0, 1000.0).into(),
                ..default()
            }],

            SamplePlayer::new(fx.belch.clone()),
            // SamplePlayer::new(fx.tone.clone()),
            PlaybackSettings {
                speed: rng.random_range(0.75 .. 1.25),
                ..default()
            },
            VolumeNode::from_linear(rng.random_range(0.1 .. 1.0)),
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
        return
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
                    commands.spawn((
                        UiSfx,
                        SamplePlayer::new(fx.shake.clone()),
                    ));
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

fn move_camera_around(
    time: Res<Time<Physics>>,
    camera_q: Single<(Entity, &Transform), With<Camera3d>>,
    mut forces_q: Query<Forces>,
) {
    if time.is_paused() {
        return
    }

    // Move some.
    let mut rng = rand::rng();
    let diff = Vec3::new(
        rng.random_range(-1.0 ..= 1.0),
        rng.random_range(-1.5 ..= 1.0),
        rng.random_range(-2.0 ..= 1.0),
    );
    let orig_pos = camera_q.1.translation;
    let diff_rot = camera_q.1.rotation * diff;

    let new_pos = orig_pos + diff_rot * 50.0 * time.delta_secs();

    // camera_q.translation += diff_rot * 10.0 * time.delta_secs();
    if let Ok(mut forces) = forces_q.get_mut(camera_q.0) {
        forces.apply_linear_impulse((new_pos - orig_pos) * time.delta_secs());
    }
}

fn aim_camera_around(
    time: Res<Time<Physics>>,
    mut camera_q: Single<(Entity, &mut Transform), With<Camera3d> /* With<OurCamera>, */>,
) {
    if time.is_paused() {
        return
    }

    let target = camera_q.1.looking_at(Vec3::ZERO, Vec3::Y);
    camera_q.1.rotation = camera_q.1.rotation.lerp(target.rotation, 0.5);
}
