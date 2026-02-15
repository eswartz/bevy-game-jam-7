use std::time::Duration;

use crate::assets::*;
use crate::game::*;
use crate::common::*;

use bevy::prelude::*;

pub(crate) const ID: &str = "level0";
pub(crate) const NAME: &str = "Level 0";

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnEnter(ProgramState::New),
                register_level
            )
            .add_systems(
                OnEnter(LevelState::LevelLoaded),
                on_level_loaded
                    .run_if(is_in_level(ID))
            )
        ;
    }
}

fn register_level(mut list: ResMut<LevelList>, maps: Res<MapAssets>) {
    list.0.push(LevelInfo {
        id: ID.to_string(),
        label: NAME.to_string(),
        scene: maps.level_0.clone()
    });
}

fn on_level_loaded(
    mut commands: Commands,
    viewer_camera_q: Query<Entity, (With<Camera3d>, With<ViewerCamera>)>,
    models: Res<ModelAssets>,

) {
    let cam = viewer_camera_q.single().unwrap();
    spawn_net(commands.reborrow(), models, cam);

    commands.insert_resource(Spawning(false));
    commands.insert_resource(SpawnDelay(Duration::from_secs(1)));
    commands.insert_resource(SpawnTimer(Timer::new(Duration::from_secs(1), TimerMode::Repeating)));
    commands.insert_resource(ShakeTime(Duration::ZERO));
}
