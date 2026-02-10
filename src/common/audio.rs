use bevy::prelude::*;
use bevy_seedling::prelude::*;

pub struct AudioCommonPlugin;

impl Plugin for AudioCommonPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(SeedlingPlugin::default())
            .add_systems(Startup, initialize_audio)

            // .add_systems(Startup,
            //     preload_audio
            // )
            // .add_systems(OnEnter(LevelState::Playing),
            //     init_background_audio
            // )
            .add_systems(PostUpdate,
                apply_volumes
            )
            // .add_systems(Update,
            //     spawn_menu_sfx
            // )
        ;
    }
}

/// This drives the volume from the user config point of view.
///
/// Our [apply_volumes] system ensures that a corresponding VolumeNode matches
/// the volume and muted state.
#[derive(Component)]
#[require(VolumeNode{ volume: Volume::SILENT, ..default() })]
pub(crate) struct UserVolume {
    pub volume: Volume,
    pub muted: bool,
}

/// Pool for in-game diegetic sound effects with spatial listening.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct Sfx;

/// Pool for UI sound effects (menus, etc), not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct UiSfx;

/// Pool for the music, not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct Music;

pub(crate) fn initialize_audio(master: Single<Entity, With<MainBus>>, mut commands: Commands) {
    commands.entity(*master).insert(UserVolume {
        volume: Volume::Linear(0.5),
        muted: false,
    });

    const DEFAULT_POOL_VOLUME: Volume = Volume::Linear(1.0);

    // For each new pool, we can provide non-default initial values for the volume.
    commands.spawn((
        Name::new("Music"),
        SamplerPool(Music),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
    ));
    commands.spawn((
        Name::new("SFX"),
        SamplerPool(Sfx),
        sample_effects![(
            SpatialBasicNode {
                panning_threshold: 0.9,
                ..default()
            },
            SpatialScale(Vec3::splat(5.0))
        )],
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
    ));
    commands.spawn((
        Name::new("UI"),
        SamplerPool(UiSfx),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
    ));
}

/// Apply mute-able UserVolume to VolumeNodes.
pub(crate) fn apply_volumes(
    mut vol_q: Query<(Entity, &UserVolume, &mut VolumeNode), Changed<UserVolume>>,
) {
    for (_ent, user, mut vol) in vol_q.iter_mut() {
        vol.volume = if user.muted { Volume::SILENT } else { user.volume };
    }
}
