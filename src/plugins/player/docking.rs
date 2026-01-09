//! Player docking systems for station interactions.

use bevy::prelude::*;

use crate::plugins::core::InputBindings;
use crate::stations::{Station, StationKind};

use super::components::{DockingState, PlayerControl};

/// Range at which player can dock at a station
pub const DOCKING_RANGE: f32 = 22.0;

// =============================================================================
// Systems
// =============================================================================

/// Handles docking at Shipyard and Refinery stations when player presses interact.
pub fn player_dock_station(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut docking: ResMut<DockingState>,
    player_query: Query<&Transform, With<PlayerControl>>,
    stations: Query<(Entity, &Transform, &Station)>,
) {
    // Only trigger on key press, not when already docked
    if !input.just_pressed(bindings.interact) {
        return;
    }

    // Don't dock if already docked
    if docking.is_docked() {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(value) => value,
        Err(_) => return,
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    // Find closest dockable station (Shipyard or Refinery)
    let mut closest: Option<(Entity, f32)> = None;

    for (entity, transform, station) in stations.iter() {
        // Only Shipyard and Refinery are dockable
        if !matches!(station.kind, StationKind::Shipyard | StationKind::Refinery) {
            continue;
        }

        let station_pos = Vec2::new(transform.translation.x, transform.translation.y);
        let dist = station_pos.distance(player_pos);

        if dist <= DOCKING_RANGE && (closest.is_none() || dist < closest.unwrap().1) {
            closest = Some((entity, dist));
        }
    }

    if let Some((station_entity, _)) = closest {
        docking.dock(station_entity);
    }
}

/// Handles undocking when player presses Escape or clicks undock button.
pub fn player_undock(input: Res<ButtonInput<KeyCode>>, mut docking: ResMut<DockingState>) {
    if !docking.is_docked() {
        return;
    }

    // Escape undocks
    if input.just_pressed(KeyCode::Escape) {
        docking.undock();
    }
}

// =============================================================================
// Run Conditions
// =============================================================================

/// Run condition: player is docked
pub fn player_is_docked(docking: Res<DockingState>) -> bool {
    docking.is_docked()
}

/// Run condition: player is not docked
pub fn player_not_docked(docking: Res<DockingState>) -> bool {
    !docking.is_docked()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docking_range_is_22_units() {
        assert!((DOCKING_RANGE - 22.0).abs() < f32::EPSILON);
    }
}
