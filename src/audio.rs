use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_seedling::prelude::*;

use crate::states_sets::GameplayState;
use crate::states_sets::LevelState;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(LevelState::Playing),
                init_background_audio
            )
            .add_systems(PostUpdate,
                apply_volumes
            )
        ;
    }
}

/// This drives the volume from the user config point of view.
#[derive(Component)]
pub(crate) struct UserVolume {
    pub volume: Volume,
    pub muted: bool,
}

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct Sfx;

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct UiSfx;

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct Music;


#[derive(Resource, AssetCollection)]
pub struct MusicAssets {
    #[asset(path = "music/song0.ogg")]
    pub song0: Handle<AudioSample>,
}


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
                panning_threshold: 1.0,
                ..default()
            },
            SpatialScale(Vec3::splat(2.0))
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


/// Add background music, which resets when the game starts or stops.
pub(crate) fn init_background_audio(
    mut commands: Commands,
    music: Res<MusicAssets>,
) {
    commands.spawn((
        DespawnOnExit(GameplayState::Playing),
        Music,
        SamplePlayer::new(music.song0.clone()).looping(),
    ));
}

/// Apply mute-able UserVolume to VolumeNodes.
pub(crate) fn apply_volumes(
    mut commands: Commands,
    vol_q: Query<(Entity, &UserVolume), Changed<UserVolume>>,
) {
    for (ent, vol) in vol_q.iter() {
        commands.entity(ent).insert(VolumeNode {
            volume: if vol.muted { Volume::SILENT } else { vol.volume },
            ..default()
        });
    }
}
