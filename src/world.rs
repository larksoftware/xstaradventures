use bevy::prelude::*;

/// Identifies which zone an entity belongs to.
/// Each node in the sector is a zone, identified by its node ID.
/// Entities can only be in one zone at a time.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZoneId(pub u32);

/// A jump gate that connects two zones.
/// Ships can use gates to travel between zones.
#[derive(Component, Clone, Copy, Debug)]
pub struct JumpGate {
    /// The zone this gate is located in
    #[allow(dead_code)]
    pub source_zone: u32,
    /// The zone this gate leads to
    pub destination_zone: u32,
}

/// Fuel cost to use a jump gate (constant for MVP)
pub const JUMP_GATE_FUEL_COST: f32 = 5.0;

/// Duration of jump transition in seconds
pub const JUMP_TRANSITION_SECONDS: f32 = 1.5;

/// Tracks an entity currently transitioning through a jump gate.
#[derive(Component, Clone, Debug)]
pub struct JumpTransition {
    /// Zone the entity is jumping to
    pub destination_zone: u32,
    /// Remaining transition time in seconds
    pub remaining_seconds: f32,
}

/// Marker for entities that have been identified by player or scouts.
/// Unidentified entities show as "Unknown Contact" in the contacts list.
#[derive(Component, Clone, Debug, Default)]
pub struct Identified;

#[derive(Resource, Default)]
pub struct Sector {
    pub nodes: Vec<SystemNode>,
    pub routes: Vec<RouteEdge>,
}

