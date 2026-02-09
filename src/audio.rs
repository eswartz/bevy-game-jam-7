use bevy::prelude::*;
use bevy_seedling::prelude::*;

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct Sfx;

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct UiSfx;

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct Music;

pub(crate) fn initialize_audio(mut master: Single<&mut VolumeNode, With<MainBus>>, mut commands: Commands) {
    master.volume = Volume::Linear(0.5);

    const DEFAULT_POOL_VOLUME: Volume = Volume::Linear(1.0);

    // For each new pool, we can provide non-default initial values for the volume.
    commands.spawn((
        Name::new("Music"),
        SamplerPool(Music),
        VolumeNode {
            volume: DEFAULT_POOL_VOLUME,
            ..default()
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
        VolumeNode {
            volume: DEFAULT_POOL_VOLUME,
            ..default()
        },
    ));
    commands.spawn((
        Name::new("UI"),
        SamplerPool(UiSfx),
        VolumeNode {
            volume: DEFAULT_POOL_VOLUME,
            ..default()
        },
    ));
}
