use bevy::prelude::*;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState as _;
use bevy_asset_loader::loading_state::config::LoadingStateConfig;
use bevy_seedling::prelude::*;
use bevy_seedling::sample::PlaybackSettings;
use leafwing_input_manager::prelude::ActionState;
use rand::RngExt;

use crate::assets::FxAssets;
use crate::assets::MusicAssets;
use crate::common::*;
use crate::game::LevelRoot;
use crate::game::MusicTrack;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(AudioCommonPlugin)
            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<MusicAssets>()
                    .load_collection::<FxAssets>()
            )

            .add_systems(OnEnter(LevelState::Playing),
                (
                    init_background_audio,
                )
            )
            .add_systems(Update,
                (
                    spawn_menu_fx,
                    handle_user_actions,
                )
            )
        ;
    }
}

/// Add background music, which resets when the game starts or stops.
pub(crate) fn init_background_audio(
    mut commands: Commands,
    music: Res<MusicAssets>,
    track_q: Query<&MusicTrack, With<LevelRoot>>,
    world_q: Res<WorldMarkerEntity>,
) {
    if let Ok(track) = track_q.single() {
        let mut rng = rand::rng();
        let sample = music.get_for(track).clone();
        commands.spawn((
            ChildOf(world_q.0),
            DespawnOnExit(GameplayState::Playing),
            Music,
            SamplePlayer::new(sample).looping(),
            PlaybackSettings {
                play_from: PlayFrom::Seconds(rng.random_range(0.0 .. 5.0 * 60.0)),
                ..default()
            },
        ));
    } else {
        log::warn!("no MusicTrackSelection");
    }
}

fn spawn_menu_fx(mut commands: Commands,
    fx: Option<Res<FxAssets>>,
    mut reader: MessageReader<MenuActionMessage>,
) {
    if reader.is_empty() {
        return
    }
    let Some(fx) = fx else { return };

    let any = reader.read().any(is_menu_action_click_bait);

    if any {
        commands.spawn((
            UiSfx,
            SamplePlayer::new(fx.action.clone()),
        ));
    }
}

fn handle_user_actions(mut commands: Commands,
    action_state: Res<ActionState<UserAction>>,
    fx: Option<Res<FxAssets>>,
    mut reader: MessageReader<MenuActionMessage>,
) {
    if action_state.just_pressed(&UserAction::SwitchNextAudioTrack) {
    }

    if reader.is_empty() {
        return
    }
    let Some(fx) = fx else { return };

    let any = reader.read().any(is_menu_action_click_bait);

    if any {
        commands.spawn((
            UiSfx,
            SamplePlayer::new(fx.action.clone()),
        ));
    }
}

/// Play a click sound on menu action?
fn is_menu_action_click_bait(event: &MenuActionMessage) -> bool {
    match event {
        MenuActionMessage::Activate(_) => false,
        MenuActionMessage::Navigate(_) |
        // MenuActionMessage::Activate(_) |
        MenuActionMessage::Next(_) |
        MenuActionMessage::Reset(_) | MenuActionMessage::Previous(_) => true,
        MenuActionMessage::Slide(..) => false,
    }
}
