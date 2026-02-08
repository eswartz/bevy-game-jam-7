mod debug;
use crate::debug::*;

use std::time::Duration;

use avian3d::prelude::*;
use bevy::{
    asset::AssetMetaCheck,
    camera::visibility::NoFrustumCulling,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    gltf::GltfMeshName,
    prelude::*,
    scene::SceneInstanceReady,
    winit::WinitSettings,
};
use bevy_egui::{
    EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass,
};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
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

#[derive(Resource, Reflect, Default, Deref, DerefMut)]
#[reflect(Resource, Default)]
#[type_path = "game"]
struct PauseState(pub bool);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
struct Base(pub Entity, pub Transform);

#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
struct Shake(pub Vec3);

fn main() {
    App::new()
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f32(
                1.0 / 120.0,
            )),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f32(
                1.0 / 24.0,
            )),
        })
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                // Wasm builds will check for meta files (that don't exist) if this isn't set.
                // This causes errors and even panics in web builds on itch.
                // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                meta_check: AssetMetaCheck::Never,
                watch_for_changes_override: Some(true),
                ..default()
            }),
            SkeinPlugin::default(),
            PhysicsPlugins::default(),
        ))
        .add_plugins(avian3d::debug_render::PhysicsDebugPlugin::default()) // show colliders
        .add_plugins(EguiPlugin::default())
        .insert_resource(EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                enabled: true,
                text_config: TextFont::from_font_size(12.0),
                ..default()
            },
        })
        .insert_gizmo_config(
            PhysicsGizmos::default(),
            GizmoConfig {
                // enabled: true,
                enabled: false,
                depth_bias: -0.1,
                ..default()
            },
        )
        .insert_resource(PauseState::default())
        .insert_resource(Base(Entity::PLACEHOLDER, Transform::IDENTITY))
        .add_observer(observe_spawn_mesh)
        .add_systems(Startup, (startup,).chain())
        .add_systems(
            PreUpdate,
            (setup_egui_style, ensure_egui_context)
                .chain()
                .run_if(egui_not_initialized),
        )
        .add_systems(Update, (check_actions, check_ball_death, shake_base))
        .add_systems(PostUpdate, spawn_ball)
        .add_systems(EguiPrimaryContextPass, (update_egui_inspector_ui,))
        .run();
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
                .insert((NoFrustumCulling, MaxLinearSpeed(256.0)));

            if owner_name_is("boy_rig") || owner_name_is("Base") || owner_name_is("Tube") {
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

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Camera {
            // Render before 3D.
            order: -1,
            clear_color: ClearColorConfig::Default,
            ..default()
        },
        // RenderLayers::from_layers(&[1]),
    ));
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("test.glb")),
    ));
}

fn spawn_ball(
    mut commands: Commands,
    generator_q: Query<(Entity, &Transform), With<Generator>>,
    time: Res<Time>,
    assets: Res<AssetServer>,
    pause: Res<PauseState>,
    mut timer: Local<Timer>,
) {
    if **pause {
        return;
    }

    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(0.0125, TimerMode::Repeating);
        // *timer = Timer::from_seconds(1., TimerMode::Repeating);
    }
    if !timer.tick(time.delta()).is_finished() {
        return;
    }

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
    }
}

fn check_actions(
    keys: Res<ButtonInput<KeyCode>>,
    mut pause: ResMut<PauseState>,
    base: Res<Base>,
    time: Res<Time>,
    shake: Option<Res<Shake>>,
    mut commands: Commands,
    mut forces: Query<Forces>,
) {
    if keys.just_released(KeyCode::Pause) || keys.just_released(KeyCode::MediaPlayPause) {
        **pause = !**pause;
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
    mut commands: Commands,
    mut forces: Query<(&Transform, Forces)>,
) {
    if let Ok((xfrm, mut forces)) = forces.get_mut(base.0) {
        if let Some(shake) = shake {
            if let Ok(xfrm) = camera.single() {
                let force = shake.0 * 10000.0;
                forces.apply_local_linear_impulse(xfrm.rotation() * force);
                commands.remove_resource::<Shake>();
            }
        } else {
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