#[derive(Component, Clone)]
pub struct SystemNode {
    pub id: u32,
    pub position: Vec2,
    pub modifier: Option<ZoneModifier>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteEdge {
    pub from: u32,
    pub to: u32,
    pub distance: f32,
    pub risk: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KnowledgeLayer {
    Existence,
    Geography,
    Resources,
    Threats,
    Stability,
}

#[derive(Component, Clone, Debug)]
pub struct SystemIntel {
    pub layer: KnowledgeLayer,
    pub confidence: f32,
    pub last_seen_tick: u64,
    pub revealed: bool,
    pub revealed_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ZoneModifier {
    HighRadiation,
    NebulaInterference,
    RichOreVeins,
    ClearSignals,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ZoneModifierEffect {
    pub fuel_risk: f32,
    pub confidence_risk: f32,
    pub pirate_risk: f32,
}

pub fn zone_modifier_effect(modifier: Option<ZoneModifier>) -> ZoneModifierEffect {
    match modifier {
        Some(ZoneModifier::HighRadiation) => ZoneModifierEffect {
            fuel_risk: 0.2,
            confidence_risk: 0.05,
            pirate_risk: 0.1,
        },
        Some(ZoneModifier::NebulaInterference) => ZoneModifierEffect {
            fuel_risk: 0.0,
            confidence_risk: 0.2,
            pirate_risk: 0.1,
        },
        Some(ZoneModifier::RichOreVeins) => ZoneModifierEffect {
            fuel_risk: 0.0,
            confidence_risk: 0.0,
            pirate_risk: 0.2,
        },
        Some(ZoneModifier::ClearSignals) => ZoneModifierEffect {
            fuel_risk: 0.0,
            confidence_risk: -0.15,
            pirate_risk: 0.0,
        },
        None => ZoneModifierEffect::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::zone_modifier_effect;

    #[test]
    fn zone_modifier_none_is_default() {
        let effect = zone_modifier_effect(None);
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.0);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_high_radiation_values() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert_eq!(effect.fuel_risk, 0.2);
        assert_eq!(effect.confidence_risk, 0.05);
        assert_eq!(effect.pirate_risk, 0.1);
    }

    #[test]
    fn zone_modifier_clear_signals_reduces_confidence_risk() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, -0.15);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_nebula_values() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.2);
        assert_eq!(effect.pirate_risk, 0.1);
    }

    #[test]
    fn zone_modifier_rich_ore_values() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::RichOreVeins));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.0);
        assert_eq!(effect.pirate_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effect_all_non_nan() {
        let modifiers = [
            None,
            Some(super::ZoneModifier::HighRadiation),
            Some(super::ZoneModifier::NebulaInterference),
            Some(super::ZoneModifier::RichOreVeins),
            Some(super::ZoneModifier::ClearSignals),
        ];

        for modifier in modifiers {
            let effect = zone_modifier_effect(modifier);
            assert!(!effect.fuel_risk.is_nan());
            assert!(!effect.confidence_risk.is_nan());
            assert!(!effect.pirate_risk.is_nan());
        }
    }

    #[test]
    fn zone_modifier_effect_clear_signals_negative_confidence_only() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert!(effect.fuel_risk >= 0.0);
        assert!(effect.pirate_risk >= 0.0);
        assert!(effect.confidence_risk < 0.0);
    }

    #[test]
    fn zone_modifier_effect_radiation_increases_fuel_and_confidence() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert!(effect.fuel_risk > 0.0);
        assert!(effect.confidence_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effect_rich_ore_increases_pirate_risk() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::RichOreVeins));
        assert!(effect.pirate_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effect_nebula_increases_confidence_risk() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert!(effect.confidence_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effect_radiation_increases_fuel_risk() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert!(effect.fuel_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_only_negative_confidence() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert!(effect.confidence_risk < 0.0);
        assert!(effect.fuel_risk >= 0.0);
        assert!(effect.pirate_risk >= 0.0);
    }

    #[test]
    fn zone_modifier_effects_no_modifier_all_zero() {
        let effect = zone_modifier_effect(None);
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.0);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_nebula_no_fuel_risk() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.fuel_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_rich_ore_no_fuel_or_confidence_risk() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::RichOreVeins));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_radiation_no_negative_risks() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert!(effect.fuel_risk >= 0.0);
        assert!(effect.confidence_risk >= 0.0);
        assert!(effect.pirate_risk >= 0.0);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_negative_confidence_only() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, -0.15);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_increases_all_three() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert!(effect.fuel_risk > 0.0);
        assert!(effect.confidence_risk > 0.0);
        assert!(effect.pirate_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effects_rich_ore_pirate_only_positive() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::RichOreVeins));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.0);
        assert!(effect.pirate_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_matches_constants() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, -0.15);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_nebula_matches_constants() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.2);
        assert_eq!(effect.pirate_risk, 0.1);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_pirate_zero() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_fuel_zero() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert_eq!(effect.fuel_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_nebula_confidence_equals_point_two() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.confidence_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effects_nebula_confidence_equals_point_two_again() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.confidence_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effects_nebula_fuel_zero_again() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.fuel_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_pirate_equals_point_one() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert_eq!(effect.pirate_risk, 0.1);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_confidence_equals_point_one() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert_eq!(effect.confidence_risk, 0.05);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_confidence_equals_neg_point_one_five() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert_eq!(effect.confidence_risk, -0.15);
    }

    #[test]
    fn zone_modifier_effects_nebula_pirate_equals_point_one() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert_eq!(effect.pirate_risk, 0.1);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_fuel_equals_point_two() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert_eq!(effect.fuel_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_fuel_equals_point_two_again() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert_eq!(effect.fuel_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_confidence_negative_only_again() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert!(effect.confidence_risk < 0.0);
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_nebula_pirate_positive_again() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert!(effect.pirate_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_fuel_positive_again() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert!(effect.fuel_risk > 0.0);
    }

    #[test]
    fn zone_modifier_effects_rich_ore_matches_constants() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::RichOreVeins));
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.confidence_risk, 0.0);
        assert_eq!(effect.pirate_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effects_rich_ore_pirate_equals_point_two() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::RichOreVeins));
        assert_eq!(effect.pirate_risk, 0.2);
    }

    #[test]
    fn zone_modifier_effects_clear_signals_confidence_negative_only() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::ClearSignals));
        assert!(effect.confidence_risk < 0.0);
        assert_eq!(effect.fuel_risk, 0.0);
        assert_eq!(effect.pirate_risk, 0.0);
    }

    #[test]
    fn zone_modifier_effects_high_radiation_values_match() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::HighRadiation));
        assert_eq!(effect.fuel_risk, 0.2);
        assert_eq!(effect.confidence_risk, 0.05);
        assert_eq!(effect.pirate_risk, 0.1);
    }

    #[test]
    fn zone_modifier_effects_nebula_confidence_and_pirate_positive() {
        let effect = zone_modifier_effect(Some(super::ZoneModifier::NebulaInterference));
        assert!(effect.confidence_risk > 0.0);
        assert!(effect.pirate_risk > 0.0);
    }

    #[test]
    fn zone_id_equality() {
        let zone_a = super::ZoneId(1);
        let zone_b = super::ZoneId(1);
        let zone_c = super::ZoneId(2);

        assert_eq!(zone_a, zone_b);
        assert_ne!(zone_a, zone_c);
    }

    #[test]
    fn zone_id_can_be_cloned() {
        let zone = super::ZoneId(42);
        let cloned = zone;
        assert_eq!(zone, cloned);
    }

    #[test]
    fn jump_gate_stores_source_and_destination() {
        let gate = super::JumpGate {
            source_zone: 100,
            destination_zone: 200,
        };

        assert_eq!(gate.source_zone, 100);
        assert_eq!(gate.destination_zone, 200);
    }

    #[test]
    fn jump_gate_fuel_cost_is_positive() {
        assert!(super::JUMP_GATE_FUEL_COST > 0.0);
    }

    #[test]
    fn jump_transition_tracks_destination_and_time() {
        let transition = super::JumpTransition {
            destination_zone: 300,
            remaining_seconds: super::JUMP_TRANSITION_SECONDS,
        };

        assert_eq!(transition.destination_zone, 300);
        assert_eq!(transition.remaining_seconds, super::JUMP_TRANSITION_SECONDS);
    }

    #[test]
    fn jump_transition_duration_is_positive() {
        assert!(super::JUMP_TRANSITION_SECONDS > 0.0);
    }
}
