use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub enum RiskTolerance {
    Cautious,
    #[default]
    Balanced,
    Bold,
}

/// Current phase of scout exploration
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub enum ScoutPhase {
    /// Actively scanning the zone (takes time)
    #[default]
    Scanning,
    /// Investigating unidentified contacts
    Investigating,
    /// Zone fully explored, ready to move to next
    ZoneComplete,
    /// Traveling to a jump gate
    TravelingToGate,
    /// Currently jumping through a gate
    Jumping,
    /// No more zones to explore
    Complete,
}

/// Contact status for scout investigation
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ContactStatus {
    /// Detected but not yet investigated by scout
    Pending,
    /// Investigated (scout got within range)
    Investigated,
    /// Skipped (e.g., ship type - potentially hostile)
    Skipped,
}

/// Type of contact detected by scout
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ContactType {
    Unknown,
    Asteroid,
    Station,
    JumpGate,
    Ship,
    PirateShip,
    PirateBase,
}

/// A contact discovered by the scout
#[derive(Clone, Debug)]
pub struct ScoutContact {
    pub entity: Entity,
    pub position: Vec2,
    pub contact_type: ContactType,
    pub status: ContactStatus,
}

impl ScoutContact {
    pub fn new_pending(entity: Entity, position: Vec2) -> Self {
        Self {
            entity,
            position,
            contact_type: ContactType::Unknown,
            status: ContactStatus::Pending,
        }
    }

    pub fn new_pirate(entity: Entity, position: Vec2, is_base: bool) -> Self {
        Self {
            entity,
            position,
            contact_type: if is_base {
                ContactType::PirateBase
            } else {
                ContactType::PirateShip
            },
            status: ContactStatus::Investigated,
        }
    }
}

/// Duration of a zone scan in seconds
pub const SCAN_DURATION_SECONDS: f32 = 5.0;

/// Distance required to identify a contact
pub const IDENTIFY_RANGE: f32 = 150.0;

#[derive(Component, Debug)]
pub struct ScoutBehavior {
    pub risk: RiskTolerance,
    /// The zone the scout is currently in
    pub current_zone: u32,
    /// Set of zones the scout has fully scanned
    pub visited_zones: HashSet<u32>,
    /// Gates leading to unvisited zones (stored as (gate_entity, destination_zone))
    pub gates_to_explore: Vec<(Entity, u32)>,
    /// Current exploration phase
    pub phase: ScoutPhase,
    /// Target gate entity when traveling to a gate
    pub target_gate: Option<Entity>,
    /// Target position within current zone (for movement)
    pub target_position: Option<Vec2>,
    /// Jump transition state
    pub jump_remaining_seconds: f32,
    pub jump_destination: Option<u32>,
    /// Remaining scan time in current zone
    pub scan_remaining_seconds: f32,
    /// Contacts discovered in current zone
    pub contacts: Vec<ScoutContact>,
    /// Index of current contact being investigated
    pub current_contact_index: usize,
    /// Number of pirates detected in current zone (for alerting)
    pub pirates_detected: u32,
}

#[allow(dead_code)]
pub fn risk_threshold(risk: RiskTolerance) -> f32 {
    match risk {
        RiskTolerance::Cautious => 0.35,
        RiskTolerance::Balanced => 0.6,
        RiskTolerance::Bold => 0.85,
    }
}

pub fn next_risk(risk: RiskTolerance, delta: i32) -> RiskTolerance {
    let order = [
        RiskTolerance::Cautious,
        RiskTolerance::Balanced,
        RiskTolerance::Bold,
    ];
    let mut index = 0;
    for (i, value) in order.iter().enumerate() {
        if *value == risk {
            index = i as i32;
            break;
        }
    }

    let next_index = (index + delta).clamp(0, (order.len() - 1) as i32) as usize;
    order[next_index]
}

#[allow(dead_code)]
pub fn scout_confidence(risk: RiskTolerance, route_risk: f32) -> f32 {
    let base = match risk {
        RiskTolerance::Cautious => 0.75,
        RiskTolerance::Balanced => 0.65,
        RiskTolerance::Bold => 0.55,
    };
    (base - (route_risk * 0.4)).clamp(0.2, 0.9)
}

