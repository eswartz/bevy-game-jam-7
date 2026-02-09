use leafwing_input_manager::action_diff::ActionDiffMessage;
use leafwing_input_manager::prelude::*;
use bevy::prelude::*;
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
        app
        .add_plugins(InputManagerPlugin::<UserAction>::default())
        .register_type::<UserAction>()
        .add_message::<ActionDiffMessage::<UserAction>>()

        .insert_resource(default_input_map())
        .init_resource::<ActionState<UserAction>>()

        // Note: this only helps with Buttonlike actions. [Dual]Axis actions are not considered.
        .insert_resource(ClashStrategy::PrioritizeLongest)
        ;
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, EnumIter)]
#[type_path = "game"]
pub enum UserAction {
    TogglePause,
    ToggleMute,

    ToggleMenu,
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
        .with_dual_axis(
            MoveFlycam,
            VirtualDPad::wasd()
               .inverted_y()
        )
        .with_axis(
            MoveDownUp,
            VirtualAxis::new(
                KeyCode::KeyC,
                KeyCode::Space,
            )
        )
        .with_axis(
            MoveLeftRight2d,
            VirtualAxis::new(
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
            )
        )
        .with_axis(
            MoveDownUp2d,
            VirtualAxis::new(
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
            )
        )
        .with_dual_axis(
            Look,
            MouseMove::default(),
        )
        .with_axis(
            Tilt,
            VirtualAxis::new(
                KeyCode::BracketRight,
                KeyCode::BracketLeft,
            )
        )
    ;

    // Lazy finger movement falsely triggers these, which is very annoying.
    if cfg!(target_os = "macos") {
        const MOD: ModifierKey = ModifierKey::Alt;
        input_map.insert_axis(Zoom, VirtualAxis::new(
            ButtonlikeChord::modified(MOD, MouseScrollDirection::UP),
            ButtonlikeChord::modified(MOD, MouseScrollDirection::DOWN)));
        input_map.insert_axis(Tilt, VirtualAxis::new(
            ButtonlikeChord::modified(MOD, MouseScrollDirection::LEFT),
            ButtonlikeChord::modified(MOD, MouseScrollDirection::RIGHT)));

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
    input_map.insert(ShiftFire, ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Left));

    input_map.insert(AlternateFire, MouseButton::Right);
    input_map.insert(ShiftAlternateFire, ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Right));

    input_map.insert(Interact, KeyCode::KeyE);

    input_map.insert(TogglePause, KeyCode::Pause);
    input_map.insert(TogglePause, ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyP])); // "P"ause
    input_map.insert(ToggleMute, KeyCode::F12);
    input_map.insert(ToggleMute, ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyM])); // "M"ute
    input_map.insert(ToggleFullScreen, KeyCode::F11);

    input_map.insert(ToggleDebugUi, KeyCode::Backquote);
    input_map.insert(ToggleMenu, KeyCode::Escape);

    input_map.insert(ToggleHelp, KeyCode::F1);
    input_map.insert(ToggleInspector, ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyI]));
    input_map.insert(ToggleFps, ButtonlikeChord::modified(MOD_CTRL_COMMAND, KeyCode::KeyG));    // "G"raph
    input_map.insert(ToggleSkybox, ButtonlikeChord::modified(MOD_CTRL_COMMAND, KeyCode::KeyB));    // "B"ackground

    input_map
}
//
