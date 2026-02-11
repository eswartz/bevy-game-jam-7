use bevy::prelude::*;

use crate::common::OverlayState;
use crate::common::PauseState;
use crate::common::ProgramState;

/// Use as a condition to test whether any field in PauseState is set.
pub fn is_paused(paused: Res<PauseState>) -> bool {
    paused.is_paused()
}
/// Use as a condition to test whether the user pause state is set.
pub fn is_user_paused(paused: Res<PauseState>) -> bool {
    paused.is_user_paused()
}
/// Use as a condition to test whether the menu pause state is set.
pub fn is_menu_paused(paused: Res<PauseState>) -> bool {
    paused.is_menu_paused()
}

pub fn is_game_active(program_state: Res<State<ProgramState>>) -> bool {
    *program_state.get() == ProgramState::InGame
}

pub fn is_in_menu(overlay: Res<State<OverlayState>>) -> bool {
    overlay.is_menu()
}