impl ScoutBehavior {
    /// Create a new scout behavior starting in the given zone
    pub fn new(starting_zone: u32, risk: RiskTolerance) -> Self {
        let mut visited_zones = HashSet::new();
        visited_zones.insert(starting_zone);
        Self {
            risk,
            current_zone: starting_zone,
            visited_zones,
            gates_to_explore: Vec::new(),
            phase: ScoutPhase::Scanning,
            target_gate: None,
            target_position: None,
            jump_remaining_seconds: 0.0,
            jump_destination: None,
            scan_remaining_seconds: SCAN_DURATION_SECONDS,
            contacts: Vec::new(),
            current_contact_index: 0,
            pirates_detected: 0,
        }
    }

    /// Start scanning the current zone
    pub fn start_scan(&mut self) {
        self.phase = ScoutPhase::Scanning;
        self.scan_remaining_seconds = SCAN_DURATION_SECONDS;
        self.contacts.clear();
        self.current_contact_index = 0;
        self.pirates_detected = 0;
    }

    /// Advance the scan timer by delta_seconds, returns true if scan completed
    pub fn advance_scan(&mut self, delta_seconds: f32) -> bool {
        if self.scan_remaining_seconds > 0.0 {
            self.scan_remaining_seconds -= delta_seconds;
            if self.scan_remaining_seconds <= 0.0 {
                self.scan_remaining_seconds = 0.0;
                return true;
            }
        }
        false
    }

    /// Check if scan is complete
    #[allow(dead_code)]
    pub fn is_scan_complete(&self) -> bool {
        self.scan_remaining_seconds <= 0.0
    }

    /// Add an unidentified contact discovered during scan
    pub fn add_contact(&mut self, entity: Entity, position: Vec2) {
        self.contacts
            .push(ScoutContact::new_pending(entity, position));
    }

    /// Add a pirate contact (identified immediately during scan)
    pub fn add_pirate_contact(&mut self, entity: Entity, position: Vec2, is_base: bool) {
        self.contacts
            .push(ScoutContact::new_pirate(entity, position, is_base));
        self.pirates_detected += 1;
    }

    /// Get the next unidentified non-ship contact to investigate
    pub fn next_contact_to_investigate(&self) -> Option<&ScoutContact> {
        self.contacts.iter().find(|c| {
            matches!(c.status, ContactStatus::Pending)
                && !matches!(c.contact_type, ContactType::Ship | ContactType::PirateShip)
        })
    }

    /// Get mutable reference to contact by index
    #[allow(dead_code)]
    pub fn get_contact_mut(&mut self, index: usize) -> Option<&mut ScoutContact> {
        self.contacts.get_mut(index)
    }

    /// Find contact index by entity
    #[allow(dead_code)]
    pub fn find_contact_index(&self, entity: Entity) -> Option<usize> {
        self.contacts.iter().position(|c| c.entity == entity)
    }

    /// Identify a contact with its actual type
    pub fn identify_contact(&mut self, entity: Entity, contact_type: ContactType) {
        if let Some(contact) = self.contacts.iter_mut().find(|c| c.entity == entity) {
            contact.contact_type = contact_type;
            contact.status = ContactStatus::Investigated;
        }
    }

    /// Skip a contact (e.g., it's a ship type)
    #[allow(dead_code)]
    pub fn skip_contact(&mut self, entity: Entity) {
        if let Some(contact) = self.contacts.iter_mut().find(|c| c.entity == entity) {
            contact.status = ContactStatus::Skipped;
        }
    }

    /// Check if all contacts have been processed (identified or skipped)
    #[allow(dead_code)]
    pub fn all_contacts_processed(&self) -> bool {
        self.contacts.iter().all(|c| {
            matches!(
                c.status,
                ContactStatus::Investigated | ContactStatus::Skipped
            )
        })
    }

    /// Transition to investigation phase after scan completes
    pub fn begin_investigation(&mut self) {
        self.phase = ScoutPhase::Investigating;
        self.current_contact_index = 0;
    }

    /// Mark zone as complete and ready to move on
    pub fn complete_zone(&mut self) {
        self.phase = ScoutPhase::ZoneComplete;
    }

