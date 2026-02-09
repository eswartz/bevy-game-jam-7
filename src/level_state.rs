use std::collections::HashMap;
use std::time::Duration;

use anyhow::bail;
use avian3d::prelude::*;
use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::system::SystemParam;
use bevy::math::Affine2;
use bevy::prelude::*;
// use bevy_trenchbroom::class::builtin::InfoPlayerStart;
// use bevy_trenchbroom::prelude::GenericMaterial3d;
// use midi_synth::synth::MidiSynthParams;
use rand::Rng as _;
use strum::EnumIter;
use strum::VariantArray;

use crate::states_sets::GameplayState;
use crate::states_sets::ProgramState;

// use crate::prelude::*;
// use daunt_common::prelude::*;
// use daunt_scripting::prelude::*;

pub struct LevelStatePlugin;

impl Plugin for LevelStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<LevelState>()
            .add_message::<LevelCreateMessage>()
            .add_message::<LevelGeometryLoadedMessage>()
            .add_message::<LevelContentDefinedMessage>()
            .add_message::<LevelLoadFinishedMessage>()
            // .register_type::<LevelInfo>()
            // .register_type::<LevelRegistry>()
            // .register_type::<LevelId>()
            // .register_type::<LevelTempo>()
            // .register_type::<LevelState>()
            // .register_type::<State<LevelState>>()
            // .register_type::<LevelSolveStates>()
            // .register_type::<SetStyle>()
            // .register_type::<ActiveNotes>()
            // .register_type::<ActiveNote>()
            // .register_type::<MidiSynthProxy>()
            .init_resource::<LevelRegistry>()
            .init_resource::<LevelId>()
            // .init_resource::<LevelDifficulty>()
            // .init_resource::<LevelTempo>()
            // .init_resource::<LevelSolveStates>()
            // .init_resource::<LevelMetadata>()
            // .init_resource::<LevelSolutionState>()
            // .init_resource::<ActiveNotes>()
            // .add_systems(
            //     PreUpdate,
            //     (
            //         style_level_content,
            //         tweak_level_content,
            //     )
            //     // .in_set(SimulationSystems)
            //     .run_if(in_state(GameplayState::AssetsLoaded).or(in_state(GameplayState::Playing)))
            //     .run_if(in_state(ProgramState::InGame))
            //     ,
            // )
            .add_systems(
                OnEnter(LevelState::Advance),
                queue_next_level, //.in_set(SimulationSystems),
            );
    }
}

/// State of a level (there is only one level in play at a time).
#[derive(States, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[states(scoped_entities)]
#[reflect(Default)]
#[type_path = "game"]

pub enum LevelState {
    /// Outside the play area.
    #[default]
    Outside,
    /// User entered level, not solving it, and can futz about.
    Exploring,
    /// User is preparing to solve.
    PreSolve,
    /// User is testing a puzzle solution.
    Solving,
    /// User reached a puzzle solution.
    Passed,
    /// User did not reach a puzzle solution.
    Failed,
    /// Puzzle was solved.
    Solved,
    /// Switching levels; will trigger Outside again.
    Advance,
}
impl LevelState {
    pub(crate) fn is_active(&self) -> bool {
        match self {
            LevelState::Solving => true,
            LevelState::Outside |
            LevelState::Exploring |
            LevelState::PreSolve |
            LevelState::Passed |
            LevelState::Failed |
            LevelState::Solved |
            LevelState::Advance => false,
        }
    }
}

// /// Which manual level we want to start.
// #[derive(Resource, Default, Debug, Clone, Reflect)]
// #[reflect(Resource)]
// #[type_path = "game"]
// pub struct StartupLevel(pub String);

/// Current level.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct LevelId(pub String);

/// A known level.
#[derive(Default, Clone, Reflect)]
#[reflect(Clone, Default)]
#[type_path = "game"]
pub struct LevelInfo {
    pub id: String,
    pub name: String,
    pub bsp_path: Option<String>,
    // #[reflect(ignore)]
    // pub obj: mlua::Value,
    /// If set, this is a level meant for testing.
    pub is_test: bool,
}

impl LevelInfo {
    pub fn get_bsp_path(&self) -> Option<String> {
        self.bsp_path.clone()
    }
    pub fn get_level_id(&self) -> String {
        self.id.clone()
    }
}

impl std::fmt::Display for LevelInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl LevelInfo {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        bsp_path: Option<String>,
        // obj: <LuaScriptManager as ScriptManager>::Value,
    ) -> Self {
        let id = id.into();
        let name = name.into();
        Self {
            id,
            name,
            bsp_path,
            // obj,
            is_test: false,
        }
    }

    pub fn with_test(self, is_test: bool) -> Self {
        Self {
            is_test,
            .. self
        }
    }
}

/// Known levels.
#[derive(Resource, Default, Clone, Reflect)]
#[reflect(Resource, Clone, Default)]
#[type_path = "game"]
pub struct LevelRegistry {
    /// id of the first level to run in a new game.
    /// (Otherwise ChangeLevelRequest says)
    start_id: String,

