use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use avian3d::prelude::*;

use bevy::core_pipeline::Skybox;
use bevy::scene::SceneInstanceReady;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState as _;
use bevy_asset_loader::loading_state::LoadingState;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;
use image::imageops::FilterType;

use crate::common::*;
use super::lifecycle::PauseState;
use super::states_sets::GameplayState;
use super::states_sets::LevelState;
use super::states_sets::OverlayState;
use super::states_sets::ProgramState;
use super::texutils::SkyboxTransform;
use super::texutils::convert_strip_to_cubemap;
use super::texutils::resize_for_quality;
use super::video::TextureQuality;
use super::video::VideoSettings;

pub struct WorldStatePlugin;

impl Plugin for WorldStatePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<WorldMarker>()
            .register_type::<NextLevelIndex>()
            // .register_type::<DecorateGltfMeshes>()
            .insert_resource(Gravity((9.8 * Vec3::NEG_Y).into()))
            // .add_loading_state(
            //     LoadingState::new(GameplayState::AssetsLoading)
            //         .continue_to_state(GameplayState::AssetsLoaded)
            //         .load_collection::<SkyboxAssets>()
            //         // .load_collection::<SoundFxAssets>()
            //         // .load_collection::<VoiceAssets>()
            //         // .load_collection::<MusicAssets>()
            //         // .load_collection::<IconAssets>()
            //         .load_collection::<MapAssets>()
            // )
            .add_systems(OnEnter(GameplayState::AssetsLoaded),
                (
                    transition_from_loading,
                    setup_world_marker,
                )
                // .in_set(SimulationSystems)
                .run_if(in_state(ProgramState::InGame))
            )
            // .add_systems(OnTransition{ exited: GameplayState::AssetsLoaded, entered: GameplayState::Playing },

            // .add_systems(OnEnter(GameplayState::AssetsLoading),
            //     set_loading_overlay,
            // )
            .add_systems(OnExit(GameplayState::AssetsLoaded),
                start_next_level,
            )

            // // First time running.
            // .add_systems(OnEnter(GameplayState::Setup),
            //     (
            //         spawn_level,
            //     )
            //     .chain()
            //     // .in_set(SimulationSystems)
            //     // .run_if(in_state(ProgramState::InGame)) // redundant
            // )
            // // Between levels.
            // .add_systems(OnExit(LevelState::Advance),
            //     (
            //         despawn_world,
            //         start_next_level,
            //     )
            //     .chain()
            //     // .run_if(in_state(ProgramState::InGame)) // redundant
            //     // .in_set(SimulationSystems)
            // )

            // .add_systems(OnEnter(LevelState::Setup),
            //     start_level_setup,
            // )
            // .add_systems(OnEnter(LevelState::Advance),
            //     start_level_setup,
            // )

            .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
                (
                    despawn_world,
                )
                .chain()
                // .in_set(SimulationSystems)
            )
            // .add_systems(
            //     PreUpdate,
            //         setup_world_bounds
            //             .run_if(on_message::<LevelLoadFinishedMessage>)
            //             .run_if(in_state(GameplayState::Playing))
            //             // .in_set(SimulationSystems)
            // )
            .add_systems(
                PreUpdate,
                    (
                        check_load_skybox,
                        check_load_reflection_probe,
                        // fixup_blender_gltf_light_angles,
                        // decorate_gltf_meshes,
                        // assign_clips,
                        // ground_rigid_bodies,
                        check_level_setup,
                    )
                    .chain()
                    .run_if(in_state(ProgramState::InGame))
                    // .run_if(in_state(LevelState::Setup))
                    .run_if(in_state(GameplayState::Setup))
                    // .in_set(SimulationSystems)
            )
        ;
    }
}

/// This resource exists while [LevelState::Setup] is active.
#[derive(Resource, Debug, Default, Reflect, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct WorldSetup {
    pub(crate) waiting_skybox: bool,
    pub(crate) waiting_reflections: bool,
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

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct PlayerStart;

/// This marks an entity playing an AudioCue as background music / sound.
#[derive(Component, Clone, Reflect)]
// #[require(Saveable)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct BackgroundAudio;

/// When defined, requests to change to the given level by index.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct NextLevelIndex(pub usize);


