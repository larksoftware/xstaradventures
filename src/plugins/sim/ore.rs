//! Asteroid field spawning and management systems.

use bevy::prelude::*;

use crate::compat::SpatialBundle;
use crate::ore::{Asteroid, OreKind, OreNode};
use crate::world::{SystemIntel, SystemNode, ZoneId, ZoneModifier};

// =============================================================================
// Resources
// =============================================================================

#[derive(Resource, Default)]
pub struct RevealedNodesTracker {
    pub spawned: std::collections::HashSet<u32>,
}

// =============================================================================
// Constants
// =============================================================================

// Zone radius where asteroid fields can spawn (5x scale)
const FIELD_MIN_RADIUS: f32 = 800.0;
const FIELD_MAX_RADIUS: f32 = 3500.0;

// Asteroid field cluster size
const CLUSTER_RADIUS: f32 = 300.0;
const MIN_ASTEROID_SPACING: f32 = 25.0;

// =============================================================================
// Systems
// =============================================================================

pub fn spawn_ore_at_revealed_nodes(
    mut commands: Commands,
    mut tracker: ResMut<RevealedNodesTracker>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    for (node, intel) in nodes.iter() {
        if intel.revealed && !tracker.spawned.contains(&node.id) {
            tracker.spawned.insert(node.id);

            let mut rng_state = node.id as u64;
            let is_starter = intel.revealed_tick == 0;

            // Determine number of asteroid field clusters (1-4)
            let field_count = field_count_for_zone(node.modifier, is_starter, &mut rng_state);

            for field_idx in 0..field_count {
                // Pick a cluster center point within the zone
                let field_angle = next_rng(&mut rng_state) * std::f32::consts::TAU;
                let field_radius = FIELD_MIN_RADIUS
                    + next_rng(&mut rng_state) * (FIELD_MAX_RADIUS - FIELD_MIN_RADIUS);
                let field_center = Vec2::new(
                    node.position.x + field_angle.cos() * field_radius,
                    node.position.y + field_angle.sin() * field_radius,
                );

                // Determine asteroids in this field
                let mineable_count =
                    mineable_count_for_field(node.modifier, is_starter, &mut rng_state);
                let decorative_count = 15 + (next_rng(&mut rng_state) * 25.0) as usize; // 15-40 decorative
                let total_asteroids = mineable_count + decorative_count;

                // Generate positions for all asteroids in the cluster
                let positions =
                    generate_cluster_positions(&mut rng_state, total_asteroids, CLUSTER_RADIUS);

                // Spawn asteroids - first `mineable_count` are mineable, rest decorative
                for (idx, offset) in positions.into_iter().enumerate() {
                    let pos = field_center + offset;
                    let is_mineable = idx < mineable_count;

                    if is_mineable {
                        // Mineable asteroid (OreNode)
                        let kind = if idx < (mineable_count * 7 / 10) {
                            OreKind::CommonOre
                        } else {
                            OreKind::FuelOre
                        };
                        let capacity = 20.0 + (idx as f32 * 5.0) + next_rng(&mut rng_state) * 10.0;
                        let kind_str = match kind {
                            OreKind::CommonOre => "Ore",
                            OreKind::FuelOre => "Fuel",
                        };

                        commands.spawn((
                            OreNode {
                                kind,
                                remaining: capacity,
                                capacity,
                                rate_per_second: 3.0,
                            },
                            ZoneId(node.id),
                            Name::new(format!("{}-{}-{}-{}", kind_str, node.id, field_idx, idx)),
                            SpatialBundle::from_transform(Transform::from_xyz(pos.x, pos.y, 0.3)),
                        ));
                    } else {
                        // Decorative asteroid
                        let size = 0.5 + next_rng(&mut rng_state) * 1.0; // 0.5 to 1.5

                        commands.spawn((
                            Asteroid { size },
                            ZoneId(node.id),
                            Name::new(format!("Rock-{}-{}-{}", node.id, field_idx, idx)),
                            SpatialBundle::from_transform(Transform::from_xyz(pos.x, pos.y, 0.25)),
                        ));
                    }
                }
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn next_rng(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let value = (*state >> 33) as u32;
    (value as f32) / (u32::MAX as f32)
}

/// Number of asteroid field clusters per zone
fn field_count_for_zone(modifier: Option<ZoneModifier>, is_starter: bool, rng: &mut u64) -> usize {
    let rand_val = next_rng(rng);

    let (min, max) = if is_starter {
        (2, 3) // Starter zone has 2-3 fields
    } else {
        match modifier {
            Some(ZoneModifier::RichOreVeins) => (3, 5), // Rich zones have more fields
            Some(ZoneModifier::HighRadiation) => (1, 2), // Radiation zones sparse
            Some(ZoneModifier::NebulaInterference) => (2, 3),
            Some(ZoneModifier::ClearSignals) => (2, 4),
            None => (1, 4),
        }
    };

    min + (rand_val * (max - min + 1) as f32) as usize
}

/// Number of mineable asteroids per field (0-10)
fn mineable_count_for_field(
    modifier: Option<ZoneModifier>,
    is_starter: bool,
    rng: &mut u64,
) -> usize {
    let rand_val = next_rng(rng);

    let (min, max) = if is_starter {
        (3, 6) // Starter fields have guaranteed mineable asteroids
    } else {
        match modifier {
            Some(ZoneModifier::RichOreVeins) => (5, 10), // Rich fields
            Some(ZoneModifier::HighRadiation) => (0, 3), // Sparse
            Some(ZoneModifier::NebulaInterference) => (2, 6),
            Some(ZoneModifier::ClearSignals) => (3, 7),
            None => (1, 8),
        }
    };

    min + (rand_val * (max - min + 1) as f32) as usize
}

/// Generate clustered positions that don't overlap
fn generate_cluster_positions(rng: &mut u64, count: usize, cluster_radius: f32) -> Vec<Vec2> {
    let mut positions = Vec::with_capacity(count);

    for _ in 0..count {
        // Try to find a non-overlapping position
        let mut attempts = 0;
        loop {
            let angle = next_rng(rng) * std::f32::consts::TAU;
            // Use gaussian-like distribution for natural clustering
            let r1 = next_rng(rng);
            let r2 = next_rng(rng);
            let dist = (r1 + r2) / 2.0 * cluster_radius; // Tends toward center

            let candidate = Vec2::new(angle.cos() * dist, angle.sin() * dist);

            // Check for overlap with existing positions
            let overlaps = positions
                .iter()
                .any(|p: &Vec2| p.distance(candidate) < MIN_ASTEROID_SPACING);

            if !overlaps || attempts > 20 {
                positions.push(candidate);
                break;
            }
            attempts += 1;
        }
    }

    positions
}
