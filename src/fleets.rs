use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RiskTolerance {
    Cautious,
    Balanced,
    Bold,
}

impl Default for RiskTolerance {
    fn default() -> Self {
        RiskTolerance::Balanced
    }
}

#[derive(Component, Debug)]
pub struct ScoutBehavior {
    pub risk: RiskTolerance,
    pub current_node: u32,
    pub target_node: Option<u32>,
    pub next_decision_tick: u64,
}

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

pub fn scout_confidence(risk: RiskTolerance, route_risk: f32) -> f32 {
    let base = match risk {
        RiskTolerance::Cautious => 0.75,
        RiskTolerance::Balanced => 0.65,
        RiskTolerance::Bold => 0.55,
    };
    (base - (route_risk * 0.4)).clamp(0.2, 0.9)
}

#[cfg(test)]
mod tests {
    use super::{next_risk, risk_threshold, scout_confidence, RiskTolerance};

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
}
