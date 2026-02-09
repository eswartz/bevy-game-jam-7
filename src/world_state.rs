use std::collections::HashMap;
use std::time::Duration;

use bevy::animation::AnimationEvent;
use bevy::animation::AnimationTargetId;
use bevy::asset::RenderAssetUsages;
use bevy::ecs::entity::EntityHashMap;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use avian3d::prelude::*;

use bevy::core_pipeline::Skybox;
use bevy::camera::primitives::Aabb;
use bevy::scene::SceneInstance;
use bevy::scene::SceneInstanceReady;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState as _;
use bevy_asset_loader::loading_state::LoadingState;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;
use image::imageops::FilterType;

use crate::PlayerStart;
use crate::Shake;
use crate::Spawning;
use crate::level_state::LevelContentDefinedMessage;
use crate::level_state::LevelGeometryLoadedMessage;
use crate::level_state::LevelLoadFinishedMessage;
use crate::level_state::LevelState;
use crate::markers::GameLayer;
use crate::states_sets::GameplayState;
use crate::states_sets::OverlayState;
use crate::states_sets::ProgramState;
use crate::texutils::SkyboxTransform;
use crate::texutils::convert_strip_to_cubemap;
use crate::texutils::resize_for_quality;
use crate::video::TextureQuality;
use crate::video::VideoSettings;
// use image::imageops::FilterType;

pub struct WorldStatePlugin;

impl Plugin for WorldStatePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<WorldMarker>()
            .register_type::<NextLevelIndex>()
            .register_type::<DecorateGltfMeshes>()
            .insert_resource(Gravity((9.8 * Vec3::NEG_Y).into()))
            .add_loading_state(
                LoadingState::new(GameplayState::AssetsLoading)
                    .continue_to_state(GameplayState::AssetsLoaded)
                    .load_collection::<SkyboxAssets>()
                    // .load_collection::<SoundFxAssets>()
                    // .load_collection::<VoiceAssets>()
                    // .load_collection::<MusicAssets>()
                    // .load_collection::<IconAssets>()
                    .load_collection::<MapAssets>()
            )
            .add_systems(OnEnter(GameplayState::AssetsLoaded),
                (
                    transition_from_loading,
                    setup_world_marker,
                )
                // .in_set(SimulationSystems)
                .run_if(in_state(ProgramState::InGame))
            )
            // .add_systems(OnTransition{ exited: GameplayState::AssetsLoaded, entered: GameplayState::Playing },
            .add_systems(OnEnter(GameplayState::Playing),
                (
                    set_loading_overlay,
                    spawn_level,
                )
                .chain()
                // .in_set(SimulationSystems)
                .run_if(in_state(ProgramState::InGame))
            )
            .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
                (
                    despawn_world,
                )
                .chain()
                // .in_set(SimulationSystems)
            )
            .add_systems(OnExit(LevelState::Advance),
                (
                    set_loading_overlay,
                    despawn_world,
                    spawn_level,
                )
                .chain()
                // .in_set(SimulationSystems)
            )
            .add_systems(
                PreUpdate,
                    setup_world_bounds
                        .run_if(on_message::<LevelLoadFinishedMessage>)
                        .run_if(in_state(GameplayState::Playing))
                        // .in_set(SimulationSystems)
            )
            .add_systems(
                PreUpdate,
                    (
                        check_load_skybox,
                        check_load_reflection_probe,
                        fixup_blender_gltf_light_angles,
                        decorate_gltf_meshes,
                        assign_clips,
                        ground_rigid_bodies,
                    )
                    .run_if(in_state(ProgramState::InGame))
                    .run_if(in_state(GameplayState::Playing))
                    // .in_set(SimulationSystems)
            )
        ;
    }
}

// fn has_level(
//     regy: Res<LevelRegistry>,
//     request: Res<ChangeLevelRequest>,
// ) -> bool {
//     start_level.is_some_and(|start| !start.0.is_empty()) || *choice != StartLevelChoice::Test
// }

/// Mark entities that are specific to the gameplay world.
/// This only needs to be placed on toplevel parent entities.
///
/// The AABB reflects the full extent of the "valid content" of the world.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct WorldMarker(pub Aabb3d);

