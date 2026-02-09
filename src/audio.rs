use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::asset_collection::AssetCollectionWorld as _;
use bevy_seedling::prelude::*;

use crate::menus_common::MenuActionMessage;
use crate::states_sets::GameplayState;
use crate::states_sets::LevelState;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup,
                preload_audio
            )
            .add_systems(OnEnter(LevelState::Playing),
                init_background_audio
            )
            .add_systems(PostUpdate,
                apply_volumes
            )
            .add_systems(Update,
                spawn_menu_sfx
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


#[derive(Resource, AssetCollection)]
pub struct FxAssets {
    #[asset(path = "sounds/164472__deleted_user_2104797__crack-of-branch-3.ogg")]
    pub action: Handle<AudioSample>,
    #[asset(path = "sounds/164472__deleted_user_2104797__crack-of-branch-3-rev.ogg")]
    pub back: Handle<AudioSample>,
}

pub(crate) fn preload_audio(world: &mut World) {
    world.init_collection::<FxAssets>();
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

fn spawn_menu_sfx(mut commands: Commands,
    fx: Res<FxAssets>,
    mut reader: MessageReader<MenuActionMessage>
) {
    if reader.is_empty() {
        return
    }

    let mut was_back = false;
    for event in reader.read() {
        match event {
            MenuActionMessage::Navigate(_) |
            MenuActionMessage::Activate(_) |
            MenuActionMessage::Next(_) => was_back = false,
            MenuActionMessage::Reset(_) | MenuActionMessage::Previous(_) => {
                was_back = true;
            }
            MenuActionMessage::Slide(..) => return,
        }
    }

    // Limit to one per frame.
    commands.spawn((
        UiSfx,
        SamplePlayer::new(if was_back {
            fx.back.clone()
        } else {
            fx.action.clone()
        }),
    ));
}
