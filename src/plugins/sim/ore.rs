//! Ore node spawning and management systems.

use bevy::prelude::*;

use crate::compat::SpatialBundle;
use crate::ore::{OreKind, OreNode};
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

const ORE_MIN_RADIUS: f32 = 400.0;
const ORE_MAX_RADIUS: f32 = 800.0;

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
            let ore_count = ore_count_for_zone(node.modifier, is_starter, &mut rng_state);

            for index in 0..ore_count {
                let angle = next_unit_ore_rng(&mut rng_state) * std::f32::consts::TAU;
                let radius = ORE_MIN_RADIUS
                    + next_unit_ore_rng(&mut rng_state) * (ORE_MAX_RADIUS - ORE_MIN_RADIUS);

                let offset_x = angle.cos() * radius;
                let offset_y = angle.sin() * radius;

                let common_ore_count = (ore_count as f32 * 0.7) as usize;
                let kind = if index < common_ore_count {
                    OreKind::CommonOre
                } else {
                    OreKind::FuelOre
                };

                let capacity = 20.0 + (index as f32 * 6.0) + ((node.id as f32) * 0.01);
                let kind_str = match kind {
                    OreKind::CommonOre => "OreNode",
                    OreKind::FuelOre => "FuelNode",
                };

                commands.spawn((
                    OreNode {
                        kind,
                        remaining: capacity,
                        capacity,
                        rate_per_second: 3.0,
                    },
                    ZoneId(node.id),
                    Name::new(format!("{}-{}-{}", kind_str, node.id, index + 1)),
                    SpatialBundle::from_transform(Transform::from_xyz(
                        node.position.x + offset_x,
                        node.position.y + offset_y,
                        0.3,
                    )),
                ));
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn next_unit_ore_rng(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let value = (*state >> 33) as u32;
    (value as f32) / (u32::MAX as f32)
}

fn ore_count_for_zone(modifier: Option<ZoneModifier>, is_starter: bool, rng: &mut u64) -> usize {
    let rand_val = next_unit_ore_rng(rng);

    // Base range 0-20, with modifiers shifting the distribution
    let (min, max) = if is_starter {
        (5, 10) // Starter has guaranteed asteroids for learning
    } else {
        match modifier {
            Some(ZoneModifier::RichOreVeins) => (12, 20), // Rich zones favor high end
            Some(ZoneModifier::HighRadiation) => (0, 8),  // Radiation zones are sparse
            Some(ZoneModifier::NebulaInterference) => (4, 14),
            Some(ZoneModifier::ClearSignals) => (6, 16),
            None => (0, 20), // Full range for unmodified zones
        }
    };

    min + (rand_val * (max - min + 1) as f32) as usize
}
