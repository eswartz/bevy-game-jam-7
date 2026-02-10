use bevy::prelude::*;

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
        matches!(self,
            Self::MainMenu
            | Self::GameMenu
            | Self::OptionsMenu
            | Self::AudioMenu
            | Self::VideoMenu
            | Self::ControlsMenu
            | Self::EscapeMenu
        )
    }
    pub fn is_debug(&self) -> bool {
        *self == Self::DebugGuiVisible
    }
    pub fn is_escape(&self) -> bool {
        *self == Self::EscapeMenu
    }
}


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
    /// The main menu, shown to decide how to enter the game, and shown after exiting the game.
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
    /// Asset loading is complete, now running systems to do final touches.
    Setup,
    /// Game in progress.
    Playing,
}


/// State of a level (there is only one level in play at a time).
#[derive(SubStates, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[states(scoped_entities)]
#[source(ProgramState = ProgramState::InGame)]
#[reflect(Default)]
#[type_path = "game"]
pub enum LevelState {
    /// Default state
    #[default]
    Initializing,
    /// Ready to play.
    Playing,
    /// Switching levels.
    Advance,
}
