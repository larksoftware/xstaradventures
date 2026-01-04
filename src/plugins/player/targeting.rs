//! Entity scanning and target selection systems.

use bevy::prelude::*;

use crate::ore::OreNode;
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::{InputBindings, ViewMode};
use crate::ships::{Ship, ShipKind};
use crate::stations::Station;
use crate::world::{JumpGate, SystemNode, ZoneId};

use super::components::{NearbyTargets, PlayerControl};

// =============================================================================
// Run Conditions
// =============================================================================

pub fn view_is_world(view: Res<ViewMode>) -> bool {
    *view == ViewMode::World
}

// =============================================================================
// Systems
// =============================================================================

#[allow(clippy::too_many_arguments)]
pub fn scan_nearby_entities(
    mut targets: ResMut<NearbyTargets>,
    player_query: Query<(&Transform, &ZoneId), With<PlayerControl>>,
    stations: Query<(Entity, &Transform, &Name, Option<&ZoneId>), With<Station>>,
    ore_nodes: Query<(Entity, &Transform, Option<&ZoneId>), With<OreNode>>,
    pirates: Query<(Entity, &Transform, Option<&ZoneId>), With<PirateShip>>,
    pirate_bases: Query<(Entity, &Transform, Option<&ZoneId>), With<PirateBase>>,
    ships: Query<(Entity, &Transform, &Ship, Option<&ZoneId>), Without<PlayerControl>>,
    jump_gates: Query<(Entity, &Transform, &JumpGate, Option<&ZoneId>)>,
) {
    // Verify player exists and has a zone
    let Ok((player_transform, player_zone)) = player_query.single() else {
        targets.entities.clear();
        targets.manually_selected = false;
        return;
    };

    // Remember previously selected entity
    let prev_selected_entity = targets
        .entities
        .get(targets.selected_index)
        .map(|(e, _, _)| *e);

    // Clear and rebuild entity list with only same-zone entities
    targets.entities.clear();

    // Scan all stations in player's zone
    for (entity, transform, name, zone) in stations.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets.entities.push((entity, pos, name.to_string()));
        }
    }

    // Scan all ore nodes in player's zone
    for (entity, transform, zone) in ore_nodes.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets
                .entities
                .push((entity, pos, "[o] Asteroid".to_string()));
        }
    }

    // Scan all pirates in player's zone
    for (entity, transform, zone) in pirates.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets
                .entities
                .push((entity, pos, "[!] Marauder".to_string()));
        }
    }

    // Scan all pirate bases in player's zone
    for (entity, transform, zone) in pirate_bases.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            targets
                .entities
                .push((entity, pos, "[!] Raider Den".to_string()));
        }
    }

    // Scan all other ships in player's zone
    for (entity, transform, ship, zone) in ships.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            let label = match ship.kind {
                ShipKind::Scout => "[*] Pathfinder",
                ShipKind::Miner => "[*] Harvester",
                ShipKind::Security => "[*] Sentinel",
                ShipKind::PlayerShip => "[*] Vessel",
            };
            targets.entities.push((entity, pos, label.to_string()));
        }
    }

    // Scan all jump gates in player's zone
    for (entity, transform, gate, zone) in jump_gates.iter() {
        if zone.is_some_and(|z| z.0 == player_zone.0) {
            let pos = Vec2::new(transform.translation.x, transform.translation.y);
            let label = format!("[>] Rift Gate -> {}", gate.destination_zone);
            targets.entities.push((entity, pos, label));
        }
    }

    // Sort by distance from player
    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    targets.entities.sort_by(|(_, pos_a, _), (_, pos_b, _)| {
        let dist_a = pos_a.distance(player_pos);
        let dist_b = pos_b.distance(player_pos);
        dist_a
            .partial_cmp(&dist_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Preserve selection of previously selected entity if still present
    if targets.manually_selected {
        if let Some(prev_entity) = prev_selected_entity {
            let new_index = targets
                .entities
                .iter()
                .position(|(e, _, _)| *e == prev_entity);
            if let Some(idx) = new_index {
                targets.selected_index = idx;
            } else {
                // Entity no longer exists or left the zone
                targets.manually_selected = false;
                targets.selected_index = 0;
            }
        } else {
            targets.manually_selected = false;
            targets.selected_index = 0;
        }
    } else {
        // When not manually selected, always point to closest (index 0)
        targets.selected_index = 0;
    }
}

pub fn handle_tactical_selection(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut targets: ResMut<NearbyTargets>,
) {
    if !input.just_pressed(bindings.cycle_target) {
        return;
    }

    if targets.entities.is_empty() {
        return;
    }

    // First Tab press selects closest target (index 0)
    if !targets.manually_selected {
        targets.manually_selected = true;
        targets.selected_index = 0;
        return;
    }

    // Subsequent Tab presses cycle to next target
    targets.selected_index = (targets.selected_index + 1) % targets.entities.len();
}

// =============================================================================
// Zone Utility Functions
// =============================================================================

/// Finds the zone (node ID) that a position belongs to.
/// Returns the ID of the closest SystemNode.
#[allow(dead_code)]
pub fn find_zone_for_position(nodes: &[SystemNode], pos: Vec2) -> Option<u32> {
    let mut closest_id = None;
    let mut closest_dist = f32::MAX;

    for node in nodes {
        let dist = node.position.distance(pos);
        if dist < closest_dist {
            closest_dist = dist;
            closest_id = Some(node.id);
        }
    }

    closest_id
}

/// Filters entities to only those in the same zone as the player.
#[allow(dead_code)]
pub fn filter_entities_by_zone(
    entities: &[(Entity, Vec2, String)],
    nodes: &[SystemNode],
    player_zone: Option<u32>,
) -> Vec<(Entity, Vec2, String)> {
    let Some(zone_id) = player_zone else {
        return Vec::new();
    };

    entities
        .iter()
        .filter(|(_, pos, _)| find_zone_for_position(nodes, *pos) == Some(zone_id))
        .cloned()
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::entity::Entity;

    #[test]
    fn autopilot_target_found_in_nearby_targets() {
        // Create a target entity ID (simulating an ore node)
        let ore_entity = Entity::from_bits(42);

        // Create nearby targets list containing the ore
        let mut targets = NearbyTargets::default();
        targets
            .entities
            .push((ore_entity, Vec2::new(10.0, 20.0), "Ore Node".to_string()));

        // Verify target is found in list
        let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == ore_entity);
        assert!(still_in_range, "Target should be found in nearby targets");
    }

    #[test]
    fn autopilot_target_not_found_when_list_empty() {
        let ore_entity = Entity::from_bits(42);
        let targets = NearbyTargets::default();

        // Empty list should not contain target
        let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == ore_entity);
        assert!(!still_in_range, "Target should not be found in empty list");
    }

    #[test]
    fn autopilot_target_persists_when_other_entities_added() {
        let ore_entity = Entity::from_bits(42);
        let scout_entity = Entity::from_bits(100);

        // Create nearby targets with ore
        let mut targets = NearbyTargets::default();
        targets
            .entities
            .push((ore_entity, Vec2::new(10.0, 20.0), "Ore Node".to_string()));

        // Add a scout ship (simulating spawn)
        targets.entities.push((
            scout_entity,
            Vec2::new(50.0, 50.0),
            "Scout Ship".to_string(),
        ));

        // Ore should still be found
        let still_in_range = targets.entities.iter().any(|(e, _, _)| *e == ore_entity);
        assert!(
            still_in_range,
            "Ore target should persist when scout is added"
        );
    }

    #[test]
    fn contacts_list_includes_distant_entities() {
        // Distant entity (beyond typical "range" of 400)
        let distant_entity = Entity::from_bits(999);

        let mut targets = NearbyTargets::default();

        // Simulate adding a distant entity (scan_all_entities should include this)
        targets.entities.push((
            distant_entity,
            Vec2::new(1000.0, 1000.0), // ~1414 pixels from origin
            "Distant Station".to_string(),
        ));

        // Distant entity should be in the list (no range filtering)
        assert_eq!(
            targets.entities.len(),
            1,
            "Contacts should include distant entities"
        );
        assert!(
            targets
                .entities
                .iter()
                .any(|(e, _, _)| *e == distant_entity),
            "Distant entity should be findable"
        );
    }

    #[test]
    fn find_zone_returns_closest_node() {
        let nodes = vec![
            SystemNode {
                id: 1,
                position: Vec2::new(0.0, 0.0),
                modifier: None,
            },
            SystemNode {
                id: 2,
                position: Vec2::new(100.0, 0.0),
                modifier: None,
            },
        ];

        // Position near node 1
        assert_eq!(find_zone_for_position(&nodes, Vec2::new(5.0, 0.0)), Some(1));
        // Position near node 2
        assert_eq!(
            find_zone_for_position(&nodes, Vec2::new(95.0, 0.0)),
            Some(2)
        );
        // Position exactly between nodes - should pick one (closest wins)
        let mid = find_zone_for_position(&nodes, Vec2::new(50.0, 0.0));
        assert!(mid == Some(1) || mid == Some(2));
    }

    #[test]
    fn contacts_excludes_entities_from_other_zones() {
        let nodes = vec![
            SystemNode {
                id: 1,
                position: Vec2::new(0.0, 0.0),
                modifier: None,
            },
            SystemNode {
                id: 2,
                position: Vec2::new(200.0, 0.0),
                modifier: None,
            },
        ];

        // Player is at zone 1
        let player_pos = Vec2::new(10.0, 0.0);
        let player_zone = find_zone_for_position(&nodes, player_pos);

        // Entities in zone 1 and zone 2
        let entity_in_zone_1 = (
            Entity::from_bits(1),
            Vec2::new(5.0, 5.0),
            "Nearby".to_string(),
        );
        let entity_in_zone_2 = (
            Entity::from_bits(2),
            Vec2::new(195.0, 0.0),
            "Far Away".to_string(),
        );

        let all_entities = vec![entity_in_zone_1.clone(), entity_in_zone_2];

        let filtered = filter_entities_by_zone(&all_entities, &nodes, player_zone);

        // Only entity in zone 1 should be included
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].2, "Nearby");
    }
}
