
use bevy::camera::ScreenSpaceTransmissionQuality;
use bevy::prelude::*;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::light_consts::lux;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;
use bevy::post_process::bloom::Bloom;

use bevy::render::view::ColorGrading;
use bevy::render::view::ColorGradingGlobal;
use bevy::render::view::ColorGradingSection;

use super::states_sets::GameplayState;
use super::states_sets::ProgramState;
use super::video::Antialiasing;
use super::video::GlassQuality;
use super::video::VideoCameraSettingsChanged;
use super::video::VideoEffectSettingsChanged;
use super::video::VideoSettings;
use super::world_state::SkyboxAssets;
use super::world_state::SkyboxModel;

pub struct WorldUiPlugin;

impl Plugin for WorldUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<SkyboxModel>()
            .add_systems(OnEnter(GameplayState::AssetsLoaded),
                (
                    init_sandbox_skybox,
                )
                // .in_set(SimulationSystems)
                .run_if(in_state(ProgramState::InGame))
            )
            .add_systems(PreUpdate,
                (
                    apply_effect_settings,
                    apply_camera_settings,
                )
                // .in_set(SimulationSystems)
            )
        ;
    }
}

pub fn apply_camera_settings(
    trigger: Option<Res<VideoCameraSettingsChanged>>,
    mut commands: Commands,
    mut camera_q: Query<&mut Projection, With<Camera3d>>,
    video_settings: Res<VideoSettings>,
) {
    if trigger.is_none() {
        return;
    }

    let Ok(mut proj) = camera_q.single_mut() else {
        return
    };

    if let Projection::Perspective(proj) = &mut *proj {
        proj.fov = video_settings.fov_degrees.to_radians();
    }

    commands.remove_resource::<VideoCameraSettingsChanged>();
}

pub fn apply_effect_settings(
    trigger: Option<Res<VideoEffectSettingsChanged>>,
    mut commands: Commands,
    mut camera_q: Query<(Entity, &mut Camera3d)>, //, With<OurCamera>>,
    video_settings: Res<VideoSettings>,
) {
    if trigger.is_none() {
        return;
    }

    let Ok((camera_ent, mut cam3d)) = camera_q.single_mut() else {
        return
    };

    info!("Setting up effects");
    let mut ent_commands = commands.entity(camera_ent);
    ent_commands.remove::<Msaa>();
    ent_commands.remove::<ScreenSpaceAmbientOcclusion>();
    ent_commands.remove::<TemporalAntiAliasing>();
    ent_commands.remove::<Bloom>();

    ent_commands.insert((
        // Tonemapping::BlenderFilmic,
        Tonemapping::TonyMcMapface,
        // Kinda ugly and contrasty
        Bloom {
            intensity: -1.0,
            low_frequency_boost: 1.0,
            low_frequency_boost_curvature: 0.0,
            high_pass_frequency: 1.0,
            ..default()
        },
        // Bloom::NATURAL,
        ColorGrading {
            global: ColorGradingGlobal {
                // exposure: 1.25,
                exposure: 1.0,
                post_saturation: 1.5,
                ..default()
            },
            shadows: ColorGradingSection {
                lift: -0.005,
                ..default()
            },
            midtones: ColorGradingSection::default(),
            highlights: ColorGradingSection {
                lift: -0.005,
                ..default()
            }
        },
    ));

    match video_settings.antialiasing {
        Antialiasing::Off => {
            ent_commands.remove::<(ScreenSpaceAmbientOcclusion, TemporalAntiAliasing)>();

            ent_commands.insert((
                Msaa::Off,
            ));
        },
        Antialiasing::TSAA => {
            ent_commands.insert((
                Msaa::Off,
                ScreenSpaceAmbientOcclusion {
                    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                    ..default()
                },
                TemporalAntiAliasing::default(),
            ));
        }
        // Antialiasing::MSAA => {
        //     ent_commands.remove::<(Msaa, ScreenSpaceAmbientOcclusion, TemporalAntiAliasing)>();
        //     // ent_commands.insert(Msaa::Sample4);
        // }
    }

    match video_settings.glass_quality {
        GlassQuality::Off => {
            cam3d.screen_space_specular_transmission_steps = 0;
            cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Low;
        }
        GlassQuality::Low => {
            cam3d.screen_space_specular_transmission_steps = 1;
            cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Low;
        }
        GlassQuality::Medium => {
            cam3d.screen_space_specular_transmission_steps = 1;
            cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Medium;
        }
        GlassQuality::High => {
            cam3d.screen_space_specular_transmission_steps = 2;
            cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::High;
        }
        GlassQuality::Ultra => {
            cam3d.screen_space_specular_transmission_steps = 3;
            cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Ultra;
        }
    }

    commands.remove_resource::<VideoEffectSettingsChanged>();
}

pub(crate) fn init_sandbox_skybox(
    mut commands: Commands,
    cam_q: Query<Entity, With<Camera3d>>,
    // parent_q: Query<&ChildOf>,
    skyboxes: Res<SkyboxAssets>,
    // state: Res<GuiState>,
    // assets: Res<AssetServer>,
) {
    if let Ok(cam) = cam_q.single() {
        // let (brightness, skybox, transform) = (100.0, skyboxes.star_map.clone(), SkyboxAssets::STAR_MAP_TRANSFORM);
        // let (brightness, skybox, transform) = (500.0, skyboxes.driving_school.clone(), SkyboxAssets::DRIVING_SCHOOL_TRANSFORM);
        // let (brightness, skybox, transform) = (lux::CLEAR_SUNRISE, skyboxes.kloppenheim_sky_map.clone(), SkyboxAssets::PURE_SKY_TRANSFORM);
        let (brightness, skybox, transform) = (lux::CLEAR_SUNRISE, skyboxes.pure_sky.clone(), SkyboxAssets::PURE_SKY_TRANSFORM);
        // let add_reflection_probe = Some(commands.spawn_empty().id());
        let with_reflection_probe = Some((cam, 100.0));
        // let with_reflection_probe = None;
        commands.entity(cam).insert(SkyboxModel {
            skybox: Skybox {
                image: skybox,
                brightness,
                ..default()
            },
            xfrm: transform,
            with_reflection_probe,
            enabled: true, //state.show_skybox,
        });
        // commands.entity(cam).insert(LoadReflectionProbe(
        //     // assets.load("textures/graffiti_shelter_4k.ktx2"),
        // ));
    }
}