impl Default for WorldMarker {
    fn default() -> Self {
        Self(Aabb3d::new(Vec3::ZERO, Vec3::ONE))
    }
}

/// This marks an entity playing an AudioCue as background music / sound.
#[derive(Component, Clone, Reflect)]
// #[require(Saveable)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct BackgroundAudio;

// #[derive(Resource, Debug, Clone, Copy, PartialEq, Default, Reflect, EnumIter, strum_macros::Display, VariantArray)]
// #[reflect(Resource, Default)]
// #[type_path = "game"]

// pub enum StartLevelChoice {
//     #[strum(to_string = "Test")]
//     Test,
//     #[strum(to_string = "Tutorial 1")]
//     #[default]
//     Tutorial1,
//     #[strum(to_string = "Tutorial 2")]
//     Tutorial2,
// }

/// When defined, requests to change to the given level by index.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct NextLevelIndex(pub usize);

fn setup_world_bounds(
    mut commands: Commands,
    mut world_q: Query<(Entity, &mut WorldMarker)>,
    child_q: Query<&Children>,
    aabb_q: Query<&Aabb>,
) {
    commands.set_state(OverlayState::Hidden);

    if let Ok((world_ent, mut marker)) = world_q.single_mut() {
        let mut aabb3d = Aabb3d::new(Vec3::ZERO, Vec3::ONE);
        child_q.iter_descendants(world_ent).for_each(|ent| {
            if let Ok(aabb) = aabb_q.get(ent) {
                aabb3d.min = aabb3d.min.min(aabb.min());
                aabb3d.max = aabb3d.max.max(aabb.max());
            }
        });
        marker.0 = aabb3d;
    }
}

fn transition_from_loading(
    mut commands: Commands,
    // mut next_mode_state: ResMut<NextState<GameplayState>>,
    // post_asset_loading_state: Option<Res<SetStateAfterReload<GameplayState>>>
) {
    // if let Some(mode) = &post_asset_loading_state {
    //     // Reloading from save.
    //     next_mode_state.set(mode.0);
    //     commands.insert_resource(State::new(GameplayState::New));
    //     commands.remove_resource::<SetStateAfterReload<GameplayState>>();
    // } else {
    commands.set_state(GameplayState::Playing);
    // }
}

#[derive(Resource, AssetCollection)]
pub struct SkyboxAssets {
    /// Cache of width (narrow dimension) to cubemapped image.
    mapped_skyboxes: HashMap<(Handle<Image>, u32), Handle<Image>>,

    // #[asset(path = "textures/kloppenheim_06_puresky_4k.exr")]
    // #[allow(unused)]
    // pub kloppenheim_sky_map: Handle<Image>,

    // #[asset(path = "textures/starmap_2020_4k.exr")]
    // #[allow(unused)]
    // pub star_map: Handle<Image>,

    // #[asset(path = "textures/driving_school_4k.exr")]
    // #[allow(unused)]
    // pub driving_school: Handle<Image>,

    #[asset(path = "textures/farm_field_puresky_4k.exr")]
    #[allow(unused)]
    pub pure_sky: Handle<Image>,

    // #[asset(path = "textures/graffiti_shelter_4k.ktx2")]
    // #[allow(unused)]
    // pub graffiti_reflection: Handle<Image>,

    // #[asset(path = "textures/farm_field_4k.exr")]
    // #[allow(unused)]
    // pub farm_field: Handle<Image>,

    // #[asset(path = "textures/zwinger_night_4k.exr")]
    // #[allow(unused)]
    // pub zwinger: Handle<Image>,

}


impl SkyboxAssets {
    #[allow(unused)]
    pub const STAR_MAP_TRANSFORM : SkyboxTransform = SkyboxTransform::From1_0_2f_3f_4_5;
    #[allow(unused)]
    pub const DRIVING_SCHOOL_TRANSFORM : SkyboxTransform = SkyboxTransform::From1_0_2f_3f_4_5;
    #[allow(unused)]
    pub const PURE_SKY_TRANSFORM : SkyboxTransform = SkyboxTransform::From1_0_2f_3f_4_5;
    #[allow(unused)]
    pub const FARM_FIELD_TRANSFORM : SkyboxTransform = SkyboxTransform::From1_0_2f_3f_4_5;
    #[allow(unused)]
    pub const ZWINGER_TRANSFORM : SkyboxTransform = SkyboxTransform::From1_0_2f_3f_4_5;
    #[allow(unused)]
    pub const GRAFFITI_SHELTER_TRANSFORM : SkyboxTransform = SkyboxTransform::From1_0_2f_3f_4_5;

