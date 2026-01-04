//! Player-specific components and resources.

use bevy::prelude::*;

// =============================================================================
// Components
// =============================================================================

/// Marker component for the player-controlled ship.
#[derive(Component, Debug, Default)]
pub struct PlayerControl;

// =============================================================================
// Resources
// =============================================================================

/// Tracks nearby targetable entities in the player's zone.
#[derive(Resource, Default)]
pub struct NearbyTargets {
    pub entities: Vec<(Entity, Vec2, String)>,
    pub selected_index: usize,
    /// True only after the player has pressed Tab to explicitly select a target
    pub manually_selected: bool,
}

/// Tracks autopilot engagement and target state.
#[derive(Resource, Default)]
pub struct AutopilotState {
    /// Whether autopilot is currently engaged
    pub engaged: bool,
    /// Target entity we're navigating toward
    pub target_entity: Option<Entity>,
}
