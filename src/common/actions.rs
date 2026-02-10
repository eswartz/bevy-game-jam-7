use crate::common::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::WindowMode;
use bevy_seedling::prelude::MainBus;
use leafwing_input_manager::action_diff::ActionDiffMessage;
use leafwing_input_manager::prelude::*;
use strum_macros::EnumIter;

pub(crate) const CTRL_COMMAND: KeyCode = if cfg!(target_os = "macos") {
    KeyCode::SuperLeft
} else {
    KeyCode::ControlLeft
};

pub(crate) const MOD_CTRL_COMMAND: ModifierKey = if cfg!(target_os = "macos") {
    ModifierKey::Super
} else {
    ModifierKey::Control
};

pub struct ActionPlugin;
impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<UserAction>::default())
            .register_type::<UserAction>()
            .add_message::<ActionDiffMessage<UserAction>>()
            .insert_resource(default_input_map())
            .init_resource::<ActionState<UserAction>>()
            // Note: this only helps with Buttonlike actions. [Dual]Axis actions are not considered.
            .insert_resource(ClashStrategy::PrioritizeLongest)
            .add_systems(
                Update,
                (process_global_actions, handle_escape).run_if(in_state(ProgramState::InGame)),
            );
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, EnumIter)]
#[type_path = "game"]
pub enum UserAction {
    TogglePause,
    ToggleMute,

    SwitchNextAudioTrack,
    SwitchPrevAudioTrack,

    // ToggleMenu,
    ToggleDebugUi,
    ToggleHelp,
    ToggleInspector,

    ToggleFps,
    ToggleSkybox,
    ToggleFullScreen,

    // MarkSolved,

    // SaveState,
    // LoadState,
    // DumpState,
    /// Move relative to the camera rotation (flycam).
    #[actionlike(DualAxis)]
    MoveFlycam,
    /// Move up/down from camera rotation.
    #[actionlike(Axis)]
    MoveDownUp,

    /// UI editing.
    #[actionlike(Axis)]
    MoveLeftRight2d,
    #[actionlike(Axis)]
    MoveDownUp2d,

    /// All-purpose "fire" (e.g. left-click)
    Fire,
    /// Shift+Fire.
    ShiftFire,
    /// Alt-Fire (e.g. right-click)
    AlternateFire,
    /// Shift+Alt-Fire (e.g. right-click)
    ShiftAlternateFire,

    /// All-purpose "action".
    Interact,

    /// Tilt/roll the camera on Z axis.
    #[actionlike(Axis)]
    Tilt,
    /// Turn the camera up/down on X and left/right on Y axes.
    #[actionlike(DualAxis)]
    Look,
    /// Reset orientation to identity.
    Home,

    /// Get closer/further from active object.
    #[actionlike(Axis)]
    Zoom,

    /// Turn around 180 degrees around Y axis.
    TurnAround,

    /// When held, move faster (i.e. Shift).
    Accelerate,

    /// When held, lower camera and move slower (i.e. Ctrl).
    ToggleCrouch,
    /// When held, lower camera and move slower (i.e. Ctrl).
    Crouch,
}

pub fn default_input_map() -> InputMap<UserAction> {
    use UserAction::*;

    let mut input_map = InputMap::default()
        .with_dual_axis(MoveFlycam, VirtualDPad::wasd().inverted_y())
        .with_axis(MoveDownUp, VirtualAxis::new(KeyCode::KeyC, KeyCode::Space))
        .with_axis(
            MoveLeftRight2d,
            VirtualAxis::new(KeyCode::ArrowLeft, KeyCode::ArrowRight),
        )
        .with_axis(
            MoveDownUp2d,
            VirtualAxis::new(KeyCode::ArrowDown, KeyCode::ArrowUp),
        )
        .with_dual_axis(Look, MouseMove::default())
        .with_axis(
            Tilt,
            VirtualAxis::new(KeyCode::BracketRight, KeyCode::BracketLeft),
        );

    // Lazy finger movement falsely triggers these, which is very annoying.
    if cfg!(target_os = "macos") {
        const MOD: ModifierKey = ModifierKey::Alt;
        input_map.insert_axis(
            Zoom,
            VirtualAxis::new(
                ButtonlikeChord::modified(MOD, MouseScrollDirection::UP),
                ButtonlikeChord::modified(MOD, MouseScrollDirection::DOWN),
            ),
        );
        input_map.insert_axis(
            Tilt,
            VirtualAxis::new(
                ButtonlikeChord::modified(MOD, MouseScrollDirection::LEFT),
                ButtonlikeChord::modified(MOD, MouseScrollDirection::RIGHT),
            ),
        );
    } else {
        input_map.insert_axis(Zoom, MouseScrollAxis::Y);
        input_map.insert_axis(Tilt, MouseScrollAxis::X);
    }

    input_map.insert(Accelerate, ModifierKey::Shift);
    input_map.insert(ToggleCrouch, ModifierKey::Control);
    input_map.insert(Crouch, KeyCode::KeyC);
    // input_map.insert(SnapRotation, ButtonlikeChord::modified(ModifierKey::Control, KeyCode::Backslash));
    input_map.insert(TurnAround, KeyCode::Backspace);
    input_map.insert(Home, KeyCode::Backslash);

    input_map.insert(Fire, MouseButton::Left);
    input_map.insert(
        ShiftFire,
        ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Left),
    );

    input_map.insert(AlternateFire, MouseButton::Right);
    input_map.insert(
        ShiftAlternateFire,
        ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Right),
    );

    input_map.insert(Interact, KeyCode::KeyE);

    input_map.insert(TogglePause, KeyCode::Pause);
    input_map.insert(
        TogglePause,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyP]),
    ); // "P"ause
    input_map.insert(ToggleMute, KeyCode::F12);
    input_map.insert(
        ToggleMute,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyM]),
    ); // "M"ute
    input_map.insert(ToggleFullScreen, KeyCode::F11);

    input_map.insert(ToggleDebugUi, KeyCode::Backquote);

    input_map.insert(ToggleHelp, KeyCode::F1);
    input_map.insert(
        ToggleInspector,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyI]),
    );
    input_map.insert(
        ToggleFps,
        ButtonlikeChord::modified(MOD_CTRL_COMMAND, KeyCode::KeyG),
    ); // "G"raph
    input_map.insert(
        ToggleSkybox,
        ButtonlikeChord::modified(MOD_CTRL_COMMAND, KeyCode::KeyB),
    ); // "B"ackground

    input_map.insert(
        SwitchNextAudioTrack,
        ButtonlikeChord::modified(MOD_CTRL_COMMAND, KeyCode::MediaTrackNext),
    );
    input_map.insert(
        SwitchPrevAudioTrack,
        ButtonlikeChord::modified(MOD_CTRL_COMMAND, KeyCode::MediaTrackPrevious),
    );

    input_map
}

