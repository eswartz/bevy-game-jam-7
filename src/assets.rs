use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::asset_collection::AssetCollectionWorld as _;
use bevy_seedling::sample::AudioSample;

use crate::common::SkyboxTransform;


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
    pub action_rev: Handle<AudioSample>,
    #[asset(path = "sounds/burp_sound.ogg")]
    pub belch_1: Handle<AudioSample>,
    #[asset(path = "sounds/burp_sound_2.ogg")]
    pub belch_2: Handle<AudioSample>,
    #[asset(path = "sounds/burp_sound_3.ogg")]
    pub belch_3: Handle<AudioSample>,
    #[asset(path = "sounds/487531__ranner__bubble-short.ogg")]
    pub click: Handle<AudioSample>,
    #[asset(path = "sounds/clip-1-611277__xkeril__footsteps-on-snow-clean.ogg")]
    pub shake: Handle<AudioSample>,
}


#[derive(Resource, AssetCollection)]
pub struct MapAssets {
    #[asset(path = "test.glb#Scene0")]
    pub level_test: Handle<Scene>,
}

#[derive(Resource, AssetCollection)]
pub struct SkyboxAssets {
    // #[asset(path = "textures/kloppenheim_06_puresky_4k.exr")]
    // #[allow(unused)]
    // pub kloppenheim_sky_map: Handle<Image>,

    #[asset(path = "textures/starmap_2020_4k.exr")]
    #[allow(unused)]
    pub star_map: Handle<Image>,

    // #[asset(path = "textures/driving_school_4k.exr")]
    // #[allow(unused)]
    // pub driving_school: Handle<Image>,

    #[asset(path = "textures/farm_field_puresky_4k.exr")]
    #[allow(unused)]
    pub pure_sky: Handle<Image>,

    // #[asset(path = "textures/graffiti_shelter_4k.ktx2")]
    // #[allow(unused)]
    // pub graffiti_reflection: Handle<Image>,

    // #[asset(path = "textures/farm_field_4k.exr")]
    // #[allow(unused)]
    // pub farm_field: Handle<Image>,

    // #[asset(path = "textures/zwinger_night_4k.exr")]
    // #[allow(unused)]
    // pub zwinger: Handle<Image>,
}