    /// Mark a zone as visited
    pub fn mark_zone_visited(&mut self, zone: u32) {
        self.visited_zones.insert(zone);
    }

    /// Check if a zone has been visited
    pub fn is_zone_visited(&self, zone: u32) -> bool {
        self.visited_zones.contains(&zone)
    }

    /// Add a gate to the exploration list if it leads to an unvisited zone
    pub fn discover_gate(&mut self, gate_entity: Entity, destination_zone: u32) {
        if !self.is_zone_visited(destination_zone) {
            // Check if gate is already in the list
            let already_known = self.gates_to_explore.iter().any(|(e, _)| *e == gate_entity);
            if !already_known {
                self.gates_to_explore.push((gate_entity, destination_zone));
            }
        }
    }

    /// Get the next gate to explore, if any
    pub fn next_gate_to_explore(&self) -> Option<(Entity, u32)> {
        self.gates_to_explore.first().copied()
    }

    /// Remove a gate from the exploration list (after using it or if destination was visited)
    pub fn remove_gate(&mut self, gate_entity: Entity) {
        self.gates_to_explore.retain(|(e, _)| *e != gate_entity);
    }

    /// Remove all gates leading to already-visited zones
    pub fn prune_visited_gates(&mut self) {
        let visited = &self.visited_zones;
        self.gates_to_explore
            .retain(|(_, dest)| !visited.contains(dest));
    }

    /// Check if exploration is complete (no more gates to explore)
    #[allow(dead_code)]
    pub fn is_exploration_complete(&self) -> bool {
        self.gates_to_explore.is_empty() && matches!(self.phase, ScoutPhase::Scanning)
    }

    /// Start jumping through a gate (used by tests, scouts use JumpTransition component)
    #[allow(dead_code)]
    pub fn start_jump(&mut self, destination_zone: u32, transition_seconds: f32) {
        self.phase = ScoutPhase::Jumping;
        self.jump_destination = Some(destination_zone);
        self.jump_remaining_seconds = transition_seconds;
        self.target_gate = None;
        self.target_position = None;
    }

    /// Complete jump and enter new zone
    pub fn complete_jump(&mut self) {
        if let Some(destination) = self.jump_destination.take() {
            self.current_zone = destination;
            self.mark_zone_visited(destination);
            self.phase = ScoutPhase::Scanning;
            self.jump_remaining_seconds = 0.0;
            self.prune_visited_gates();
        }
    }
}