    pub fn get_openexr_skybox(&mut self, images: &mut Assets<Image>, source_image: Handle<Image>, quality: TextureQuality, transform: SkyboxTransform)
    -> Handle<Image> {
        let side_res = match quality {
            TextureQuality::Low => 256,
            TextureQuality::Medium => 512,
            TextureQuality::High => 1024,
            TextureQuality::Ultra => 1200,
        };

        // Already cached?
        let key = (source_image.clone(), side_res);
        if let Some(skybox_image) = self.mapped_skyboxes.get(&key) {
            return skybox_image.clone();
        }

        let Some(source_image) = images.get(&source_image) else {
            // This can persist for many frames...
            return default()
        };

        let resized_image = if let Some(dyn_image) = resize_for_quality(
            source_image, side_res, side_res * 6, FilterType::Nearest) {
            &Image::from_dynamic(dyn_image, true,
                // since we convert it again just below
                RenderAssetUsages::MAIN_WORLD)
        } else {
            // Don't resize or let any error propagate.
            source_image
        };
        let image = convert_strip_to_cubemap(resized_image, transform).expect("we created data");
        let skybox_image = images.add(image);

        self.mapped_skyboxes.insert(key, skybox_image.clone());
        skybox_image
    }
}

/// Set this component when you wish to load a skybox asynchronously
/// (given that it may take a long time to load the texture).
/// The `Skybox::image` will be scaled to the desired video settings'
/// resolution, converted to a cubemap, then provide a Skybox directly
/// in place of the component.
/// If the reflection probe option is set, apply it with the given brightness.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SkyboxModel{
    pub skybox: Skybox,
    pub xfrm: SkyboxTransform,
    pub with_reflection_probe: Option<(Entity, f32)>,
    pub enabled: bool,
}

/// Generic system to check for any LoadSkybox component, and if found,
/// make sure its image is loaded. Once loaded, convert it to a cubemap
/// and apply to the camera, then remove the component.
pub(crate) fn check_load_skybox(
    load_skybox_q: Query<(Entity, &SkyboxModel), Changed<SkyboxModel>>,
    mut commands: Commands,
    video_settings: Res<VideoSettings>,
    mut images: ResMut<Assets<Image>>,
    mut skyboxes: ResMut<SkyboxAssets>,
) {
    // use bevy::render::render_resource::*;
    let Some((cam, SkyboxModel{ skybox, xfrm, with_reflection_probe, enabled })) = load_skybox_q.iter().next() else {
        return
    };

    if !*enabled {
        commands.entity(cam).remove::<Skybox>();
        commands.entity(cam).remove::<LightProbe>();
        commands.entity(cam).remove::<EnvironmentMapLight>();
        return;
    }

    let quality = video_settings.texture_quality;
    // let (skybox, transform) = (skyboxes.pure_sky.clone(), SkyboxAssets::PURE_SKY_TRANSFORM);
    // let (skybox, transform) = (skyboxes.farm_field.clone(), SkyboxAssets::FARM_FIELD_TRANSFORM);
    let skybox_image = skyboxes.get_openexr_skybox(&mut images, skybox.image.clone(), quality, *xfrm);

    if skybox_image != Handle::default() {
        let mut sky = skybox.clone();
        sky.image = skybox_image.clone();
        commands.entity(cam).insert(sky);

        if let Some((ent, brightness)) = with_reflection_probe {
            commands.entity(*ent).insert((
                ReflectionProbeModel{
                    image: skybox_image,
                    brightness: *brightness,
                },
            ));
        }
    }
}


/// Set this component when you wish to load a reflection probe asynchronously
/// (given that it may take a long time to load the texture).
#[derive(Component)]
pub struct ReflectionProbeModel {
    pub(crate) image: Handle<Image>,
    pub(crate) brightness: f32,
}

