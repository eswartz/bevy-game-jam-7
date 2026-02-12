
use bevy::prelude::*;
use avian3d::prelude::*;
use bevy_tweening::Lens;
use bevy_tweening::TweeningPlugin;

use crate::common::WorldCamera;


pub(crate) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(TweeningPlugin)
            .add_systems(Update, shrink_and_disappear)
            .add_systems(Update, aim_for_camera)
        ;
    }
}

/// Marker for things that should shrink and disappear.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct ShrinkAndDisappear;

fn shrink_and_disappear(mut commands: Commands,
    time: Res<Time<Physics>>,
    mut shrink_q: Query<(Entity, &mut Transform), With<ShrinkAndDisappear>>
) {
    for (ent, mut xfrm) in shrink_q.iter_mut() {
        let cur_scale = xfrm.scale.max_element();
        let new_scale = cur_scale - time.delta_secs() * 5.0;
        if new_scale >= 0.01 {
            xfrm.scale = Vec3::splat(new_scale);
        } else {
            commands.entity(ent).try_despawn();
        }
    }
}

/// Marker for things that should fly towards the camera.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub(crate) struct AimForCamera;

fn aim_for_camera(
    // mut commands: Commands,
    // time: Res<Time<Physics>>,
    camera_q: Single<(Entity, &Transform), (With<WorldCamera>, Without<AimForCamera>)>,
    mut aim_q: Query<(Entity, &mut Transform, &GlobalTransform), With<AimForCamera>>
) {
    let (_cam_ent, cam_xfrm) = *camera_q;
    for (_ent, mut xfrm, _gxfrm) in aim_q.iter_mut() {
        // dbg!(xfrm.translation, cam_xfrm.translation);
        xfrm.translation = xfrm.translation.lerp(cam_xfrm.translation, 0.25);
    }
}


#[derive(Debug)]
pub struct TransformPositionScaleLens {
    pub start: Transform,
    pub end: Transform,
}

impl Lens<Transform> for TransformPositionScaleLens {
    fn lerp(&mut self, mut target: Mut<Transform>, ratio: f32) {
        target.translation = self.start.translation.lerp(self.end.translation, ratio);
        target.scale = self.start.scale.lerp(self.end.scale, ratio);
    }
}
