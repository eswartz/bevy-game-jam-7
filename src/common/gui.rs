use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy::window::CursorOptions;
use bevy::window::PrimaryWindow;
use bevy::window::WindowFocused;
use bevy_asset_loader::prelude::*;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_seedling::prelude::MainBus;

use crate::common::RENDER_LAYER_UI;

use super::audio::UserVolume;
use super::lifecycle::PauseState;
use super::states_sets::OverlayState;
use super::states_sets::ProgramState;
use super::world_state::SkyboxModel;

pub struct GuiPlugin;
impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(DefaultInspectorConfigPlugin)
        // .register_type::<GuiState>()
        .insert_resource(GuiState::default())
        .insert_resource(StatusVisible(false))
        .init_resource::<GrabState>()
        .add_message::<GrabCursor>()
        .configure_loading_state(
            LoadingStateConfig::new(ProgramState::Initializing)
                .load_collection::<GuiAssets>()
        )
        .configure_loading_state(
            LoadingStateConfig::new(ProgramState::LoadingSave)
                .load_collection::<GuiAssets>()
        )
        .add_systems(OnEnter(ProgramState::InGame),
            (
                check_gui_state,    // initialize
                ensure_font_assets,
                grab_cursor_for_game,
                setup_gui_nodes,
                // (
                //     ensure_egui_context,
                //     setup_egui_style,
                // )
                //     .after(EguiStartupSet::InitContexts)
                //     .in_set(SimulationSystems),
            )
            .chain()
            // .in_set(InteractionSystems)
        )
        .add_systems(OnEnter(OverlayState::Hidden),
            grab_cursor_for_game,
        )
        .add_systems(OnExit(OverlayState::Hidden),
            ungrab_cursor_for_overlay,
        )
        .add_systems(OnEnter(ProgramState::Initializing),
            on_loading)
        .add_systems(OnExit(ProgramState::Initializing),
            on_loading_finished)
        .add_systems(OnEnter(OverlayState::Loading),
            on_loading)
        .add_systems(OnExit(OverlayState::Loading),
            on_loading_finished)
        .add_systems(
            Update,
            check_gui_state.run_if(resource_changed::<GuiState>.or(resource_changed::<State<OverlayState>>)),
        )
        .add_systems(
            Update,
            (
                // update_debug_status,
                check_grab_focus_state,
                update_pause_ui,
                update_mute_ui,
            )
            // .in_set(InteractionSystems)
            .run_if(in_state(ProgramState::InGame))
        )
        // .add_systems(
        //     Update,
        //     update_status_messages
        //     .in_set(InteractionSystems)
        //     .run_if(in_state(ProgramState::InGame))
        // )
        ;
    }
}

#[derive(Resource, AssetCollection)]
pub struct GuiAssets {
    #[asset(path = "fonts/Recursive-Bold.ttf")]
    pub std_ui: Handle<Font>,
    #[asset(path = "fonts/emoji-icon-font.ttf")]
    pub emoji: Handle<Font>,
    #[asset(path = "textures/crosshair.png")]
    pub crosshair: Handle<Image>,
}

impl GuiAssets {
    pub const STD_UI_FONT_PATH: &'static str = "fonts/Recursive-Bold.ttf";
    pub const STD_UI_FONT_NAME: &'static str = "Recursive";
}

fn ensure_font_assets(
    world: &mut World,
) {
    world.init_collection::<GuiAssets>();
}

#[derive(Component)]
pub(crate) struct LoadingScreen;

pub(crate) fn on_loading(
    mut commands: Commands,
    fonts: Option<Res<GuiAssets>>,
) {
    let ent_commands = commands.spawn((
        Name::new("Loading..."),
        LoadingScreen,
        DespawnOnExit(OverlayState::Loading)
    ));
    setup_loading_screen(ent_commands, fonts.as_deref());
}

