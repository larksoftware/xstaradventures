//! Player plugin for ship control and interactions.
//!
//! This module provides:
//! - Manual ship movement (thrust, rotation, braking)
//! - Autopilot navigation to targets
//! - Mining, building, refueling interactions
//! - Jump gate activation and zone transitions
//! - Target scanning and selection
//! - Combat (firing at pirates)

mod autopilot;
mod components;
mod docking;
mod gates;
mod interactions;
mod movement;
mod targeting;

use bevy::prelude::*;

use crate::plugins::core::SimConfig;

// Re-export public types
pub use components::{AutopilotState, DockingState, NearbyTargets, PlayerControl};
pub use gates::process_jump_transition;
#[allow(unused_imports)]
pub use targeting::{filter_entities_by_zone, find_zone_for_position};

// =============================================================================
// Plugin
// =============================================================================

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NearbyTargets>()
            .init_resource::<AutopilotState>()
            .init_resource::<DockingState>()
            // Continuous simulation systems (FixedUpdate)
            .add_systems(
                FixedUpdate,
                (
                    movement::player_movement
                        .run_if(autopilot::autopilot_not_engaged)
                        .run_if(docking::player_not_docked),
                    interactions::player_mining.run_if(docking::player_not_docked),
                    interactions::player_identify_nearby,
                    gates::process_jump_transition,
                )
                    .run_if(sim_not_paused),
            )
            .add_systems(
                FixedUpdate,
                targeting::scan_nearby_entities.run_if(targeting::view_is_world),
            )
            .add_systems(
                FixedUpdate,
                autopilot::autopilot_control_system
                    .run_if(sim_not_paused)
                    .run_if(autopilot::autopilot_engaged)
                    .run_if(docking::player_not_docked)
                    .after(targeting::scan_nearby_entities),
            )
            // One-shot input systems (Update) - just_pressed must run in Update
            .add_systems(
                Update,
                (
                    targeting::handle_tactical_selection,
                    autopilot::autopilot_input_system,
                    docking::player_undock.run_if(docking::player_is_docked),
                    // One-shot interactions that use just_pressed
                    gates::player_activate_jump_gate
                        .run_if(sim_not_paused)
                        .run_if(gates::not_in_jump_transition)
                        .run_if(docking::player_not_docked),
                    docking::player_dock_station
                        .run_if(sim_not_paused)
                        .run_if(docking::player_not_docked),
                    interactions::player_fire
                        .run_if(sim_not_paused)
                        .run_if(docking::player_not_docked),
                    interactions::player_refuel_station
                        .run_if(sim_not_paused)
                        .run_if(docking::player_not_docked),
                    interactions::player_build_outpost
                        .run_if(sim_not_paused)
                        .run_if(docking::player_not_docked),
                )
                    .run_if(targeting::view_is_world),
            );
    }
}

// =============================================================================
// Run Conditions
// =============================================================================

fn sim_not_paused(config: Res<SimConfig>) -> bool {
    !config.paused
}
