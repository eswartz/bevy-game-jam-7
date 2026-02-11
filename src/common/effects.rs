
use bevy::prelude::*;
use avian3d::prelude::*;


pub(crate) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, shrink_and_disappear)
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