/// Generic system to check for any LoadSkybox component, and if found,
/// make sure its image is loaded. Once loaded, convert it to a cubemap
/// and apply to the camera, then remove the component.
pub(crate) fn check_load_reflection_probe(
    load_probe_q: Query<(Entity, &ReflectionProbeModel), Changed<ReflectionProbeModel>>,
    world_q: Query<Entity, With<WorldMarker>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    use bevy::render::render_resource::*;
    let Some((entity, ReflectionProbeModel{ image, brightness })) = load_probe_q.iter().next() else {
        return
    };

    if *image == Handle::default() {
        return
    }

    if images.get(image).is_none() {
        // This can persist for many frames...
        return
    };

    // Make a solid diffuse map.
    let extents = Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 6
    };

    const B: u8 = 192;
    let mut diffuse = Image::new_fill(
        extents,
        TextureDimension::D2,
        &[
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
        ],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    diffuse.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    let diffuse = images.add(diffuse);

    let reflection_image = images.get_mut(image).unwrap();

    reflection_image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    commands.entity(entity).insert((
        LightProbe,
        EnvironmentMapLight {
            diffuse_map: diffuse.clone(),
            specular_map: image.clone(),
            intensity: *brightness,
            affects_lightmapped_mesh_diffuse: false,
            ..default()
        },
    ));

    if let Ok(world) = world_q.single() {
        commands.spawn((
            Name::new("Reflection Probe"),
            LightProbe,
            EnvironmentMapLight {
                diffuse_map: diffuse.clone(),
                specular_map: image.clone(),
                intensity: *brightness,
                affects_lightmapped_mesh_diffuse: false,
                ..default()
            },
            Transform::from_scale(Vec3::splat(100.0)),
            ChildOf(world),
        ));
    } else {
        error!("no single WorldMarker entity found @ world");
    }
}

#[derive(Resource, AssetCollection)]
pub struct MapAssets {
    #[asset(path = "test.glb#Scene0")]
    pub level_test: Handle<Scene>,
    // #[asset(path = "models/robot.glb")]
    // pub robot_gltf: Handle<Gltf>,
    // #[asset(path = "models/robot.glb#Scene0")]
    // pub robot_model: Handle<Scene>,
    // #[asset(path = "models/boy_looking_at_phone.glb#Scene0")]
    // pub boy_with_phone_model: Handle<Scene>,
    // #[asset(path = "models/hand.glb#Scene0")]
    // pub hand_model: Handle<Scene>,
    // #[asset(path = "materials/bullet.mat")]
    // pub bullet_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/hand.mat")]
    // pub hand_material: Handle<GenericMaterial>,

    // #[asset(path = "materials/area_danger.mat")]
    // pub area_danger_material: Handle<GenericMaterial>,

    // #[asset(path = "textures/tile_diffuse.webp")]
    // #[asset(image(sampler(filter = linear, wrap = repeat)))]
    // pub tile_diffuse_texture: Handle<Image>,
    // #[asset(path = "textures/tile_normal.webp")]
    // #[asset(image(sampler(filter = linear, wrap = repeat)))]
    // pub tile_normal_texture: Handle<Image>,

    // #[asset(path = "materials/tile_nothing.mat")]
    // pub tile_nothing_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_start.mat")]
    // pub tile_start_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_finish.mat")]
    // pub tile_finish_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_red.mat")]
    // pub tile_red_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_green.mat")]
    // pub tile_green_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_blue.mat")]
    // pub tile_blue_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_yellow.mat")]
    // pub tile_yellow_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_purple.mat")]
    // pub tile_purple_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_pink.mat")]
    // pub tile_pink_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_orange.mat")]
    // pub tile_orange_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_white.mat")]
    // pub tile_white_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_black.mat")]
    // pub tile_black_material: Handle<GenericMaterial>,
    // #[asset(path = "materials/tile_grey.mat")]
    // pub tile_grey_material: Handle<GenericMaterial>,
}

pub(crate) fn setup_world_marker(
    mut commands: Commands,
) {
    info!("Entering simulation");

    commands.spawn((
        Name::new("World"),
        DespawnOnExit(ProgramState::InGame),
        WorldMarker::default(),
        Transform::IDENTITY,
        Visibility::Inherited,
    ));
}

