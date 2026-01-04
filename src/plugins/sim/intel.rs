//! Intel decay and management systems.

use bevy::prelude::*;

use crate::plugins::core::FogConfig;
use crate::world::{zone_modifier_effect, KnowledgeLayer, Sector, SystemIntel};

use super::SimTickCount;

// =============================================================================
// Systems
// =============================================================================

pub fn decay_intel(
    ticks: Res<SimTickCount>,
    config: Res<FogConfig>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    for mut intel in intel_query.iter_mut() {
        let age = ticks.tick.saturating_sub(intel.last_seen_tick);
        let base_decay = match intel.layer {
            KnowledgeLayer::Existence => config.decay_existence,
            KnowledgeLayer::Geography => config.decay_geography,
            KnowledgeLayer::Resources => config.decay_resources,
            KnowledgeLayer::Threats => config.decay_threats,
            KnowledgeLayer::Stability => config.decay_stability,
        };
        let age_factor = (age as f32 / 1000.0).clamp(0.0, 1.0);
        let decay = base_decay * (1.0 + age_factor);

        if intel.confidence > decay {
            intel.confidence -= decay;
        } else {
            intel.confidence = 0.0;
        }
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

pub fn refresh_intel(intel: &mut SystemIntel, tick: u64) {
    intel.last_seen_tick = tick;
    intel.confidence = 1.0;
}

pub fn advance_intel_layer(intel: &mut SystemIntel) {
    intel.layer = match intel.layer {
        KnowledgeLayer::Existence => KnowledgeLayer::Geography,
        KnowledgeLayer::Geography => KnowledgeLayer::Resources,
        KnowledgeLayer::Resources => KnowledgeLayer::Threats,
        KnowledgeLayer::Threats => KnowledgeLayer::Stability,
        KnowledgeLayer::Stability => KnowledgeLayer::Stability,
    };
}

pub fn zone_modifier_risk(sector: &Sector) -> f32 {
    if sector.nodes.is_empty() {
        return 0.0;
    }

    let total = sector
        .nodes
        .iter()
        .map(|node| {
            let effect = zone_modifier_effect(node.modifier);
            effect.fuel_risk + effect.confidence_risk + effect.pirate_risk
        })
        .sum::<f32>();

    total / (sector.nodes.len() as f32)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_intel_layer_stops_at_stability() {
        let mut intel = SystemIntel {
            layer: KnowledgeLayer::Threats,
            confidence: 0.5,
            last_seen_tick: 0,
            revealed: false,
            revealed_tick: 0,
        };

        advance_intel_layer(&mut intel);
        assert_eq!(intel.layer, KnowledgeLayer::Stability);
        advance_intel_layer(&mut intel);
        assert_eq!(intel.layer, KnowledgeLayer::Stability);
    }

    #[test]
    fn refresh_intel_sets_confidence_and_tick() {
        let mut intel = SystemIntel {
            layer: KnowledgeLayer::Existence,
            confidence: 0.2,
            last_seen_tick: 5,
            revealed: false,
            revealed_tick: 0,
        };

        refresh_intel(&mut intel, 42);
        assert_eq!(intel.last_seen_tick, 42);
        assert_eq!(intel.confidence, 1.0);
    }

    #[test]
    fn zone_modifier_risk_empty_sector_is_zero() {
        let sector = Sector::default();
        let risk = zone_modifier_risk(&sector);
        assert_eq!(risk, 0.0);
    }
}
