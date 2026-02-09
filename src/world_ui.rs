
use std::time::Duration;

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
use rand::seq::IndexedRandom;
use rand::Rng as _;

use crate::states_sets::GameplayState;
use crate::states_sets::ProgramState;
use crate::video::Antialiasing;
use crate::video::GlassQuality;
use crate::video::VideoCameraSettingsChanged;
use crate::video::VideoEffectSettingsChanged;
use crate::video::VideoSettings;
use crate::world_state::SkyboxAssets;
use crate::world_state::SkyboxModel;
use crate::world_state::WorldMarker;

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
            .add_systems(OnEnter(GameplayState::Playing),
                init_background_audio
                // .in_set(SimulationSystems)
            )
            // // FIXME: nonsymmetric logic here
            // .add_systems(OnExit(LevelState::Advance),
            //     init_background_audio
            //     // .in_set(SimulationSystems)
            // )
            .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
                (
                    cancel_audio,
                )
                .chain()
                // .in_set(SimulationSystems)
            )
            .add_systems(OnEnter(ProgramState::LoadingSave),
                cancel_audio
                .chain()
                // .in_set(SimulationSystems)
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
        Tonemapping::BlenderFilmic,
        // Kinda ugly and contrasty
        // Bloom {
        //     intensity: -1.0,
        //     low_frequency_boost: 1.0,
        //     low_frequency_boost_curvature: 0.0,
        //     high_pass_frequency: 1.0,
        //     ..default()
        // },
        Bloom::NATURAL,
        ColorGrading {
            global: ColorGradingGlobal {
                // exposure: 1.25,
                exposure: 1.0,
                post_saturation: 1.25,
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

pub(crate) fn init_background_audio(
    mut commands: Commands,
    world_q: Single<Entity, With<WorldMarker>>,
    // bg_spawner_q: Query<Entity, With<BackgroundAudioSpawner>>,
    // bg_q: Query<Entity, With<BackgroundAudio>>,
    // music: Res<MusicAssets>,
    // voices: Res<VoiceAssets>,
) {
    // // Remove current background audio.
    // for ent in bg_spawner_q.iter() {
    //     commands.entity(ent).try_despawn();
    // }
    // for ent in bg_q.iter() {
    //     commands.entity(ent).try_despawn();
    // }

    // // Rewrite.
    // add_background_voice_spawners(commands.reborrow(), *world_q, &voices);
    // add_background_music_spawners(commands.reborrow(), *world_q, &music);
}

// fn add_background_music_spawners(
//     mut commands: Commands,
//     world: Entity,
//     music: &MusicAssets,
// ) {
//     let songs = music.all_songs();
//     let clips = music.all_clips();
//     let mut rng = rand::rng();

//     commands.spawn((
//         ChildOf(world),
//         Name::new("Background Audio"),
//         Saveable,
//         BackgroundAudio,
//         AudioCue::new(songs.choose(&mut rng).unwrap().clone())
//             .with_loop_start(Some(0.0))
//             .with_volume(bevy::audio::Volume::Decibels(-12.0))
//             .with_channel(AudioVirtualChannel::Music),
//     ));
//     for _ in 0..3 {
//         commands.spawn((
//             ChildOf(world),
//             Name::new("Background Audio"),
//             Saveable,
//             Transform::from_xyz(
//                 rng.random_range(-50f32..50.),
//                 rng.random_range(-20f32..20.),
//                 rng.random_range(-50f32..50.)),

//             BackgroundAudioSpawner::new(
//                 AudioVirtualChannel::Ambient,
//                 clips.clone(),
//                 Duration::from_secs(10)..Duration::from_secs(30),
//                 Vec3::splat(50.0),
//                 -48f32..-24f32,
//                 4,
//             ),
//                         // Transform::from_xyz(
//             //     rng.random_range(-100f32..100.),
//             //     rng.random_range(-20f32..20.),
//             //     rng.random_range(-100f32..100.)),
//             //     AudioCue::new(clips.choose(&mut rng).unwrap().clone())
//             //         .with_loop_start(Some(-5.0))
//             //         .with_loop_end(Some(rng.random_range(13.0..41.0)))
//             //         .with_spatial_radius(10.0)
//             //         .with_volume(bevy::audio::Volume::Decibels(rng.random_range(-48.0..-24.0)))
//             //         .with_channel(AudioVirtualChannel::Ambient),

//         ));
//     }
// }

// fn add_background_voice_spawners(
//     mut commands: Commands,
//     world: Entity,
//     voices: &VoiceAssets,
// ) {
//     let nonsense = voices.all_nonsense_clips();
//     let mut rng = rand::rng();
//     for _ in 0..10 {
//         commands.spawn((
//             ChildOf(world),
//             Name::new("Background Voice"),
//             Transform::from_xyz(
//                 rng.random_range(-50f32..50.),
//                 rng.random_range(-20f32..20.),
//                 rng.random_range(-50f32..50.)),

//             BackgroundAudioSpawner::new(
//                 AudioVirtualChannel::Ambient,
//                 nonsense.clone(),
//                 Duration::from_secs(1)..Duration::from_secs(5),
//                 Vec3::new(10.0, 5.0, 10.0),
//                 -36f32..-12f32,
//                 4,
//             ),
//         ));
//     }
// }

pub(crate) fn cancel_audio(
    mut commands: Commands,
    // mut audio: Audio,
    // bg_q: Query<Entity, With<BackgroundAudio>>,
) {
    // audio.stop_all();

    // for ent in bg_q.iter() {
    //     commands.entity(ent).try_despawn();
    // }
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