/// Adjust lights on gltf import.
///
/// 1) When scaling the geometry, also scale the intensity/range/etc.
/// 2) Enable shadows (gltf export in Blender 4.4.3 doesn't pass along)
pub fn adjust_lights(
    target: Entity,
    mut light_q: Query<&mut PointLight>,
    child_q: Query<&Children>,
    scale: f32,
) {
    for ent in child_q.iter_descendants(target) {
        if let Ok(mut light) = light_q.get_mut(ent) {
            light.intensity *= scale;
            light.range *= scale;
            light.radius *= scale;

            light.shadows_enabled = true;
            light.soft_shadows_enabled = true;
            light.affects_lightmapped_mesh_diffuse = false;
        }
    }
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct FixupBlenderGltfLights;

/// Marker asking system to convert embedded lights to use gltfs from Blender,
/// with which Bevy seems to disagree about whether angles are radians or hemisphere degrees.
fn fixup_blender_gltf_light_angles(
    mut commands: Commands,
    instance_q: Query<Entity, (With<FixupBlenderGltfLights>, With<SceneRoot>, Added<SceneInstance>)>,
    child_q: Query<&Children>,
    mut spotlight_q: Query<&mut SpotLight>,
) {
    for scene in instance_q.iter() {
        for ent in child_q.iter_descendants(scene) {
            if let Ok(mut light) = spotlight_q.get_mut(ent) {
                light.outer_angle = light.outer_angle.to_degrees() / 2.0;
                light.inner_angle = light.inner_angle.to_degrees() / 2.0;
                light.shadows_enabled = true;
            }
        }
        commands.entity(scene).remove::<FixupBlenderGltfLights>();
    }
}

/// Marker for a constructor hierarchy to add to a loaded glTF scene (parallel to [SceneRoot]).
/// This is an indirection that allows using this inside e.g. trenchbroom loaders
/// or scene instancing. (Adding CCH directly seems to get lost in practice.)
#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct DecorateGltfMeshes {
    cch: ColliderConstructorHierarchy,
    events: bool,
}

impl DecorateGltfMeshes {
    pub fn new(cch: ColliderConstructorHierarchy) -> Self {
        Self {
            cch,
            events: false,
        }
    }
    pub fn with_events(self, events: bool) -> Self {
        Self {
            events,
            .. self
        }
    }
}

pub fn decorate_gltf_meshes(
    mut commands: Commands,
    decorate_gltf_q: Query<(Entity, &DecorateGltfMeshes), (With<SceneRoot>, Added<SceneInstance>)>,
    child_q: Query<&Children>,
    mesh_q: Query<&Mesh3d>,
) {
    for (ent, DecorateGltfMeshes{ cch, events }) in decorate_gltf_q.iter() {
        if *events {
            for kid in child_q.iter_descendants(ent) {
                if mesh_q.contains(kid) {
                    commands.entity(kid).insert(CollisionEventsEnabled);
                }
            }
        }
        // Forward request to Avian to do all the other work generating colliders.
        let mut ent_commands = commands.entity(ent);
        ent_commands.insert((cch.clone(), RigidBody::Static));
        ent_commands.remove::<DecorateGltfMeshes>();
    }
}

pub fn generate_collisions(
    target: Entity,
    mut commands: Commands,
    name_q: Query<&Name>,
    child_q: Query<&Children>,
    mesh_q: Query<&Mesh3d>,
    bundle: impl Bundle + Clone
) {
    for ent in child_q.iter_descendants(target) {
        if mesh_q.get(ent).is_ok() {
            if let Ok(name) = name_q.get(ent)
            && name.ends_with(":nocol") {
                continue;
            }

            generate_collisions_on(commands.entity(ent), bundle.clone());
        }
    }
}

pub fn generate_collisions_on(
    mut ent_commands: EntityCommands,
    bundle: impl Bundle
) {
    ent_commands.insert((
        RigidBody::Static,
        CollisionLayers::new(GameLayer::World, [GameLayer::Player, GameLayer::Default, GameLayer::Gameplay]),
        ColliderConstructor::TrimeshFromMeshWithConfig(
            TrimeshFlags::FIX_INTERNAL_EDGES
        ),
    ));
    ent_commands.insert(bundle);
}

fn set_loading_overlay(
    mut commands: Commands,
) {
    commands.set_state(OverlayState::Loading);
}

