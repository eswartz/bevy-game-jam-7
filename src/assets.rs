use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_seedling::sample::AudioSample;

use crate::game::MusicTrack;

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
    // pub const STD_UI_FONT_PATH: &'static str = "fonts/Recursive-Bold.ttf";
    // pub const STD_UI_FONT_NAME: &'static str = "Recursive";
}

#[derive(Resource, AssetCollection)]
#[allow(unused)]
pub struct MusicAssets {
    #[asset(path = "music/song0.ogg")]
    pub song0: Handle<AudioSample>,
    #[asset(path = "music/song2.ogg")]
    pub song2: Handle<AudioSample>,
}

impl MusicAssets {
    pub fn get_for(&self, selection: &MusicTrack) -> &Handle<AudioSample> {
        match selection {
            MusicTrack::Song0 => &self.song0,
            MusicTrack::Song2 => &self.song2,
        }
    }
}

#[derive(Resource, AssetCollection)]
#[allow(unused)]
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
    #[asset(path = "sounds/clip-1-459583__vintage2005__snap-buttons.ogg")]
    pub snap_1: Handle<AudioSample>,
    #[asset(path = "sounds/clip-2-459583__vintage2005__snap-buttons.ogg")]
    pub snap_2: Handle<AudioSample>,
    #[asset(path = "sounds/clip-3-459583__vintage2005__snap-buttons.ogg")]
    pub snap_3: Handle<AudioSample>,
    #[asset(path = "sounds/edited-415538__thescarlettwitch89__sloshing.ogg")]
    pub sloshing: Handle<AudioSample>,
    #[asset(path = "sounds/out-86228__nmscher__car_-internal_warning-ding_plymouth-acclaim_edited.ogg")]
    pub off: Handle<AudioSample>,
    #[asset(path = "sounds/recover-86228__nmscher__car_-internal_warning-ding_plymouth-acclaim_edited.ogg")]
    pub on: Handle<AudioSample>,
    #[asset(path = "sounds/swish-178056__eneasz__folder-snapped-shut.ogg")]
    pub swish: Handle<AudioSample>,
    #[asset(path = "sounds/fail_612892__avajoliec__19-plain-creak.ogg")]
    pub loss: Handle<AudioSample>,
    #[asset(path = "sounds/250518-bong.ogg")]
    pub gain: Handle<AudioSample>,
    #[asset(path = "sounds/257803__xtrgamr__swish-2.ogg")]
    pub swoosh: Handle<AudioSample>,
}


#[derive(Resource, AssetCollection)]
pub struct MapAssets {
    #[asset(path = "maps/level_0.glb#Scene0")]
    pub level_0: Handle<Scene>,
    #[asset(path = "maps/level_1.glb#Scene0")]
    pub level_1: Handle<Scene>,
    #[asset(path = "maps/level_2.glb#Scene0")]
    pub level_2: Handle<Scene>,
    #[asset(path = "maps/level_3.glb#Scene0")]
    pub level_3: Handle<Scene>,
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

#[derive(Resource, AssetCollection)]
pub struct ModelAssets {
    #[asset(path = "models/sphere.glb#Scene0")]
    pub sphere: Handle<Scene>,

    #[asset(path = "models/net.glb#Scene0")]
    pub net: Handle<Scene>,
}
