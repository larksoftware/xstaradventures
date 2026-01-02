use bevy::prelude::*;

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
}
