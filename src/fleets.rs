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
    /// Scanning resources and gates in current zone
    #[default]
    Scanning,
    /// Traveling to a jump gate
    TravelingToGate,
    /// Currently jumping through a gate
    Jumping,
    /// No more zones to explore
    Complete,
}

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
        }
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

    /// Start jumping through a gate
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
        find_path_to_unvisited_zone, next_risk, risk_threshold, scout_confidence, RiskTolerance,
        ScoutBehavior, ScoutPhase,
    };
    use bevy::prelude::Entity;
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
}
