//! Boundary warning systems for player straying too far.

use bevy::prelude::*;

use crate::plugins::core::EventLog;
use crate::plugins::player::PlayerControl;

// =============================================================================
// Components
// =============================================================================

#[derive(Component, Default)]
pub struct BoundaryWarningState {
    pub last_level: BoundaryWarningLevel,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum BoundaryWarningLevel {
    #[default]
    Safe,
    SoftWarning,
    DangerZone,
}

// =============================================================================
// Constants
// =============================================================================

const BOUNDARY_SOFT_WARNING: f32 = 1200.0;
const BOUNDARY_DANGER_ZONE: f32 = 2200.0;

// =============================================================================
// Systems
// =============================================================================

pub fn check_boundary_warnings(
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut BoundaryWarningState), With<PlayerControl>>,
) {
    let (transform, mut warning_state) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => return,
    };

    let player_pos = Vec2::new(transform.translation.x, transform.translation.y);
    let distance_from_origin = player_pos.length();

    let current_level = if distance_from_origin >= BOUNDARY_DANGER_ZONE {
        BoundaryWarningLevel::DangerZone
    } else if distance_from_origin >= BOUNDARY_SOFT_WARNING {
        BoundaryWarningLevel::SoftWarning
    } else {
        BoundaryWarningLevel::Safe
    };

    if current_level != warning_state.last_level {
        match current_level {
            BoundaryWarningLevel::Safe => {
                // Don't log when returning to safe zone
            }
            BoundaryWarningLevel::SoftWarning => {
                log.push("Long-range sensors detect signal degradation. Consider returning to civilization.".to_string());
            }
            BoundaryWarningLevel::DangerZone => {
                log.push("WARNING: You are drifting into the void. Hull stress increasing. Fuel reserves critical. Turn back NOW.".to_string());
            }
        }
        warning_state.last_level = current_level;
    }
}