    levels: HashMap<String, LevelInfo>,
    level_ids: Vec<String>,
}

impl LevelRegistry {
    pub fn set_start_level(&mut self, id: &str) {
        self.start_id = id.to_string();
    }
    pub fn register_level(&mut self, level: LevelInfo) -> usize {
        if !self.level_ids.contains(&level.id) {
            self.level_ids.push(level.id.clone());
        }
        let index = self.level_ids.iter().position(|id| *id == level.id).unwrap();
        self.levels.insert(level.id.clone(), level);
        index
    }
    pub fn get_level_ids(&self) -> Vec<String> {
        self.level_ids.clone()
    }
    pub fn contains_level(&self, id: &str) -> bool {
        self.levels.contains_key(id)
    }
    pub fn find_level(&self, id: &str) -> Option<LevelInfo> {
        self.levels.get(id).cloned()
    }
    pub fn start_level(&self) -> String {
        self.start_id.clone()
    }

    /// Get the next level in definition order (e.g. for a combo in the menu).
    pub fn next_defined_level(&self, id: &str) -> String {
        if let Some(index) = self
            .level_ids
            .iter()
            .position(|i| *i == id)
            && let Some(next) = self.level_ids.get(index + 1) {
                return next.clone();
            }
        self.start_id.clone()
    }
    pub fn level_name(&self, id: &str) -> String {
        if let Some(info) = self.levels.get(id) {
            info.name.clone()
        } else {
            id.to_string()
        }
    }
    pub fn level_index_id(&self, index: usize) -> String {
        self.level_ids[index].clone()
    }
    pub fn level_count(&self) -> usize {
        self.level_ids.len()
    }
    pub fn level_id_index(&self, id: &str) -> Option<usize> {
        self.level_ids.iter().position(|level_id| *level_id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LevelLoadKind {
    /// Emitted after level loading or when the user issues "Reset"
    /// (which despawns the entire puzzle `root` before loading).
    ///
    /// This should e.g. scan the level,
    /// look for marker entities that define the puzzle,
    /// and establish any observers/entities/etc. that are not
    /// represented in the .bsp.
    New,
    /// Emitted after loading a save. Any [Saveable] entities will be
    /// present.
    ///
    /// This should do whatever work is needed to re-establish
    /// observers/entities/etc. that are not
    /// represented in the save state or .bsp.
    Reloaded,
}

/// This event tells the game that a level is being created,
/// between a clean slate and wiring up the mechanics of a puzzle.
/// Listeners can add/modify Func... components during this time.
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LevelCreateMessage {
    /// How the level was loaded.
    pub kind: LevelLoadKind,
    /// Root entity of the level content.
    pub root: Entity,
}

/// This event from the map loader
/// tells the game that the raw content of a level (but no metadata) was loaded.
/// Listeners can add/modify Func... components during this time.
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LevelGeometryLoadedMessage;

/// This event from the map loader
/// tells the game that the content is finished.
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LevelContentDefinedMessage;

/// This event tells the game that the LevelMetadata has been established from its content.
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LevelLoadFinishedMessage;


/// The root of the level.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct LevelRoot {
    pub root: Entity,
    pub level_id: String,
}


/// Tell if a level was just loaded.
pub fn level_loaded(reader: MessageReader<LevelLoadFinishedMessage>) -> bool {
    !reader.is_empty()
}

fn queue_next_level(
    mut commands: Commands,
    level_id: Res<LevelId>,
    level_regy: Res<LevelRegistry>,
    // mut script_mgr: ResMut<LuaScriptManager>,
) -> Result {
    let this_level_id = level_id.0.clone();
    if this_level_id.is_empty() {
        return Err("No level id defined!".into());
    }
    let Some(level_info) = level_regy.find_level(&this_level_id) else {
        return Err(format!("No level info for level '{this_level_id}' defined!").into());
    };

    // script_mgr.globals().put("level", level_info.obj)?;

    // let next_index = {
    //     if let Ok(rez) = script_mgr.exec(
    //         &mut *script_mgr.globals(),
    //         "return level:get_next_level()",
    //     ) {
    //         let next_level_id = from_script::<String>(&mut script_mgr, rez)?;

    //         let next_index = level_regy.level_id_index(&next_level_id);
    //         if next_index.is_none() {
    //             error!(
    //                 "No such level defined from {this_level_id:?} to {next_level_id:?}, selecting next in order."
    //             );
    //         }
    //         next_index
    //     } else {
    //         None
    //     }
    // };
    // let next_index = match next_index {
    //     Some(next_index) => next_index,
    //     None => {
    //         // nothing defined, use typical
    //         let index = level_regy.level_id_index(&level_id.0).unwrap_or_default();
    //         (index + 1) % level_regy.level_count()
    //     }
    // };

    // commands.insert_resource(NextLevelIndex(next_index));

    commands.set_state(LevelState::Outside);

    Ok(())
}
