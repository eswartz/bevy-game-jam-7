use bevy::prelude::*;

/// State machine for overall program behavior.
#[derive(States, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
#[reflect(State, Default)]
#[type_path = "game"]
pub enum ProgramState {
    /// State before initial assets loaded.
    #[default]
    Initializing,
    /// State when starting fresh, assets loaded.
    New,
    /// Transitional state when re-loading.
    /// This is used to distinguish from New -> ... state transitions,
    /// which initialize content from scratch.
    LoadingSave,
    /// The main menu, shown to decide how to enter, or after exiting.
    LaunchMenu,
    /// This state means some aspect of the game is active,
    /// possibly paused, scripted, or behind a transient menu.
    InGame,
}

/// While the program state is in game,
/// these are the various modes the player can be in.
#[derive(SubStates, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
#[reflect(State, Default)]
#[source(ProgramState = ProgramState::InGame)]
#[type_path = "game"]
pub enum GameplayState {
    Inactive,
    /// Initial state when starting fresh.
    #[default]
    New,
    /// Transitional state when re-loading.
    /// This is used to distinguish from New -> ... transitions.
    LoadingSave,
    /// Loading the assets for the mode.
    AssetsLoading,
    /// Assets for the mode are loaded; continue to the appropriate state.
    AssetsLoaded,
    /// Game in progress.
    Playing,
}

/// This reflects the 2D overlay state.
#[derive(States, Default, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
#[reflect(State, Default)]
#[type_path = "game"]
pub enum OverlayState {
    /// No overlay.
    #[default]
    Hidden,
    /// Loading assets or levels.
    Loading,
    /// Main menu is up at startup.
    MainMenu,
    /// Escape Menu is up during gameplay.
    EscapeMenu,
    /// Game menu is up.
    GameMenu,
    /// Options menu is up.
    OptionsMenu,
    /// Audio menu is up.
    AudioMenu,
    /// Video menu is up.
    VideoMenu,
    /// Control menu is up.
    ControlsMenu,
    /// egui controls are up
    DebugGuiVisible,
}

impl OverlayState {
    pub fn is_hidden(&self) -> bool {
        *self == Self::Hidden
    }
    pub fn is_menu(&self) -> bool {
        *self == Self::MainMenu
        || *self == Self::GameMenu
        || *self == Self::OptionsMenu
        || *self == Self::AudioMenu
        || *self == Self::VideoMenu
        || *self == Self::ControlsMenu
        || *self == Self::EscapeMenu
    }
    pub fn is_debug(&self) -> bool {
        *self == Self::DebugGuiVisible
    }
}
