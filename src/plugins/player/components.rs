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

/// Tracks whether the player is docked at a station.
#[derive(Resource, Default)]
pub struct DockingState {
    /// Entity of the station the player is docked at (if any)
    pub docked_at: Option<Entity>,
}

impl DockingState {
    /// Check if player is currently docked
    pub fn is_docked(&self) -> bool {
        self.docked_at.is_some()
    }

    /// Dock at a station
    pub fn dock(&mut self, station: Entity) {
        self.docked_at = Some(station);
    }

    /// Undock from station
    pub fn undock(&mut self) {
        self.docked_at = None;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docking_state_starts_undocked() {
        let state = DockingState::default();
        assert!(!state.is_docked());
        assert!(state.docked_at.is_none());
    }

    #[test]
    fn docking_state_dock_sets_station() {
        let mut state = DockingState::default();
        let station = Entity::from_bits(42);
        state.dock(station);
        assert!(state.is_docked());
        assert_eq!(state.docked_at, Some(station));
    }

    #[test]
    fn docking_state_undock_clears_station() {
        let mut state = DockingState::default();
        let station = Entity::from_bits(42);
        state.dock(station);
        state.undock();
        assert!(!state.is_docked());
        assert!(state.docked_at.is_none());
    }
}
