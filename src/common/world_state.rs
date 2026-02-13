use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use avian3d::prelude::*;

use bevy::core_pipeline::Skybox;
use image::imageops::FilterType;

use crate::assets::SkyboxAssets;
use crate::common::LevelState;
use crate::common::WorldCamera;

use super::states_sets::GameplayState;
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
            .insert_resource(Gravity((9.8 * Vec3::NEG_Y).into()))
            .init_resource::<SkyboxCache>()
            .add_systems(OnEnter(GameplayState::AssetsLoaded),
                (
                    transition_from_loading,
                    setup_world_marker,
                )
                // .in_set(SimulationSystems)
                .run_if(in_state(ProgramState::InGame))
            )
            .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
                (
                    despawn_world,
                )
                .chain()
            )
            // .add_systems(
            //     PreUpdate,
            //     insert_skybox
            //         .run_if(in_state(GameplayState::Setup))
            //         .run_if(in_state(ProgramState::InGame))
            // )

            .add_systems(OnEnter(LevelState::LoadingSkybox),
                (
                    start_skybox_setup
                ).chain()
                    .run_if(in_state(ProgramState::InGame))
            )

            .add_systems(
                PreUpdate,
                    (
                        check_load_skybox,
                        check_load_reflection_probe,
                        check_skybox_setup,
                    )
                    .chain()
                    .run_if(resource_exists::<SkyboxSetup>)
                    .run_if(in_state(ProgramState::InGame))
                    // .run_if(in_state(GameplayState::Setup))
                    .run_if(in_state(LevelState::LoadingSkybox))
            )
        ;
    }
}

#[derive(Component, Default, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[reflect(Default)]
#[type_path = "game"]
// #[number_key]
pub enum AreaContent {
    /// Air.
    #[default]
    Air = 0,
    /// Water.
    Water = 1,
}

impl AreaContent {
    pub fn in_liquid(&self) -> bool {
        match self {
            AreaContent::Air => false,
            AreaContent::Water => true,
        }
    }
}


/// Add this resource when creating a new level.
/// Removed when [LevelState::Setup] finishes.
#[derive(Resource, Debug, Default, Reflect, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub(crate) struct SkyboxSetup {
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

/// The AABB reflects the full extent of the "valid content" of the world.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct WorldMarkerEntity(pub Entity);

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

#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct SkyboxCache {
    /// Cache of width (narrow dimension) to cubemapped image.
    mapped_skyboxes: HashMap<(Handle<Image>, u32), Handle<Image>>,
}

impl SkyboxCache {
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

// pub(crate) fn insert_skybox(
//     mut commands: Commands,
//     cam_q: Query<Entity, (With<Camera3d>, Added<WorldCamera>)>,
//     skyboxes: Res<SkyboxAssets>,
// ) {
//     log::warn!("1");
//     if let Some(cam) = cam_q.iter().next() {
//         log::warn!("2");
//         let (brightness, skybox) = (100.0, skyboxes.star_map.clone());
//         // let (brightness, skybox, transform) = (500.0, skyboxes.driving_school.clone());
//         // let (brightness, skybox, transform) = (lux::CLEAR_SUNRISE, skyboxes.kloppenheim_sky_map.clone());
//         // let (brightness, skybox, transform) = (lux::CLEAR_SUNRISE, skyboxes.pure_sky.clone());
//         // let add_reflection_probe = Some(commands.spawn_empty().id());
//         let with_reflection_probe = Some((cam, 100.0));
//         // let with_reflection_probe = None;
//         commands.entity(cam).insert(SkyboxModel {
//             skybox: Skybox {
//                 image: skybox,
//                 brightness,
//                 ..default()
//             },
//             xfrm: SkyboxTransform::From1_0_2f_3f_4_5,
//             with_reflection_probe,
//             enabled: true, //state.show_skybox,
//         });
//     }
// }

/// This marker is created once and marks where game level content is swapped out.
pub(crate) fn setup_world_marker(
    mut commands: Commands,
    world_q: Query<&WorldMarker>,
) {
    if world_q.is_empty() {
        let ent = commands.spawn((
            Name::new("World"),
            DespawnOnExit(ProgramState::InGame),
            WorldMarker::default(),
            Transform::IDENTITY,
            Visibility::Inherited,
        )).id();
        commands.insert_resource(WorldMarkerEntity(ent));
    }
}

fn start_skybox_setup(
    mut commands: Commands,
) {
    commands.insert_resource(SkyboxSetup {
        waiting_skybox: true,
        waiting_reflections: false,
    });
}

fn check_skybox_setup(
    mut commands: Commands,
    setup: Res<SkyboxSetup>,
) {
    // Done?
    if *setup == SkyboxSetup::default() {
        commands.remove_resource::<SkyboxSetup>();
        commands.set_state(LevelState::Playing);
    }
}

pub(crate) fn despawn_world(
    world: Single<Entity, With<WorldMarker>>,
    child_q: Query<&Children>,
    mut commands: Commands,
) {
    for kid in child_q.iter_descendants(*world) {
        commands.entity(kid).try_despawn();
    }
}

/// Generic system to check for any LoadSkybox component, and if found,
/// make sure its image is loaded. Once loaded, convert it to a cubemap
/// and apply to the camera, then remove the component.
pub(crate) fn check_load_skybox(
    load_skybox_q: Query<(Entity, &SkyboxModel), Changed<SkyboxModel>>,
    mut commands: Commands,
    video_settings: Res<VideoSettings>,
    mut images: ResMut<Assets<Image>>,
    mut skyboxes: ResMut<SkyboxCache>,
    mut setup: ResMut<SkyboxSetup>,
) {
    // use bevy::render::render_resource::*;
    let Some((cam, SkyboxModel{ skybox, xfrm, with_reflection_probe, enabled })) = load_skybox_q.iter().next() else {
        setup.waiting_skybox = false;
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
    world: Res<WorldMarkerEntity>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut setup: ResMut<SkyboxSetup>,
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
        ChildOf(world.0),
    ));

    setup.waiting_reflections = false;
}