pub(crate) fn spawn_level(
    mut commands: Commands,
    assets: Res<AssetServer>,
    // level_regy: Res<LevelRegistry>,
    // next_level: Res<NextLevelIndex>,
    // // lighting_animators: ResMut<bevy_trenchbroom::prelude::LightingAnimators>,
    // meshes: ResMut<Assets<Mesh>>,
    // materials: ResMut<Assets<StandardMaterial>>,
    map_assets: Res<MapAssets>,
    // world_q: Query<Entity, With<WorldMarker>>,
 ) {
    // let level_id = level_regy.level_index_id(next_level.0);
    // let Some(level_info) = level_regy.find_level(&level_id) else { error!("no registered level '{level_id}'"); return };
    // let Ok(world) = world_q.single() else { error!("No single WorldMarker found!"); return };

    // if let Some(bsp_path) = level_info.bsp_path {
    //     let scale = Vec3::splat(1.0);
    //     commands.insert_resource(GlobalAmbientLight::NONE);
    //     let mut ent_commands = commands.spawn((
    //         Name::new("BSP Room"),
    //         DespawnOnExit(GameplayState::Playing),
    //         Transform::from_scale(scale),
    //         ChildOf(world),
    //     ));

    //     let world_ent = ent_commands.id();
    //     ent_commands.commands().write_message(LevelCreateMessage {
    //     kind: LevelLoadKind::New,
    //         root: world_ent,
    //     });

    //     ent_commands.commands().insert_resource(
    //         LevelRoot {
    //             root: world_ent,
    //             level_id: level_id.clone(),
    //         }
    //     );

    //     // Load asynchronously.
    //     let bsp_path = format!("{bsp_path}#Scene");
    //     add_bsp_room(ent_commands, assets,
    //         // lighting_animators,
    //         bsp_path);
    // } else {
    //     spawn_test_world(commands, assets, meshes, materials, map_assets, world, level_id);
    // }

    // commands.spawn(SceneRoot(
    //     assets.load(GltfAssetLabel::Scene(0).from_asset("test.glb")),
    // ))
    // .observe(|_ready: On<SceneInstanceReady>, mut commands: Commands
    //     , mut time: ResMut<Time<Physics>>
    //     | {
    //     // commands.insert_resource(NextState::Pending(ProgramState::LaunchMenu));
    //     time.pause();
    // })
    // ;
    commands.spawn((
        DespawnOnExit(GameplayState::New),
        SceneRoot(map_assets.level_test.clone()),
    ))
        .observe(|_ready: On<SceneInstanceReady>,
            player_q: Query<&Transform, With<PlayerStart>>,
            camera_q: Query<Entity, With<Camera3d>>,
            mut commands: Commands| {
                if let Ok(xfrm) = player_q.single()
                && let Ok(camera) = camera_q.single()  {
                    commands.entity(camera).insert(xfrm.clone());
                } else {
                    log::error!("no PlayerStart");
                }

                commands.insert_resource(Spawning(true));
                commands.insert_resource(Shake(Vec3::ZERO));

                commands.write_message(LevelGeometryLoadedMessage);
                commands.write_message(LevelContentDefinedMessage);
                commands.write_message(LevelLoadFinishedMessage);
        });

}

pub(crate) fn despawn_world(
    ent_q: Query<Entity, With<WorldMarker>>,
    child_q: Query<&Children>,
    mut commands: Commands,
) {
    for ent in ent_q.iter() {
        for kid in child_q.iter_descendants(ent) {
            commands.entity(kid).try_despawn();
        }
    }
}

/// Marks the entity with a Gltf subtree being loaded and indicates
/// we want to assign animation clips.
#[derive(Component)]
pub struct GltfAnimatedModel(pub Handle<Gltf>);

///// copied from animation_plugin.rs in bevy/examples/tools/scene_viewer/src

/// Controls animation clips for a unique entity.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct NamedAnimationClips {
    named_clips: HashMap<String, AnimationNodeIndex>,
}
impl NamedAnimationClips {
    pub fn new(named_clips: HashMap<String, AnimationNodeIndex>) -> Self {
        NamedAnimationClips {
            named_clips,
        }
    }
    pub fn clip_named(&self, clip_name: &str) -> Option<AnimationNodeIndex> {
        if let Some(index) = self.named_clips.get(clip_name) {
            return Some(*index)
        }
        None
    }
}