fn transition_from_loading(
    mut commands: Commands,
) {
    commands.set_state(GameplayState::Setup);
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
        let image = convert_strip_to_cubemap(resized_image, transform).unwrap();
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
    mut setup: ResMut<WorldSetup>,
) {
    // use bevy::render::render_resource::*;
    let Some((cam, SkyboxModel{ skybox, xfrm, with_reflection_probe, enabled })) = load_skybox_q.iter().next() else {
        setup.waiting_skybox = false;
        dbg!(&*setup);
        return
    };

    if !*enabled {
        commands.entity(cam).remove::<Skybox>();
        commands.entity(cam).remove::<LightProbe>();
        commands.entity(cam).remove::<EnvironmentMapLight>();
        setup.waiting_skybox = false;
        return;
    }

    let quality = video_settings.texture_quality;
    // let (skybox, transform) = (skyboxes.pure_sky.clone(), SkyboxAssets::PURE_SKY_TRANSFORM);
    // let (skybox, transform) = (skyboxes.farm_field.clone(), SkyboxAssets::FARM_FIELD_TRANSFORM);
    let skybox_image = skyboxes.get_openexr_skybox(&mut images, skybox.image.clone(), quality, *xfrm);

    if skybox_image == Handle::default() {
        // Still waiting.
        return;
    }

    let mut sky = skybox.clone();
    sky.image = skybox_image.clone();
    commands.entity(cam).insert(sky);
    setup.waiting_skybox = false;

    if let Some((ent, brightness)) = with_reflection_probe {
        commands.entity(*ent).insert((
            ReflectionProbeModel{
                image: skybox_image,
                brightness: *brightness,
            },
        ));
        setup.waiting_reflections = true;
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
    mut setup: ResMut<WorldSetup>,
) {
    use bevy::render::render_resource::*;
    let Some((entity, ReflectionProbeModel{ image, brightness })) = load_probe_q.iter().next() else {
        setup.waiting_reflections = false;
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

    setup.waiting_reflections = false;
}

pub(crate) fn setup_world_marker(
    mut commands: Commands,
) {
    commands.spawn((
        Name::new("World"),
        DespawnOnExit(ProgramState::InGame),
        WorldMarker::default(),
        Transform::IDENTITY,
        Visibility::Inherited,
    ));
}

fn start_next_level(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
) {
    commands.set_state(LevelState::Initializing);
    commands.set_state(OverlayState::Loading);
    commands.insert_resource(WorldSetup {
        waiting_skybox: true,
        waiting_reflections: false,
    });
    pause.set_menu_paused(true);
}

fn check_level_setup(
    mut commands: Commands,
    setup: Res<WorldSetup>,
    mut pause: ResMut<PauseState>,
) {
    // Done?
    if *setup == WorldSetup::default() {
        commands.set_state(OverlayState::Hidden);
        commands.set_state(GameplayState::Playing);
        commands.set_state(LevelState::Playing);
        pause.set_menu_paused(false);
    }
}

// pub(crate) fn spawn_level(
//     mut commands: Commands,
//     map_assets: Res<MapAssets>,
//     world: Single<Entity, With<WorldMarker>>,
//  ) {
//     commands.insert_resource(WorldSetup {
//         waiting_skybox: true,
//         waiting_reflections: false,
//     });

//     let level = commands.spawn((
//         SceneRoot(map_assets.level_test.clone()),
//     ))
//         .observe(|_ready: On<SceneInstanceReady>,
//             player_q: Query<&Transform, With<PlayerStart>>,
//             camera_q: Query<Entity, With<Camera3d>>,
//             mut commands: Commands| {
//                 if let Ok(xfrm) = player_q.single()
//                 && let Ok(camera) = camera_q.single()  {
//                     commands.entity(camera).insert(xfrm.clone());
//                 } else {
//                     log::error!("no PlayerStart");
//                 }

//                 commands.insert_resource(Spawning(true));
//                 commands.insert_resource(Shake(Vec3::ZERO));
//         }).id();

//     commands.entity(*world).add_child(level);
// }

pub(crate) fn despawn_world(
    world: Single<Entity, With<WorldMarker>>,
    child_q: Query<&Children>,
    mut commands: Commands,
) {
    for kid in child_q.iter_descendants(*world) {
        commands.entity(kid).try_despawn();
    }
}
