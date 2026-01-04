use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ShipKind {
    PlayerShip,
    Scout,
    Miner,
    Security,
}

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum FleetRole {
    Scout,
    Mining,
    #[default]
    Security,
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
pub struct Cargo {
    pub ore: u32,
    pub ore_capacity: u32,
    #[allow(dead_code)]
    pub fuel: f32,
    #[allow(dead_code)]
    pub fuel_capacity: f32,
}

impl Default for Cargo {
    fn default() -> Self {
        Self {
            ore: 0,
            ore_capacity: 50,
            fuel: 0.0,
            fuel_capacity: 100.0,
        }
    }
}

impl Cargo {
    /// Add ore up to capacity. Returns the amount actually added.
    pub fn add_ore(&mut self, amount: u32) -> u32 {
        let free = self.ore_capacity.saturating_sub(self.ore);
        let added = amount.min(free);
        self.ore += added;
        added
    }

    /// Remove ore. Returns the amount actually removed.
    pub fn remove_ore(&mut self, amount: u32) -> u32 {
        let removed = amount.min(self.ore);
        self.ore -= removed;
        removed
    }

    /// Add fuel up to capacity. Returns the amount actually added.
    #[allow(dead_code)]
    pub fn add_fuel(&mut self, amount: f32) -> f32 {
        let free = (self.fuel_capacity - self.fuel).max(0.0);
        let added = amount.min(free);
        self.fuel += added;
        added
    }

    /// Remove fuel. Returns the amount actually removed.
    #[allow(dead_code)]
    pub fn remove_fuel(&mut self, amount: f32) -> f32 {
        let removed = amount.min(self.fuel);
        self.fuel -= removed;
        removed
    }

    /// Get available ore space.
    pub fn ore_free_space(&self) -> u32 {
        self.ore_capacity.saturating_sub(self.ore)
    }

    /// Get available fuel space.
    #[allow(dead_code)]
    pub fn fuel_free_space(&self) -> f32 {
        (self.fuel_capacity - self.fuel).max(0.0)
    }
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Velocity {
    #[allow(dead_code)]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[allow(dead_code)]
    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

pub fn cargo_capacity(kind: ShipKind) -> f32 {
    match kind {
        ShipKind::PlayerShip => 40.0,
        ShipKind::Scout => 10.0,
        ShipKind::Miner => 60.0,
        ShipKind::Security => 20.0,
    }
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
    use super::{cargo_capacity, ship_default_role, Cargo, FleetRole, ShipKind};

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

    #[test]
    fn cargo_capacity_player_exceeds_scout() {
        let player = cargo_capacity(ShipKind::PlayerShip);
        let scout = cargo_capacity(ShipKind::Scout);
        assert!(player > scout);
    }

    // =============================================================================
    // Cargo system tests (TDD - write failing tests first)
    // =============================================================================

    #[test]
    fn cargo_default_has_zero_ore() {
        let cargo = Cargo::default();
        assert_eq!(cargo.ore, 0);
    }

    #[test]
    fn cargo_default_has_zero_fuel() {
        let cargo = Cargo::default();
        assert!((cargo.fuel - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cargo_default_ore_capacity_is_50() {
        let cargo = Cargo::default();
        assert_eq!(cargo.ore_capacity, 50);
    }

    #[test]
    fn cargo_default_fuel_capacity_is_100() {
        let cargo = Cargo::default();
        assert!((cargo.fuel_capacity - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cargo_add_ore_respects_capacity() {
        let mut cargo = Cargo::default();
        let added = cargo.add_ore(30);
        assert_eq!(added, 30);
        assert_eq!(cargo.ore, 30);
    }

    #[test]
    fn cargo_add_ore_cannot_exceed_capacity() {
        let mut cargo = Cargo::default();
        cargo.add_ore(40);
        let added = cargo.add_ore(20);
        assert_eq!(added, 10); // Only 10 fits
        assert_eq!(cargo.ore, 50); // At capacity
    }

    #[test]
    fn cargo_add_fuel_respects_capacity() {
        let mut cargo = Cargo::default();
        let added = cargo.add_fuel(50.0);
        assert!((added - 50.0).abs() < f32::EPSILON);
        assert!((cargo.fuel - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cargo_add_fuel_cannot_exceed_capacity() {
        let mut cargo = Cargo::default();
        cargo.add_fuel(80.0);
        let added = cargo.add_fuel(30.0);
        assert!((added - 20.0).abs() < f32::EPSILON); // Only 20 fits
        assert!((cargo.fuel - 100.0).abs() < f32::EPSILON); // At capacity
    }

    #[test]
    fn cargo_remove_ore_returns_amount_removed() {
        let mut cargo = Cargo::default();
        cargo.add_ore(30);
        let removed = cargo.remove_ore(20);
        assert_eq!(removed, 20);
        assert_eq!(cargo.ore, 10);
    }

    #[test]
    fn cargo_remove_ore_cannot_go_negative() {
        let mut cargo = Cargo::default();
        cargo.add_ore(10);
        let removed = cargo.remove_ore(20);
        assert_eq!(removed, 10); // Only had 10
        assert_eq!(cargo.ore, 0);
    }

    #[test]
    fn cargo_remove_fuel_returns_amount_removed() {
        let mut cargo = Cargo::default();
        cargo.add_fuel(50.0);
        let removed = cargo.remove_fuel(30.0);
        assert!((removed - 30.0).abs() < f32::EPSILON);
        assert!((cargo.fuel - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cargo_remove_fuel_cannot_go_negative() {
        let mut cargo = Cargo::default();
        cargo.add_fuel(10.0);
        let removed = cargo.remove_fuel(20.0);
        assert!((removed - 10.0).abs() < f32::EPSILON); // Only had 10
        assert!(cargo.fuel.abs() < f32::EPSILON);
    }

    #[test]
    fn cargo_ore_free_space_calculated_correctly() {
        let mut cargo = Cargo::default();
        cargo.add_ore(30);
        assert_eq!(cargo.ore_free_space(), 20);
    }

    #[test]
    fn cargo_fuel_free_space_calculated_correctly() {
        let mut cargo = Cargo::default();
        cargo.add_fuel(60.0);
        assert!((cargo.fuel_free_space() - 40.0).abs() < f32::EPSILON);
    }
}