/// Find a path through visited zones to reach a gate leading to an unvisited zone
#[allow(dead_code)]
pub fn find_path_to_unvisited_zone(
    current_zone: u32,
    visited_zones: &HashSet<u32>,
    zone_connections: &[(u32, u32)], // (from_zone, to_zone)
) -> Option<Vec<u32>> {
    // BFS to find path from current_zone to any zone with a gate to unvisited
    use std::collections::{HashMap, VecDeque};

    // Build adjacency list of visited zones
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
    for (from, to) in zone_connections {
        if visited_zones.contains(from) && visited_zones.contains(to) {
            adjacency.entry(*from).or_default().push(*to);
            adjacency.entry(*to).or_default().push(*from);
        }
    }

    // Find zones that have gates to unvisited zones
    let mut target_zones: HashSet<u32> = HashSet::new();
    for (from, to) in zone_connections {
        if visited_zones.contains(from) && !visited_zones.contains(to) {
            target_zones.insert(*from);
        }
        if visited_zones.contains(to) && !visited_zones.contains(from) {
            target_zones.insert(*to);
        }
    }

    if target_zones.is_empty() {
        return None;
    }

    if target_zones.contains(&current_zone) {
        return Some(vec![current_zone]);
    }

    // BFS
    let mut queue: VecDeque<u32> = VecDeque::new();
    let mut came_from: HashMap<u32, u32> = HashMap::new();
    queue.push_back(current_zone);
    came_from.insert(current_zone, current_zone);

    while let Some(zone) = queue.pop_front() {
        if target_zones.contains(&zone) {
            // Reconstruct path
            let mut path = vec![zone];
            let mut current = zone;
            while came_from.get(&current) != Some(&current) {
                current = match came_from.get(&current) {
                    Some(c) => *c,
                    None => break,
                };
                path.push(current);
            }
            path.reverse();
            return Some(path);
        }

        if let Some(neighbors) = adjacency.get(&zone) {
            for neighbor in neighbors {
                if !came_from.contains_key(neighbor) {
                    came_from.insert(*neighbor, zone);
                    queue.push_back(*neighbor);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        find_path_to_unvisited_zone, next_risk, risk_threshold, scout_confidence, ContactStatus,
        ContactType, RiskTolerance, ScoutBehavior, ScoutPhase, IDENTIFY_RANGE,
        SCAN_DURATION_SECONDS,
    };
    use bevy::prelude::{Entity, Vec2};
    use std::collections::HashSet;

    #[test]
    fn risk_threshold_orders_low_to_high() {
        assert!(risk_threshold(RiskTolerance::Cautious) < risk_threshold(RiskTolerance::Balanced));
        assert!(risk_threshold(RiskTolerance::Balanced) < risk_threshold(RiskTolerance::Bold));
    }

    #[test]
    fn next_risk_clamps_at_edges() {
        assert_eq!(
            next_risk(RiskTolerance::Cautious, -1),
            RiskTolerance::Cautious
        );
        assert_eq!(next_risk(RiskTolerance::Bold, 1), RiskTolerance::Bold);
    }

    #[test]
    fn scout_confidence_decreases_with_risk() {
        let low = scout_confidence(RiskTolerance::Cautious, 0.8);
        let high = scout_confidence(RiskTolerance::Bold, 0.8);
        assert!(low > high);
    }

    // --- New scout behavior tests ---

    #[test]
    fn scout_starts_in_scanning_phase() {
        let scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        assert_eq!(scout.phase, ScoutPhase::Scanning);
        assert_eq!(scout.current_zone, 100);
    }

    #[test]
    fn scout_marks_starting_zone_as_visited() {
        let scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        assert!(scout.is_zone_visited(100));
        assert!(!scout.is_zone_visited(200));
    }

    #[test]
    fn scout_discovers_gate_to_unvisited_zone() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate_entity = Entity::from_bits(1);

        scout.discover_gate(gate_entity, 200);

        assert_eq!(scout.gates_to_explore.len(), 1);
        assert_eq!(scout.gates_to_explore[0], (gate_entity, 200));
    }

    #[test]
    fn scout_ignores_gate_to_visited_zone() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate_entity = Entity::from_bits(1);

        // 100 is already visited (starting zone)
        scout.discover_gate(gate_entity, 100);

        assert!(scout.gates_to_explore.is_empty());
    }

    #[test]
    fn scout_does_not_add_duplicate_gates() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate_entity = Entity::from_bits(1);

        scout.discover_gate(gate_entity, 200);
        scout.discover_gate(gate_entity, 200);

        assert_eq!(scout.gates_to_explore.len(), 1);
    }

    #[test]
    fn scout_next_gate_returns_first_gate() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate1 = Entity::from_bits(1);
        let gate2 = Entity::from_bits(2);

        scout.discover_gate(gate1, 200);
        scout.discover_gate(gate2, 300);

        let next = scout.next_gate_to_explore();
        assert_eq!(next, Some((gate1, 200)));
    }

    #[test]
    fn scout_remove_gate_removes_correct_gate() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate1 = Entity::from_bits(1);
        let gate2 = Entity::from_bits(2);

        scout.discover_gate(gate1, 200);
        scout.discover_gate(gate2, 300);
        scout.remove_gate(gate1);

        assert_eq!(scout.gates_to_explore.len(), 1);
        assert_eq!(scout.gates_to_explore[0], (gate2, 300));
    }

    #[test]
    fn scout_prune_visited_gates_removes_visited_destinations() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate1 = Entity::from_bits(1);
        let gate2 = Entity::from_bits(2);

        scout.discover_gate(gate1, 200);
        scout.discover_gate(gate2, 300);

        // Mark 200 as visited
        scout.mark_zone_visited(200);
        scout.prune_visited_gates();

        assert_eq!(scout.gates_to_explore.len(), 1);
        assert_eq!(scout.gates_to_explore[0], (gate2, 300));
    }

    #[test]
    fn scout_is_complete_when_no_gates_and_scanning() {
        let scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        assert!(scout.is_exploration_complete());
    }

    #[test]
    fn scout_not_complete_when_gates_remain() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.discover_gate(Entity::from_bits(1), 200);
        assert!(!scout.is_exploration_complete());
    }

    #[test]
    fn scout_start_jump_changes_phase() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.start_jump(200, 1.5);

        assert_eq!(scout.phase, ScoutPhase::Jumping);
        assert_eq!(scout.jump_destination, Some(200));
        assert_eq!(scout.jump_remaining_seconds, 1.5);
    }

    #[test]
    fn scout_complete_jump_changes_zone() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.start_jump(200, 1.5);
        scout.complete_jump();

        assert_eq!(scout.current_zone, 200);
        assert_eq!(scout.phase, ScoutPhase::Scanning);
        assert!(scout.is_zone_visited(200));
    }

    #[test]
    fn scout_complete_jump_prunes_gates() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate1 = Entity::from_bits(1);
        let gate2 = Entity::from_bits(2);

        scout.discover_gate(gate1, 200);
        scout.discover_gate(gate2, 300);

        scout.start_jump(200, 1.5);
        scout.complete_jump();

        // Gate to 200 should be pruned since we're now in 200
        assert_eq!(scout.gates_to_explore.len(), 1);
        assert_eq!(scout.gates_to_explore[0], (gate2, 300));
    }

    #[test]
    fn scout_exploration_state_persists_across_zone_transitions() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let gate1 = Entity::from_bits(1);
        let gate2 = Entity::from_bits(2);
        let gate3 = Entity::from_bits(3);

        // Discover gates from zone 100
        scout.discover_gate(gate1, 200);
        scout.discover_gate(gate2, 300);

        // Jump to 200
        scout.start_jump(200, 1.5);
        scout.complete_jump();

        // Discover more gates from zone 200
        scout.discover_gate(gate3, 400);

        // Verify state persists
        assert!(scout.is_zone_visited(100));
        assert!(scout.is_zone_visited(200));
        assert!(!scout.is_zone_visited(300));
        assert!(!scout.is_zone_visited(400));

        // Gate to 300 and 400 should remain
        assert_eq!(scout.gates_to_explore.len(), 2);
    }

    #[test]
    fn find_path_returns_none_when_all_reachable_visited() {
        let visited: HashSet<u32> = [100, 200].into_iter().collect();
        let connections = vec![(100, 200)];

        let path = find_path_to_unvisited_zone(100, &visited, &connections);
        assert!(path.is_none());
    }

    #[test]
    fn find_path_returns_current_zone_when_has_unvisited_neighbor() {
        let visited: HashSet<u32> = [100].into_iter().collect();
        let connections = vec![(100, 200)];

        let path = find_path_to_unvisited_zone(100, &visited, &connections);
        assert_eq!(path, Some(vec![100]));
    }

    #[test]
    fn find_path_finds_path_through_visited_zones() {
        // Zone layout: 100 -- 200 -- 300 -- 400 (unvisited)
        let visited: HashSet<u32> = [100, 200, 300].into_iter().collect();
        let connections = vec![(100, 200), (200, 300), (300, 400)];

        let path = find_path_to_unvisited_zone(100, &visited, &connections);
        assert_eq!(path, Some(vec![100, 200, 300]));
    }

    #[test]
    fn find_path_handles_branching_topology() {
        // Zone layout:
        //      200
        //     /
        // 100
        //     \
        //      300 -- 400 (unvisited)
        let visited: HashSet<u32> = [100, 200, 300].into_iter().collect();
        let connections = vec![(100, 200), (100, 300), (300, 400)];

        let path = find_path_to_unvisited_zone(100, &visited, &connections);
        // Should find path to 300 (which has gate to unvisited 400)
        assert_eq!(path, Some(vec![100, 300]));
    }

    #[test]
    fn scout_stops_when_all_reachable_zones_visited() {
        let scout = ScoutBehavior::new(100, RiskTolerance::Balanced);

        // No gates discovered, exploration is complete
        assert!(scout.is_exploration_complete());
        assert_eq!(scout.phase, ScoutPhase::Scanning);
    }

    // --- Scanning phase tests ---

    #[test]
    fn scan_takes_measurable_time() {
        let scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        // Scan should take SCAN_DURATION_SECONDS, not be instant
        assert!(SCAN_DURATION_SECONDS > 0.0);
        assert_eq!(scout.scan_remaining_seconds, SCAN_DURATION_SECONDS);
        assert!(!scout.is_scan_complete());
    }

    #[test]
    fn scan_advances_with_time() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let initial = scout.scan_remaining_seconds;

        // Advance by 1 second
        let completed = scout.advance_scan(1.0);

        assert!(!completed);
        assert!(scout.scan_remaining_seconds < initial);
        assert_eq!(scout.scan_remaining_seconds, initial - 1.0);
    }

    #[test]
    fn scan_completes_after_duration() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);

        // Advance past the full scan duration
        let completed = scout.advance_scan(SCAN_DURATION_SECONDS + 1.0);

        assert!(completed);
        assert!(scout.is_scan_complete());
        assert_eq!(scout.scan_remaining_seconds, 0.0);
    }

    #[test]
    fn scan_reveals_contacts_as_pending() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let entity = Entity::from_bits(1);
        let pos = Vec2::new(100.0, 200.0);

        scout.add_contact(entity, pos);

        assert_eq!(scout.contacts.len(), 1);
        assert_eq!(scout.contacts[0].status, ContactStatus::Pending);
        assert_eq!(scout.contacts[0].contact_type, ContactType::Unknown);
    }

    #[test]
    fn scan_detects_pirates_immediately() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let pirate_entity = Entity::from_bits(1);
        let pos = Vec2::new(100.0, 200.0);

        scout.add_pirate_contact(pirate_entity, pos, false);

        assert_eq!(scout.contacts.len(), 1);
        assert_eq!(scout.contacts[0].status, ContactStatus::Investigated);
        assert_eq!(scout.contacts[0].contact_type, ContactType::PirateShip);
        assert_eq!(scout.pirates_detected, 1);
    }

    #[test]
    fn scan_detects_pirate_bases_immediately() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let base_entity = Entity::from_bits(1);
        let pos = Vec2::new(100.0, 200.0);

        scout.add_pirate_contact(base_entity, pos, true);

        assert_eq!(scout.contacts[0].contact_type, ContactType::PirateBase);
        assert_eq!(scout.pirates_detected, 1);
    }

    // --- Investigation phase tests ---

    #[test]
    fn scout_transitions_to_investigating_after_scan() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.add_contact(Entity::from_bits(1), Vec2::new(100.0, 100.0));

        // Complete scan
        scout.advance_scan(SCAN_DURATION_SECONDS + 1.0);
        scout.begin_investigation();

        assert_eq!(scout.phase, ScoutPhase::Investigating);
    }

    #[test]
    fn scout_finds_next_unidentified_contact() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let entity1 = Entity::from_bits(1);
        let entity2 = Entity::from_bits(2);

        scout.add_contact(entity1, Vec2::new(100.0, 100.0));
        scout.add_contact(entity2, Vec2::new(200.0, 200.0));

        let next = scout.next_contact_to_investigate();
        assert!(next.is_some());
        assert_eq!(next.unwrap().entity, entity1);
    }

    #[test]
    fn contact_identified_updates_status() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let entity = Entity::from_bits(1);

        scout.add_contact(entity, Vec2::new(100.0, 100.0));
        scout.identify_contact(entity, ContactType::Asteroid);

        assert_eq!(scout.contacts[0].status, ContactStatus::Investigated);
        assert_eq!(scout.contacts[0].contact_type, ContactType::Asteroid);
    }

    #[test]
    fn scout_skips_ship_contacts() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let ship_entity = Entity::from_bits(1);
        let asteroid_entity = Entity::from_bits(2);

        // Add contacts - ship should be skipped
        scout.add_contact(ship_entity, Vec2::new(100.0, 100.0));
        scout.contacts[0].contact_type = ContactType::Ship;
        scout.add_contact(asteroid_entity, Vec2::new(200.0, 200.0));

        // Next contact to investigate should skip the ship
        let next = scout.next_contact_to_investigate();
        assert!(next.is_some());
        assert_eq!(next.unwrap().entity, asteroid_entity);
    }

    #[test]
    fn scout_moves_to_next_contact_after_identification() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        let entity1 = Entity::from_bits(1);
        let entity2 = Entity::from_bits(2);

        scout.add_contact(entity1, Vec2::new(100.0, 100.0));
        scout.add_contact(entity2, Vec2::new(200.0, 200.0));

        // Identify first contact
        scout.identify_contact(entity1, ContactType::Asteroid);

        // Next should be the second contact
        let next = scout.next_contact_to_investigate();
        assert!(next.is_some());
        assert_eq!(next.unwrap().entity, entity2);
    }

    #[test]
    fn identify_range_is_150_units() {
        assert_eq!(IDENTIFY_RANGE, 150.0);
    }

    // --- Zone completion tests ---

    #[test]
    fn zone_not_complete_until_all_contacts_processed() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.add_contact(Entity::from_bits(1), Vec2::new(100.0, 100.0));
        scout.add_contact(Entity::from_bits(2), Vec2::new(200.0, 200.0));

        assert!(!scout.all_contacts_processed());

        // Identify first, still not complete
        scout.identify_contact(Entity::from_bits(1), ContactType::Asteroid);
        assert!(!scout.all_contacts_processed());

        // Identify second, now complete
        scout.identify_contact(Entity::from_bits(2), ContactType::Station);
        assert!(scout.all_contacts_processed());
    }

    #[test]
    fn zone_complete_with_skipped_contacts() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.add_contact(Entity::from_bits(1), Vec2::new(100.0, 100.0));
        scout.add_contact(Entity::from_bits(2), Vec2::new(200.0, 200.0));

        // Identify one, skip the other (e.g., it's a ship)
        scout.identify_contact(Entity::from_bits(1), ContactType::Asteroid);
        scout.skip_contact(Entity::from_bits(2));

        assert!(scout.all_contacts_processed());
    }

    #[test]
    fn scout_transitions_to_zone_complete() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.phase = ScoutPhase::Investigating;

        scout.complete_zone();

        assert_eq!(scout.phase, ScoutPhase::ZoneComplete);
    }

    #[test]
    fn scout_only_jumps_after_zone_complete() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);
        scout.discover_gate(Entity::from_bits(1), 200);

        // Before zone complete, phase should not be TravelingToGate
        assert_eq!(scout.phase, ScoutPhase::Scanning);

        // After marking zone complete, scout can proceed
        scout.complete_zone();
        assert_eq!(scout.phase, ScoutPhase::ZoneComplete);
    }

    // --- Pirate alerting tests ---

    #[test]
    fn pirate_detection_increments_count() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);

        scout.add_pirate_contact(Entity::from_bits(1), Vec2::new(100.0, 100.0), false);
        scout.add_pirate_contact(Entity::from_bits(2), Vec2::new(200.0, 200.0), false);
        scout.add_pirate_contact(Entity::from_bits(3), Vec2::new(300.0, 300.0), true);

        assert_eq!(scout.pirates_detected, 3);
    }

    #[test]
    fn pirate_count_resets_on_new_scan() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);

        scout.add_pirate_contact(Entity::from_bits(1), Vec2::new(100.0, 100.0), false);
        assert_eq!(scout.pirates_detected, 1);

        // Start new scan (e.g., after entering new zone)
        scout.start_scan();

        assert_eq!(scout.pirates_detected, 0);
        assert!(scout.contacts.is_empty());
    }

    #[test]
    fn scout_start_scan_resets_state() {
        let mut scout = ScoutBehavior::new(100, RiskTolerance::Balanced);

        // Add some state
        scout.add_contact(Entity::from_bits(1), Vec2::new(100.0, 100.0));
        scout.advance_scan(2.0);
        scout.pirates_detected = 2;

        // Start fresh scan
        scout.start_scan();

        assert_eq!(scout.phase, ScoutPhase::Scanning);
        assert_eq!(scout.scan_remaining_seconds, SCAN_DURATION_SECONDS);
        assert!(scout.contacts.is_empty());
        assert_eq!(scout.current_contact_index, 0);
        assert_eq!(scout.pirates_detected, 0);
    }
}