/// Automatically assign [`AnimationClip`]s to [`AnimationPlayer`] and play
/// them, if the clips refer to descendants of the animation player (which is
/// the common case).
fn assign_clips(
    models_q: Query<(Entity, &GltfAnimatedModel), (With<SceneRoot>, Without<NamedAnimationClips>)>,
    mut players_q: Query<&mut AnimationPlayer>,
    targets_q: Query<(Entity, &AnimationTargetId)>,
    parent_q: Query<&ChildOf>,
    mut clips: ResMut<Assets<AnimationClip>>,
    gltf_assets: Res<Assets<Gltf>>,
    assets: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    // Make sure there's something expecting it.
    let Some((model_ent, gltf_model)) = models_q.iter().next() else {
        return
    };

    // Placeholder in case of errors.
    commands.entity(model_ent).insert(NamedAnimationClips::default());

    let Some(gltf) = gltf_assets.get(&gltf_model.0) else {
        error!("missing gltf model {:?} for {model_ent}", gltf_model.0);
        return
    };

    let animations = &gltf.animations;
    if animations.is_empty() {
        return;
    }

    let count = animations.len();
    let plural = if count == 1 { "" } else { "s" };
    debug!("Found {} animation{plural}", animations.len());
    let names: Vec<_> = gltf.named_animations.keys().collect();
    debug!("Animation names: {names:?}");

    // Map animation target IDs to entities.
    let animation_target_id_to_entity: HashMap<_, _> = targets_q
        .iter()
        .map(|(entity, target)| (target, entity))
        .collect();

    // Build up a list of all animation clips that belong to each player. A clip
    // is considered to belong to an animation player if all targets of the clip
    // refer to entities whose nearest ancestor player is that animation player.

    let mut player_to_graph: EntityHashMap<(AnimationGraph, HashMap<String, AnimationNodeIndex>)> =
        EntityHashMap::default();

    for (clip_id, clip) in clips.iter_mut() {
        debug!("Found {clip_id:?}");
        let mut ancestor_player = None;
        for target_id in clip.curves().keys() {
            // If the animation clip refers to entities that aren't present in
            // the scene, bail.
            let Some(&target) = animation_target_id_to_entity.get(target_id) else {
                continue;
            };

            // Find the nearest ancestor animation player.
            let mut current = Some(target);
            while let Some(entity) = current {
                if players_q.contains(entity) {
                    match ancestor_player {
                        None => {
                            // If we haven't found a player yet, record the one
                            // we found.
                            ancestor_player = Some(entity);
                        }
                        Some(ancestor) => {
                            // If we have found a player, then make sure it's
                            // the same player we located before.
                            if ancestor != entity {
                                // It's a different player. Bail.
                                debug!("Ignoring {ancestor} != {entity}");
                                ancestor_player = None;
                                break;
                            }
                        }
                    }
                }

                // Go to the next parent.
                current = parent_q.get(entity).ok().map(ChildOf::parent);
            }
        }

        let Some(ancestor_player) = ancestor_player else {
            debug!("Unexpected animation hierarchy for animation clip {}; ignoring.", clip_id);
            continue;
        };

        let Some(clip_handle) = assets.get_id_handle(clip_id) else {
            debug!("Clip {} wasn't loaded.", clip_id);
            continue;
        };

        let &mut (ref mut graph, ref mut named_clips) = player_to_graph.entry(ancestor_player).or_default();
        let node_index = graph.add_clip(clip_handle, 1.0, graph.root);

        let mut anim_name = None;
        for (name, aclip) in &gltf.named_animations {
            if aclip.id() == clip_id {
                anim_name = Some(name.to_string());
                break;
            }
        }

        let anim_name = anim_name.unwrap_or_else(|| format!("{clip_id}"));

        info!("Clip {anim_name} is {:?}", Duration::from_secs_f32(clip.duration()));
        clip.add_event(clip.duration() + 1.0 / 15.0, ClipFinished{ entity: Entity::PLACEHOLDER, index: node_index });

        named_clips.insert(anim_name, node_index);
    }

    if player_to_graph.is_empty() {
        // Remove placeholder, try again.
        // warn!("No clips loaded, retrying");
        commands.entity(model_ent).remove::<NamedAnimationClips>();
        return;
    }

    // Now that we've built up a list of all clips that belong to each player,
    // package them up into a `Clips` component, play the first such animation,
    // and add that component to the player.
    for (player_entity, (graph, named_clips)) in player_to_graph {
        let Ok(mut player) = players_q.get_mut(player_entity) else {
            warn!("Animation targets referenced a nonexistent player. This shouldn't happen.");
            continue;
        };
        let graph = graphs.add(graph);
        if let Some(reset) = named_clips.get("RESET") {
            info!("Resetting {player_entity}");
            player.play(*reset).replay();
        }
        let animations = NamedAnimationClips::new(named_clips);

        commands
            .entity(player_entity)
            .insert(animations)
            .insert(AnimationGraphHandle(graph))
            .observe(on_clip_finished);
    }
}

