//! Simulation plugin for game logic and AI systems.

mod boundary;
mod intel;
mod ore;
mod pirates;
mod scouts;
mod ships;
mod stations;

use bevy::prelude::*;

use crate::plugins::core::SimConfig;
use crate::world::Sector;

// Re-export public items
pub use boundary::BoundaryWarningState;
pub use intel::{advance_intel_layer, refresh_intel, zone_modifier_risk};
pub use ore::RevealedNodesTracker;

// =============================================================================
// Plugin
// =============================================================================

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimTickCount>()
            .init_resource::<RevealedNodesTracker>()
            .add_systems(
                FixedUpdate,
                (
                    tick_simulation,
                    intel::decay_intel,
                    stations::station_fuel_burn,
                    stations::station_ore_production,
                    stations::station_build_progress,
                    stations::station_crisis_stub,
                    stations::station_lifecycle,
                    stations::log_station_crisis_changes,
                    scouts::scout_behavior,
                    ore::spawn_ore_at_revealed_nodes,
                    boundary::check_boundary_warnings,
                    pirates::pirate_launches,
                    pirates::pirate_move,
                    pirates::pirate_harassment,
                    ships::ship_fuel_burn,
                    ships::ship_fuel_alerts,
                    ships::ship_state_stub,
                )
                    .run_if(sim_not_paused),
            )
            .add_systems(Update, scouts::handle_scout_risk_input);
    }
}

// =============================================================================
// Resources
// =============================================================================

#[derive(Resource, Default)]
pub struct SimTickCount {
    pub tick: u64,
}

// =============================================================================
// Run Conditions
// =============================================================================

fn sim_not_paused(config: Res<SimConfig>) -> bool {
    !config.paused
}

// =============================================================================
// Systems
// =============================================================================

fn tick_simulation(mut counter: ResMut<SimTickCount>, sector: Res<Sector>) {
    counter.tick = counter.tick.saturating_add(1);

    if counter.tick.is_multiple_of(10) {
        let total_distance = sector
            .routes
            .iter()
            .map(|route| route.distance)
            .sum::<f32>();

        let endpoint_sum = sector
            .routes
            .iter()
            .map(|route| route.from + route.to)
            .sum::<u32>();

        let average_risk = if sector.routes.is_empty() {
            0.0
        } else {
            let total_risk = sector.routes.iter().map(|route| route.risk).sum::<f32>();
            total_risk / (sector.routes.len() as f32)
        };

        let modifier_risk = zone_modifier_risk(&sector);

        info!(
            "Sim tick {} (nodes: {}, routes: {}, distance: {:.2}, endpoints: {}, risk: {:.2}, mod: {:.2})",
            counter.tick,
            sector.nodes.len(),
            sector.routes.len(),
            total_distance,
            endpoint_sum,
            average_risk,
            modifier_risk
        );
    }
}
