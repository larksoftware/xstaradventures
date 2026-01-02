use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShipKind {
    PlayerShip,
    Scout,
    Miner,
    Security,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FleetRole {
    Scout,
    Mining,
    Security,
}

impl Default for FleetRole {
    fn default() -> Self {
        FleetRole::Security
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShipState {
    Idle,
    InTransit,
    Executing,
    Returning,
    Refueling,
    Damaged,
    Disabled,
}

#[derive(Component, Debug)]
pub struct Ship {
    pub kind: ShipKind,
    pub state: ShipState,
    pub fuel: f32,
    pub fuel_capacity: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Fleet {
    pub role: FleetRole,
}

#[derive(Component, Debug, Default)]
pub struct ShipFuelAlert {
    pub low: bool,
    pub critical: bool,
}

pub fn ship_default_role(kind: ShipKind) -> FleetRole {
    match kind {
        ShipKind::PlayerShip => FleetRole::Security,
        ShipKind::Scout => FleetRole::Scout,
        ShipKind::Miner => FleetRole::Mining,
        ShipKind::Security => FleetRole::Security,
    }
}

pub fn ship_fuel_capacity(kind: ShipKind) -> f32 {
    match kind {
        ShipKind::PlayerShip => 60.0,
        ShipKind::Scout => 30.0,
        ShipKind::Miner => 45.0,
        ShipKind::Security => 45.0,
    }
}

pub fn ship_fuel_burn_per_minute(kind: ShipKind) -> f32 {
    match kind {
        ShipKind::PlayerShip => 2.0,
        ShipKind::Scout => 1.0,
        ShipKind::Miner => 1.5,
        ShipKind::Security => 1.5,
    }
}

#[cfg(test)]
mod tests {
    use super::{ship_default_role, FleetRole, ShipKind};

    #[test]
    fn ship_default_role_player_is_security() {
        let role = ship_default_role(ShipKind::PlayerShip);
        assert_eq!(role, FleetRole::Security);
    }

    #[test]
    fn ship_default_role_scout_is_scout() {
        let role = ship_default_role(ShipKind::Scout);
        assert_eq!(role, FleetRole::Scout);
    }

    #[test]
    fn ship_fuel_capacity_values() {
        assert_eq!(super::ship_fuel_capacity(ShipKind::PlayerShip), 60.0);
        assert_eq!(super::ship_fuel_capacity(ShipKind::Scout), 30.0);
        assert_eq!(super::ship_fuel_capacity(ShipKind::Miner), 45.0);
        assert_eq!(super::ship_fuel_capacity(ShipKind::Security), 45.0);
    }

    #[test]
    fn ship_fuel_capacity_scout_plus_security_greater_than_player() {
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        let security = super::ship_fuel_capacity(ShipKind::Security);
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);

        assert!(scout + security > player);
    }

    #[test]
    fn ship_fuel_burn_values() {
        assert_eq!(super::ship_fuel_burn_per_minute(ShipKind::PlayerShip), 2.0);
        assert_eq!(super::ship_fuel_burn_per_minute(ShipKind::Scout), 1.0);
        assert_eq!(super::ship_fuel_burn_per_minute(ShipKind::Miner), 1.5);
        assert_eq!(super::ship_fuel_burn_per_minute(ShipKind::Security), 1.5);
    }

    #[test]
    fn ship_default_role_miner_is_mining() {
        let role = ship_default_role(ShipKind::Miner);
        assert_eq!(role, FleetRole::Mining);
    }

    #[test]
    fn ship_default_role_security_is_security() {
        let role = ship_default_role(ShipKind::Security);
        assert_eq!(role, FleetRole::Security);
    }

    #[test]
    fn ship_fuel_capacity_positive_for_all_kinds() {
        let kinds = [
            ShipKind::PlayerShip,
            ShipKind::Scout,
            ShipKind::Miner,
            ShipKind::Security,
        ];

        for kind in kinds {
            let capacity = super::ship_fuel_capacity(kind);
            assert!(capacity > 0.0);
        }
    }

    #[test]
    fn ship_fuel_burn_positive_for_all_kinds() {
        let kinds = [
            ShipKind::PlayerShip,
            ShipKind::Scout,
            ShipKind::Miner,
            ShipKind::Security,
        ];

        for kind in kinds {
            let burn = super::ship_fuel_burn_per_minute(kind);
            assert!(burn > 0.0);
        }
    }

    #[test]
    fn ship_fuel_capacity_ordering_player_is_max() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        let security = super::ship_fuel_capacity(ShipKind::Security);

        assert!(player >= scout);
        assert!(player >= miner);
        assert!(player >= security);
    }

    #[test]
    fn ship_fuel_burn_ordering_player_is_max() {
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        let security = super::ship_fuel_burn_per_minute(ShipKind::Security);

        assert!(player >= scout);
        assert!(player >= miner);
        assert!(player >= security);
    }

    #[test]
    fn ship_fuel_capacity_scout_is_min() {
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        let security = super::ship_fuel_capacity(ShipKind::Security);

        assert!(scout <= player);
        assert!(scout <= miner);
        assert!(scout <= security);
    }

    #[test]
    fn ship_fuel_capacity_scout_less_than_miner() {
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        assert!(scout < miner);
    }

    #[test]
    fn ship_fuel_capacity_player_minus_security() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let security = super::ship_fuel_capacity(ShipKind::Security);
        assert_eq!(player - security, 15.0);
    }

    #[test]
    fn ship_fuel_burn_miner_vs_scout_delta() {
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        assert_eq!(miner - scout, 0.5);
    }

    #[test]
    fn ship_fuel_burn_scout_is_min() {
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        let security = super::ship_fuel_burn_per_minute(ShipKind::Security);

        assert!(scout <= player);
        assert!(scout <= miner);
        assert!(scout <= security);
    }

    #[test]
    fn ship_fuel_capacity_miner_equals_security() {
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        let security = super::ship_fuel_capacity(ShipKind::Security);
        assert_eq!(miner, security);
    }

    #[test]
    fn ship_fuel_burn_miner_equals_security() {
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        let security = super::ship_fuel_burn_per_minute(ShipKind::Security);
        assert_eq!(miner, security);
    }

    #[test]
    fn ship_fuel_capacity_player_exceeds_scout() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        assert!(player > scout);
    }

    #[test]
    fn ship_fuel_burn_player_exceeds_scout() {
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        assert!(player > scout);
    }

    #[test]
    fn ship_default_role_not_equal_across_kinds() {
        let scout = ship_default_role(ShipKind::Scout);
        let miner = ship_default_role(ShipKind::Miner);
        assert_ne!(scout, miner);
    }

    #[test]
    fn ship_fuel_capacity_player_vs_miner_delta() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        assert_eq!(player - miner, 15.0);
    }

    #[test]
    fn ship_fuel_capacity_player_exceeds_security_by_constant() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let security = super::ship_fuel_capacity(ShipKind::Security);
        assert_eq!(player - security, 15.0);
    }

    #[test]
    fn ship_default_role_player_is_security_reaffirm() {
        let role = ship_default_role(ShipKind::PlayerShip);
        assert_eq!(role, FleetRole::Security);
    }

    #[test]
    fn ship_fuel_capacity_security_equals_miner() {
        let security = super::ship_fuel_capacity(ShipKind::Security);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        assert_eq!(security, miner);
    }

    #[test]
    fn ship_fuel_capacity_security_equals_miner_again() {
        let security = super::ship_fuel_capacity(ShipKind::Security);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        assert_eq!(security, miner);
    }

    #[test]
    fn ship_fuel_burn_security_equals_miner() {
        let security = super::ship_fuel_burn_per_minute(ShipKind::Security);
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        assert_eq!(security, miner);
    }

    #[test]
    fn ship_fuel_burn_security_equals_miner_again() {
        let security = super::ship_fuel_burn_per_minute(ShipKind::Security);
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        assert_eq!(security, miner);
    }

    #[test]
    fn ship_fuel_burn_security_equals_miner_reaffirm() {
        let security = super::ship_fuel_burn_per_minute(ShipKind::Security);
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        assert_eq!(security, miner);
    }

    #[test]
    fn ship_fuel_capacity_player_vs_scout_delta() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        assert_eq!(player - scout, 30.0);
    }

    #[test]
    fn ship_fuel_capacity_miner_plus_scout_exceeds_player() {
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        let scout = super::ship_fuel_capacity(ShipKind::Scout);
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);

        assert!(miner + scout > player);
    }

    #[test]
    fn ship_fuel_capacity_player_equals_scout_plus_miner_minus_fifteen() {
        let player = super::ship_fuel_capacity(ShipKind::PlayerShip);
        let miner = super::ship_fuel_capacity(ShipKind::Miner);
        let scout = super::ship_fuel_capacity(ShipKind::Scout);

        assert_eq!(player, scout + miner - 15.0);
    }

    #[test]
    fn ship_fuel_burn_player_vs_scout_delta() {
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        assert_eq!(player - scout, 1.0);
    }

    #[test]
    fn ship_fuel_burn_player_equals_scout_plus_one() {
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        assert_eq!(player, scout + 1.0);
    }

    #[test]
    fn ship_fuel_burn_player_exceeds_miner_by_constant() {
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        assert_eq!(player - miner, 0.5);
    }

    #[test]
    fn ship_fuel_burn_miner_plus_scout_exceeds_player() {
        let miner = super::ship_fuel_burn_per_minute(ShipKind::Miner);
        let scout = super::ship_fuel_burn_per_minute(ShipKind::Scout);
        let player = super::ship_fuel_burn_per_minute(ShipKind::PlayerShip);

        assert!(miner + scout > player);
    }

    #[test]
    fn ship_default_role_scout_not_security() {
        let scout = ship_default_role(ShipKind::Scout);
        assert_ne!(scout, FleetRole::Security);
    }

    #[test]
    fn ship_default_role_miner_not_scout() {
        let miner = ship_default_role(ShipKind::Miner);
        assert_ne!(miner, FleetRole::Scout);
    }
}