#[derive(AnimationEvent, Reflect, Clone)]
pub struct ClipFinished {
    pub entity: Entity,
    pub index: AnimationNodeIndex,
}


impl bevy::ecs::event::EntityEvent for ClipFinished {
    fn event_target(&self) -> bevy::ecs::entity::Entity {
        self.entity
    }
}

impl bevy::ecs::event::SetEntityEventTarget for ClipFinished {
    fn set_event_target(&mut self, entity: Entity) {
        self.entity = entity;
    }
}


fn on_clip_finished(
    message: On<ClipFinished>,
    mut players_q: Query<&mut AnimationPlayer>,
) {
    let player_ent = message.trigger().target;
    if let Ok(mut player) = players_q.get_mut(player_ent) {
        info!("Stopped {:?}", player_ent);
        player.stop_all();
    } else {
        dbg!(player_ent);
    }
}


/// Marks the entity as the root of a model which we want to align to
/// the ground (typically upward) to ensure it won't clip through the
/// ground and lead to an explosive collision fixup.
#[derive(Component)]
pub struct GroundRigidBodyAndEnable(pub RigidBody);

pub fn ground_rigid_bodies(
    mut commands: Commands,
    new_model_q: Query<(Entity, Option<&RigidBody>), Added<ColliderAabb>>,
    mut ground_model_q: Query<(&GroundRigidBodyAndEnable, &mut Transform)>,
    mut watch: Local<EntityHashMap<u8>>,
    collisions: Collisions,
) {
    // See if we get new candidates.
    for (ent, rigid_opt) in new_model_q.iter() {
        if ground_model_q.contains(ent) {
            if rigid_opt == Some(&RigidBody::Kinematic) || rigid_opt == Some(&RigidBody::Dynamic) {
                watch.insert(ent, 3);
            } else {
                error!("ignoring GroundRigidBodyAndEnable on {ent} since it lacks RigidBody::{{Kinematic,Dynamic}}");
            }
        }
    }

    if watch.is_empty() { return };

    for (ent, frames) in watch.clone() {
        if let Ok((GroundRigidBodyAndEnable(new_body), mut xfrm)) = ground_model_q.get_mut(ent) {
            let mut any = false;
            let mut max_y = 0.0f32;
            for pair in collisions.collisions_with(ent) {
                if let Some(contact) = pair.find_deepest_contact() {
                    // OK, we're colliding. Move away.
                    // let local = if pair.collider1 == ent {
                    //     contact.local_point1
                    // } else {
                    //     contact.local_point2
                    // };
                    // max_y = max_y.max(-local.y as f32);
                    max_y = max_y.max(-contact.penetration);
                    any = true;
                }
            }
            if any || frames <= 1 {
                watch.remove(&ent);
                if any {
                    info!("GroundRigidBodyAndEnable: moving {ent} by {max_y}");
                    xfrm.translation.y += max_y;
                } else {
                    error!("GroundRigidBodyAndEnable: no collisions detected on {ent}");
                }
                // Update type.
                commands.entity(ent).insert(*new_body);
                continue;
            }

            // No collisions yet.
            debug!("GroundRigidBodyAndEnable: no collisions detected, waiting {ent} for {}", frames - 1);
            watch.insert(ent, frames - 1);
        }
    }
}
