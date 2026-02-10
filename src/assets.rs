use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::asset_collection::AssetCollectionWorld as _;
use bevy_seedling::sample::AudioSample;


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


#[derive(Resource, AssetCollection)]
pub struct MapAssets {
    #[asset(path = "test.glb#Scene0")]
    pub level_test: Handle<Scene>,
}