pub(crate) fn on_loading_finished(
    mut commands: Commands,
    loading_q: Query<Entity, With<LoadingScreen>>,
) {
    for ent in loading_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub fn setup_loading_screen(
    mut ent_commands: EntityCommands,
    fonts: Option<&GuiAssets>,
) -> Entity {
    ent_commands.insert((
        DespawnOnExit(OverlayState::Loading),
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            .. default()
        },
        BackgroundColor(tailwind::BLUE_950.with_alpha(0.75).into()),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    ))
    .with_children(|builder| {
        builder.spawn((
            Text::new(
                "Loading...",
            ),
            TextFont {
                font: fonts.map_or(default(), |fonts| fonts.std_ui.clone()),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
    })
    .id()
}

impl Default for GrabState {
    fn default() -> Self {
        Self {
            was_grabbed: false,
            options: CursorOptions{
                visible: false,
                grab_mode: GRABBED_MODE,
                .. default()
            }
        }
    }
}

const GRABBED_MODE: CursorGrabMode = CursorGrabMode::Locked;

/// Indicate the desire to change the cursor grab state (false = not grabbed).
#[derive(Message)]
pub struct GrabCursor(pub bool);

/// Tells whether we're in a mode where the status area is displayed.
#[derive(Resource, Clone, PartialEq)]
pub(crate) struct StatusVisible(pub bool);

#[derive(Resource, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub(crate) struct GuiState {
    pub(crate) show_status: bool,
    pub(crate) show_fps: bool,
    pub(crate) show_skybox: bool,
    pub(crate) show_inspector: bool,
    pub(crate) show_inspector_always: bool,
    pub(crate) show_help: bool,
    pub(crate) show_dev_info: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            show_status: false,
            show_fps: false,
            show_skybox: true,
            show_inspector: true,
            show_inspector_always: false,
            show_help: false,
            show_dev_info: false,
        }
    }
}


#[derive(Resource)]
pub(crate) struct GrabState{ was_grabbed: bool, options: CursorOptions }

fn check_gui_state(
    state: Res<GuiState>,
    mut fps: ResMut<bevy::dev_tools::fps_overlay::FpsOverlayConfig>,
    mut status_visible: ResMut<StatusVisible>,
    mut skybox_q: Query<&mut SkyboxModel>,
    overlay: Res<State<OverlayState>>,
) {
    fps.enabled = state.show_fps || overlay.is_debug();
    status_visible.0 = state.show_status;
    for mut skybox in skybox_q.iter_mut() {
        skybox.enabled = state.show_skybox;
    }
}

fn grab_cursor_for_game(
    mut commands: Commands,
) {
    commands.write_message(GrabCursor(true));
}

fn ungrab_cursor_for_overlay(
    mut commands: Commands,
) {
    commands.write_message(GrabCursor(false));
}

fn check_grab_focus_state(
    mut grab: MessageReader<GrabCursor>,
    mut focused: MessageReader<WindowFocused>,
    overlay_state: Res<State<OverlayState>>,
    mut grab_state: ResMut<GrabState>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut desired_grab: Option<bool> = None;

    for event in focused.read() {
        if !event.focused {
            desired_grab = Some(false);
        } else {
            desired_grab = Some(*overlay_state.get() == OverlayState::Hidden);
        }
    }

    for event in grab.read() {
        desired_grab = Some(event.0);
    }

    if let Some(grab) = desired_grab {
        if grab {
            cursor_options.grab_mode = GRABBED_MODE;
            cursor_options.visible = false;

            grab_state.was_grabbed = true;
        } else {
            if grab_state.was_grabbed {
                grab_state.was_grabbed = false;
                grab_state.options = cursor_options.clone();
            }

            // Release mouse, if captured
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
}

#[derive(Component)]
struct Info;

#[derive(Component)]
struct PauseArea;

#[derive(Component)]
struct MuteArea;

fn setup_gui_nodes(
    mut commands: Commands,
    gui_assets: Res<GuiAssets>,
) {
    // Info
    commands.spawn((
        DespawnOnExit(ProgramState::InGame),
        Info,
        Text::new(""),
        TextFont {
            font: gui_assets.emoji.clone(),
            font_size: 10.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // Pause icon in upper right
    commands.spawn((
        DespawnOnExit(ProgramState::InGame),
        PauseArea,
        Visibility::Visible,
        TextFont {
            font: gui_assets.emoji.clone(),
            font_size: 32.0,
            .. default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            right: Val::Px(4.0),
            .. default()
        },
        Text::new(""),
    ));

    // Mute icon in upper right
    commands.spawn((
        DespawnOnExit(ProgramState::InGame),
        MuteArea,
        Visibility::Visible,
        TextFont {
            font: gui_assets.emoji.clone(),
            font_size: 32.0,
            .. default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            right: Val::Px(36.0),
            .. default()
        },
        Text::new(""),
    ));

}

fn update_pause_ui(
    paused: Res<PauseState>,
    mut text_q: Query<&mut Text, With<PauseArea>>,
) {
    if let Ok(mut text) = text_q.single_mut() {
        let new_text = if paused.is_paused() { "\u{1F6AB}" } else { " " };
        if new_text != text.0 {
            text.0 = new_text.to_string();
        }
    }
}

fn update_mute_ui(
    vol_q: Single<&UserVolume, With<MainBus>>,
    mut text_q: Query<&mut Text, With<MuteArea>>,
) {
    if let Ok(mut text) = text_q.single_mut() {
        let new_text = if vol_q.muted { "\u{1F508}" } else { " " };
        if new_text != text.0 {
            text.0 = new_text.to_string();
        }
    }
}
