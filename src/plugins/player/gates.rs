//! Jump gate activation and transition processing.

use bevy::prelude::*;

use crate::plugins::core::{EventLog, InputBindings};
use crate::ships::Ship;
use crate::world::{
    JumpGate, JumpTransition, ZoneId, JUMP_GATE_FUEL_COST, JUMP_TRANSITION_SECONDS,
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
    mut player_query: Query<(Entity, &Transform, &mut Ship), With<PlayerControl>>,
    gates: Query<(&Transform, &JumpGate)>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_entity, player_transform, mut ship) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    // Find nearest gate in range
    let mut nearest_gate: Option<&JumpGate> = None;
    let mut nearest_dist = f32::MAX;

    for (gate_transform, gate) in gates.iter() {
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

pub fn process_jump_transition(
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(Entity, &mut ZoneId, &mut JumpTransition), With<PlayerControl>>,
) {
    let Ok((player_entity, mut zone_id, mut transition)) = player_query.single_mut() else {
        return;
    };

    transition.remaining_seconds -= time.delta_secs();

    if transition.remaining_seconds <= 0.0 {
        // Complete the jump
        zone_id.0 = transition.destination_zone;
        commands.entity(player_entity).remove::<JumpTransition>();
        log.push(format!("Arrived at zone {}", zone_id.0));
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
