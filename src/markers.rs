use std::time::Duration;

use avian3d::prelude::PhysicsLayer;
use bevy::prelude::*;

/// Mark the object for persistence.
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Saveable;

/// Mark an entity as temporary.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct DespawnAfter(pub Duration);

#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum GameLayer {
    #[default]
    /// Layer 0 - the default layer that objects are assigned to
    Default,
    /// Layer 1 = player/camera
    Player,
    /// Layer 2 - static geometry
    World,
    /// Layer 3 - components with gameplay-specific physics
    Gameplay,
    /// Layer 4 - temporary bullets/projectiles/etc.
    Projectiles,
}
