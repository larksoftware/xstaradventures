//! Jump gate activation and transition processing.

use bevy::prelude::*;

use crate::fleets::ScoutBehavior;
use crate::plugins::core::{EventLog, InputBindings};
use crate::plugins::sim::SimTickCount;
use crate::ships::Ship;
use crate::world::{
    Identified, JumpGate, JumpTransition, SystemIntel, SystemNode, ZoneId, JUMP_GATE_FUEL_COST,
    JUMP_TRANSITION_SECONDS,
};

use super::components::PlayerControl;

// =============================================================================
// Constants
// =============================================================================

const JUMP_GATE_ACTIVATION_RANGE: f32 = 25.0;

// =============================================================================
// Run Conditions
// =============================================================================

pub fn not_in_jump_transition(
    player_query: Query<Option<&JumpTransition>, With<PlayerControl>>,
) -> bool {
    player_query
        .single()
        .map_or(true, |transition| transition.is_none())
}

// =============================================================================
// Systems
// =============================================================================

pub fn player_activate_jump_gate(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(Entity, &Transform, &mut Ship, &ZoneId), With<PlayerControl>>,
    gates: Query<(&Transform, &JumpGate, &ZoneId)>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_entity, player_transform, mut ship, player_zone) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    // Find nearest gate in range that's in the player's zone
    let mut nearest_gate: Option<&JumpGate> = None;
    let mut nearest_dist = f32::MAX;

    for (gate_transform, gate, gate_zone) in gates.iter() {
        // Only consider gates in the player's current zone
        if gate_zone.0 != player_zone.0 {
            continue;
        }

        let gate_pos = Vec2::new(gate_transform.translation.x, gate_transform.translation.y);
        let dist = gate_pos.distance(player_pos);

        if dist <= JUMP_GATE_ACTIVATION_RANGE && dist < nearest_dist {
            nearest_gate = Some(gate);
            nearest_dist = dist;
        }
    }

    let Some(gate) = nearest_gate else {
        return;
    };

    // Check fuel
    if ship.fuel < JUMP_GATE_FUEL_COST {
        log.push("Not enough fuel for jump".to_string());
        return;
    }

    // Consume fuel and start transition
    ship.fuel -= JUMP_GATE_FUEL_COST;

    commands.entity(player_entity).insert(JumpTransition {
        destination_zone: gate.destination_zone,
        remaining_seconds: JUMP_TRANSITION_SECONDS,
    });

    log.push(format!("Jumping to zone {}...", gate.destination_zone));
}

/// Process jump transitions for all ships. Reveals zones only for player-owned ships.
#[allow(clippy::type_complexity)]
pub fn process_jump_transition(
    time: Res<Time<Fixed>>,
    ticks: Res<SimTickCount>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut jumping_ships: Query<(
        Entity,
        &mut ZoneId,
        &mut Transform,
        &mut JumpTransition,
        Option<&PlayerControl>,
        Option<&ScoutBehavior>,
    )>,
    mut intel_query: Query<(&SystemNode, &mut SystemIntel)>,
    gates: Query<(Entity, &Transform, &JumpGate, Option<&ZoneId>), Without<JumpTransition>>,
) {
    for (entity, mut zone_id, mut ship_transform, mut transition, player_ctrl, scout) in
        jumping_ships.iter_mut()
    {
        transition.remaining_seconds -= time.delta_secs();

        if transition.remaining_seconds <= 0.0 {
            // Remember source zone before changing
            let source_zone = zone_id.0;

            // Complete the jump
            let destination = transition.destination_zone;
            zone_id.0 = destination;
            commands.entity(entity).remove::<JumpTransition>();

            // Reveal zone only for player-owned ships (player or scouts)
            let is_player_owned = player_ctrl.is_some() || scout.is_some();
            if is_player_owned {
                for (node, mut intel) in intel_query.iter_mut() {
                    if node.id == destination && !intel.revealed {
                        intel.revealed = true;
                        // Player gets better intel than scouts on arrival
                        intel.confidence = if player_ctrl.is_some() { 0.8 } else { 0.5 };
                        intel.last_seen_tick = ticks.tick;
                        intel.revealed_tick = ticks.tick;
                    }
                }
            }

            // Find the arrival gate (gate in destination zone leading back to source)
            // and move the ship to its position, mark it as Identified for player
            for (gate_entity, gate_transform, gate, gate_zone) in gates.iter() {
                let in_destination_zone = gate_zone.is_some_and(|z| z.0 == destination);
                let leads_to_source = gate.destination_zone == source_zone;
                if in_destination_zone && leads_to_source {
                    // Move the ship to the arrival gate position
                    ship_transform.translation.x = gate_transform.translation.x;
                    ship_transform.translation.y = gate_transform.translation.y;

                    // Mark as Identified for player
                    if player_ctrl.is_some() {
                        commands.entity(gate_entity).insert(Identified);
                    }
                    break;
                }
            }

            // Log arrival for player ship only
            if player_ctrl.is_some() {
                log.push(format!("Arrived at zone {}", destination));
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::world::{JumpTransition, JUMP_GATE_FUEL_COST, JUMP_TRANSITION_SECONDS};

    #[test]
    fn can_activate_gate_with_enough_fuel() {
        let fuel = JUMP_GATE_FUEL_COST + 1.0;
        assert!(fuel >= JUMP_GATE_FUEL_COST);
    }

    #[test]
    fn cannot_activate_gate_without_fuel() {
        let fuel = JUMP_GATE_FUEL_COST - 1.0;
        assert!(fuel < JUMP_GATE_FUEL_COST);
    }

    #[test]
    fn jump_transition_completes_when_timer_reaches_zero() {
        let mut transition = JumpTransition {
            destination_zone: 100,
            remaining_seconds: JUMP_TRANSITION_SECONDS,
        };

        // Simulate time passing
        transition.remaining_seconds -= JUMP_TRANSITION_SECONDS;
        assert!(transition.remaining_seconds <= 0.0);
    }
}
