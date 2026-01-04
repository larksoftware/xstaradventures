//! Scout behavior AI systems.

use bevy::prelude::*;

use crate::fleets::{next_risk, RiskTolerance, ScoutBehavior, ScoutPhase};
use crate::ore::OreNode;
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::{EventLog, InputBindings};
use crate::plugins::player::PlayerControl;
use crate::ships::{Ship, ShipState};
use crate::stations::Station;
use crate::world::{
    JumpGate, KnowledgeLayer, SystemIntel, SystemNode, ZoneId, JUMP_TRANSITION_SECONDS,
};

use super::SimTickCount;

// =============================================================================
// Constants
// =============================================================================

pub const SCOUT_SPEED: f32 = 80.0;
pub const SCOUT_GATE_RANGE: f32 = 25.0;

// =============================================================================
// Systems
// =============================================================================

pub fn handle_scout_risk_input(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut log: ResMut<EventLog>,
    mut scouts: Query<&mut ScoutBehavior>,
) {
    let delta = if input.just_pressed(bindings.scout_risk_down) {
        Some(-1)
    } else if input.just_pressed(bindings.scout_risk_up) {
        Some(1)
    } else {
        None
    };

    let delta = match delta {
        Some(value) => value,
        None => {
            return;
        }
    };

    let mut updated = None;
    for mut scout in scouts.iter_mut() {
        scout.risk = next_risk(scout.risk, delta);
        updated = Some(scout.risk);
    }

    if let Some(risk) = updated {
        let label = match risk {
            RiskTolerance::Cautious => "Cautious",
            RiskTolerance::Balanced => "Balanced",
            RiskTolerance::Bold => "Bold",
        };
        log.push(format!("Scout risk set to {}", label));
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn scout_behavior(
    time: Res<Time<Fixed>>,
    ticks: Res<SimTickCount>,
    mut log: ResMut<EventLog>,
    mut scouts: Query<(
        Entity,
        &mut Ship,
        &mut Transform,
        &mut ScoutBehavior,
        &mut ZoneId,
    )>,
    gates: Query<(Entity, &Transform, &JumpGate, &ZoneId), Without<ScoutBehavior>>,
    ore_nodes: Query<(Entity, &Transform, &ZoneId), (With<OreNode>, Without<ScoutBehavior>)>,
    stations: Query<(Entity, &Transform, &ZoneId), (With<Station>, Without<ScoutBehavior>)>,
    pirates: Query<(Entity, &Transform, &ZoneId), (With<PirateShip>, Without<ScoutBehavior>)>,
    pirate_bases: Query<(Entity, &Transform, &ZoneId), (With<PirateBase>, Without<ScoutBehavior>)>,
    other_ships: Query<
        (Entity, &Transform, &ZoneId),
        (With<Ship>, Without<ScoutBehavior>, Without<PlayerControl>),
    >,
    mut intel_query: Query<(&SystemNode, &mut SystemIntel)>,
) {
    use crate::fleets::{ContactType, IDENTIFY_RANGE};

    let delta_seconds = time.delta_secs();

    for (_scout_entity, mut ship, mut transform, mut behavior, mut zone_id) in scouts.iter_mut() {
        if matches!(ship.state, ShipState::Disabled) {
            continue;
        }

        match behavior.phase {
            ScoutPhase::Scanning => {
                ship.state = ShipState::Executing;

                // Advance scan timer
                let scan_completed = behavior.advance_scan(delta_seconds);

                if scan_completed {
                    info!("Scout: Scan complete in zone {}", zone_id.0);

                    // Mark zone as visited
                    behavior.mark_zone_visited(zone_id.0);

                    // Reveal intel for current zone
                    for (node, mut intel) in intel_query.iter_mut() {
                        if node.id == zone_id.0 && !intel.revealed {
                            intel.revealed = true;
                            intel.confidence = 0.8;
                            intel.last_seen_tick = ticks.tick;
                            intel.revealed_tick = ticks.tick;
                            if matches!(intel.layer, KnowledgeLayer::Existence) {
                                intel.layer = KnowledgeLayer::Geography;
                            }
                        }
                    }

                    // Discover gates (identified immediately for navigation)
                    for (gate_entity, gate_transform, gate, gate_zone) in gates.iter() {
                        if gate_zone.0 == zone_id.0 {
                            behavior.discover_gate(gate_entity, gate.destination_zone);
                            // Add gate as identified contact
                            let pos = Vec2::new(
                                gate_transform.translation.x,
                                gate_transform.translation.y,
                            );
                            behavior.contacts.push(crate::fleets::ScoutContact {
                                entity: gate_entity,
                                position: pos,
                                contact_type: ContactType::JumpGate,
                                status: crate::fleets::ContactStatus::Identified,
                            });
                        }
                    }

                    // Detect pirates (identified immediately - special case)
                    for (pirate_entity, pirate_transform, pirate_zone) in pirates.iter() {
                        if pirate_zone.0 == zone_id.0 {
                            let pos = Vec2::new(
                                pirate_transform.translation.x,
                                pirate_transform.translation.y,
                            );
                            behavior.add_pirate_contact(pirate_entity, pos, false);
                            info!(
                                "Scout: Detected pirate ship at ({:.0}, {:.0})",
                                pos.x, pos.y
                            );
                        }
                    }

                    // Detect pirate bases (identified immediately)
                    for (base_entity, base_transform, base_zone) in pirate_bases.iter() {
                        if base_zone.0 == zone_id.0 {
                            let pos = Vec2::new(
                                base_transform.translation.x,
                                base_transform.translation.y,
                            );
                            behavior.add_pirate_contact(base_entity, pos, true);
                            info!(
                                "Scout: Detected pirate base at ({:.0}, {:.0})",
                                pos.x, pos.y
                            );
                        }
                    }

                    // Add ore nodes as unidentified contacts
                    for (ore_entity, ore_transform, ore_zone) in ore_nodes.iter() {
                        if ore_zone.0 == zone_id.0 {
                            let pos =
                                Vec2::new(ore_transform.translation.x, ore_transform.translation.y);
                            behavior.add_contact(ore_entity, pos);
                        }
                    }

                    // Add stations as unidentified contacts
                    for (station_entity, station_transform, station_zone) in stations.iter() {
                        if station_zone.0 == zone_id.0 {
                            let pos = Vec2::new(
                                station_transform.translation.x,
                                station_transform.translation.y,
                            );
                            behavior.add_contact(station_entity, pos);
                        }
                    }

                    // Add other ships as unidentified (will be skipped during investigation)
                    for (ship_entity, ship_transform, ship_zone) in other_ships.iter() {
                        if ship_zone.0 == zone_id.0 {
                            let pos = Vec2::new(
                                ship_transform.translation.x,
                                ship_transform.translation.y,
                            );
                            behavior.add_contact(ship_entity, pos);
                            // Mark as ship type so it gets skipped
                            if let Some(contact) = behavior.contacts.last_mut() {
                                contact.contact_type = ContactType::Ship;
                                contact.status = crate::fleets::ContactStatus::Skipped;
                            }
                        }
                    }

                    // Log scan complete with pirate warning if applicable
                    if behavior.pirates_detected > 0 {
                        log.push(format!(
                            "Scout scan: Zone {} - {} pirates detected!",
                            zone_id.0, behavior.pirates_detected
                        ));
                    } else {
                        log.push(format!("Scout scan complete: Zone {}", zone_id.0));
                    }

                    // Transition to investigation phase
                    let unidentified_count = behavior
                        .contacts
                        .iter()
                        .filter(|c| matches!(c.status, crate::fleets::ContactStatus::Unidentified))
                        .count();
                    info!(
                        "Scout: Beginning investigation of {} contacts",
                        unidentified_count
                    );
                    behavior.begin_investigation();
                }
            }
            ScoutPhase::Investigating => {
                // Navigate to next unidentified contact
                if let Some(contact) = behavior.next_contact_to_investigate() {
                    let contact_entity = contact.entity;
                    let contact_pos = contact.position;

                    // Set target position
                    behavior.target_position = Some(contact_pos);
                    ship.state = ShipState::InTransit;

                    // Move toward contact
                    let scout_pos = Vec2::new(transform.translation.x, transform.translation.y);
                    let to_contact = contact_pos - scout_pos;
                    let distance = to_contact.length();

                    if distance <= IDENTIFY_RANGE {
                        // Within range - identify the contact
                        // Determine actual type based on entity queries
                        let actual_type = if ore_nodes.get(contact_entity).is_ok() {
                            ContactType::Asteroid
                        } else if stations.get(contact_entity).is_ok() {
                            ContactType::Station
                        } else {
                            ContactType::Unknown
                        };

                        behavior.identify_contact(contact_entity, actual_type);
                        info!("Scout: Identified contact as {:?}", actual_type);
                        behavior.target_position = None;
                    } else {
                        // Move toward contact
                        let direction = to_contact.normalize_or_zero();
                        let step = direction * SCOUT_SPEED * delta_seconds;
                        transform.translation.x += step.x;
                        transform.translation.y += step.y;
                    }
                } else {
                    // No more contacts to investigate - zone complete
                    info!("Scout: Zone {} investigation complete", zone_id.0);
                    behavior.complete_zone();
                }
            }
            ScoutPhase::ZoneComplete => {
                scout_zone_complete(&mut behavior, &mut ship, &zone_id, &gates);
            }
            ScoutPhase::TravelingToGate => {
                scout_travel_to_gate(
                    &mut behavior,
                    &mut ship,
                    &mut transform,
                    &gates,
                    delta_seconds,
                );
            }
            ScoutPhase::Jumping => {
                scout_process_jump(
                    &mut behavior,
                    &mut ship,
                    &mut zone_id,
                    delta_seconds,
                    &mut log,
                );
            }
            ScoutPhase::Complete => {
                ship.state = ShipState::Idle;
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn scout_travel_to_gate(
    behavior: &mut ScoutBehavior,
    ship: &mut Ship,
    transform: &mut Transform,
    gates: &Query<(Entity, &Transform, &JumpGate, &ZoneId), Without<ScoutBehavior>>,
    delta_seconds: f32,
) {
    let target_pos = match behavior.target_position {
        Some(pos) => pos,
        None => {
            // Lost target, go back to scanning
            behavior.phase = ScoutPhase::Scanning;
            behavior.target_gate = None;
            return;
        }
    };

    let current_pos = Vec2::new(transform.translation.x, transform.translation.y);
    let to_target = target_pos - current_pos;
    let distance = to_target.length();

    if distance <= SCOUT_GATE_RANGE {
        // Arrived at gate - start jump
        if let Some(target_gate) = behavior.target_gate {
            for (entity, _gate_transform, gate, _gate_zone) in gates.iter() {
                if entity == target_gate {
                    info!("Scout: Jumping to zone {}", gate.destination_zone);
                    behavior.start_jump(gate.destination_zone, JUMP_TRANSITION_SECONDS);
                    behavior.remove_gate(target_gate);
                    ship.state = ShipState::Executing;
                    return;
                }
            }
        }
        // Gate not found, go back to scanning
        behavior.phase = ScoutPhase::Scanning;
        behavior.target_gate = None;
        behavior.target_position = None;
    } else {
        // Move toward gate
        let direction = to_target.normalize_or_zero();
        let step = direction * SCOUT_SPEED * delta_seconds;
        transform.translation.x += step.x;
        transform.translation.y += step.y;
        ship.state = ShipState::InTransit;
    }
}

fn scout_process_jump(
    behavior: &mut ScoutBehavior,
    ship: &mut Ship,
    zone_id: &mut ZoneId,
    delta_seconds: f32,
    log: &mut EventLog,
) {
    behavior.jump_remaining_seconds -= delta_seconds;

    if behavior.jump_remaining_seconds <= 0.0 {
        let destination = behavior.jump_destination.unwrap_or(behavior.current_zone);
        behavior.complete_jump();
        // Update the actual ZoneId component
        zone_id.0 = behavior.current_zone;
        ship.state = ShipState::Executing;
        info!("Scout: Arrived at zone {}, beginning scan", destination);
        log.push(format!("Scout arrived at zone {}", destination));
        // Start scanning the new zone
        behavior.start_scan();
    } else {
        ship.state = ShipState::InTransit;
    }
}

fn scout_zone_complete(
    behavior: &mut ScoutBehavior,
    ship: &mut Ship,
    zone_id: &ZoneId,
    gates: &Query<(Entity, &Transform, &JumpGate, &ZoneId), Without<ScoutBehavior>>,
) {
    // Find next gate to explore
    if let Some((gate_entity, _dest_zone)) = behavior.next_gate_to_explore() {
        // Find the gate's position
        for (entity, gate_transform, gate, gate_zone) in gates.iter() {
            if entity == gate_entity && gate_zone.0 == zone_id.0 {
                info!(
                    "Scout: Traveling to gate (destination zone {})",
                    gate.destination_zone
                );
                behavior.target_gate = Some(gate_entity);
                behavior.target_position = Some(Vec2::new(
                    gate_transform.translation.x,
                    gate_transform.translation.y,
                ));
                behavior.phase = ScoutPhase::TravelingToGate;
                ship.state = ShipState::InTransit;
                return;
            }
        }
    }

    // No more gates to explore - exploration complete
    info!("Scout: Exploration complete - all reachable zones visited");
    behavior.phase = ScoutPhase::Complete;
    ship.state = ShipState::Idle;
}