/// Handle Escape, which is handled differently outside and inside menus.
fn handle_escape(
    mut commands: Commands,
    overlay_state: Res<State<OverlayState>>,
    going_back: Option<Res<GoBackInMenuRequest>>,
    mut previous_menu: ResMut<PreviousMenuStack>,
    mut reader: MessageReader<KeyboardInput>,
) {
    // Menu logic handles this itself.
    if overlay_state.is_menu() {
        return;
    }
    if going_back.is_some() {
        return;
    }

    for key_event in reader.read() {
        if key_event.state == ButtonState::Pressed && key_event.key_code == KeyCode::Escape {
            // If we reach the root, handle it here.
            match overlay_state.get() {
                OverlayState::Hidden => {
                    // The one case where Escape *opens* the menu the first time.
                    previous_menu.0.clear();
                    // commands.insert_resource(GoBackInMenuRequest);
                    commands.set_state(OverlayState::EscapeMenu)
                }
                OverlayState::EscapeMenu => {
                    // previous_menu.0.clear();
                    commands.insert_resource(GoBackInMenuRequest);
                    // commands.set_state(OverlayState::Hidden)
                }
                OverlayState::MainMenu => {
                    // Ignore, since we don't leave exit via Quit (TODO: can this quit?)
                }
                OverlayState::DebugGuiVisible => commands.set_state(OverlayState::Hidden),
                _ => (),
            }
        }
    }
}

/// Process actions, sampling actions globally.
///
/// Clients handle sub-UserActions on their own
/// in similar systems. Multiple clients independently
/// see the UserActions and can respond appropriately.
fn process_global_actions(
    mut commands: Commands,
    action_state: Res<ActionState<UserAction>>,
    overlay_state: Res<State<OverlayState>>,
    // previous_menu: ResMut<PreviousMenu>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut pause_state: ResMut<PauseState>,
    mut vol_q: Single<&mut UserVolume, With<MainBus>>,
) {
    if action_state.just_pressed(&UserAction::TogglePause) {
        let paused = !pause_state.is_user_paused();
        pause_state.set_user_paused(paused);
    }
    if action_state.just_pressed(&UserAction::ToggleDebugUi) {
        commands.set_state(match overlay_state.get() {
            OverlayState::Hidden => OverlayState::DebugGuiVisible,
            OverlayState::DebugGuiVisible => OverlayState::Hidden,
            current => *current,
        });
    }
    if action_state.just_pressed(&UserAction::ToggleFullScreen)
        && let Ok(mut window) = primary_window.single_mut()
    {
        let cur_mode = window.mode;
        window.mode = match cur_mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            WindowMode::BorderlessFullscreen(_monitor_selection) => WindowMode::Windowed,

            // WindowMode::BorderlessFullscreen(monitor_selection) => WindowMode::Fullscreen(
            //     monitor_selection, VideoModeSelection::Current),
            WindowMode::Fullscreen(_monitor_selection, _video_mode_selection) => {
                WindowMode::Windowed
            }
        };
    }
    if action_state.just_pressed(&UserAction::ToggleMute) {
        vol_q.muted = !vol_q.muted;
    }

    // other [UserAction]::s handled separately.
}
