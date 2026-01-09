use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct PirateBase {
    pub launch_interval_ticks: u64,
    pub next_launch_tick: u64,
}

/// Pirate ship behavior state
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PirateShipBehavior {
    /// Normal roaming/harassing behavior
    #[default]
    Roaming,
    /// Docked at an Outpost for resupply (flavor only)
    DockedAtOutpost { ticks_remaining: u64 },
}

#[derive(Component, Debug)]
pub struct PirateShip {
    pub speed: f32,
    pub behavior: PirateShipBehavior,
}

pub fn schedule_next_launch(current_tick: u64, interval: u64) -> u64 {
    current_tick.saturating_add(interval)
}

#[cfg(test)]
mod tests {
    use super::{schedule_next_launch, PirateShipBehavior};

    #[test]
    fn schedule_next_launch_advances_by_interval() {
        let next = schedule_next_launch(10, 25);
        assert_eq!(next, 35);
    }

    #[test]
    fn pirate_behavior_default_is_roaming() {
        let behavior = PirateShipBehavior::default();
        assert_eq!(behavior, PirateShipBehavior::Roaming);
    }

    #[test]
    fn pirate_behavior_docked_tracks_ticks() {
        let behavior = PirateShipBehavior::DockedAtOutpost {
            ticks_remaining: 100,
        };
        if let PirateShipBehavior::DockedAtOutpost { ticks_remaining } = behavior {
            assert_eq!(ticks_remaining, 100);
        } else {
            panic!("Expected DockedAtOutpost");
        }
    }

    #[test]
    fn pirate_behavior_roaming_is_not_docked() {
        let behavior = PirateShipBehavior::Roaming;
        assert!(!matches!(
            behavior,
            PirateShipBehavior::DockedAtOutpost { .. }
        ));
    }
}
