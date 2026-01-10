use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub enum OreKind {
    #[default]
    CommonOre,
    FuelOre,
}

/// Mineable asteroid with ore resources
#[derive(Component, Debug, Clone, Copy)]
pub struct OreNode {
    pub kind: OreKind,
    pub remaining: f32,
    pub capacity: f32,
    pub rate_per_second: f32,
}

/// Non-mineable decorative asteroid (obstacle/scenery)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Asteroid {
    /// Visual size multiplier (0.5 to 1.5)
    pub size: f32,
}

impl OreNode {
    pub fn remaining_ratio(&self) -> f32 {
        if self.capacity > 0.0 {
            (self.remaining / self.capacity).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

pub fn mine_amount(
    available: f32,
    rate_per_second: f32,
    delta_seconds: f32,
    free_capacity: f32,
) -> f32 {
    if available <= 0.0 || rate_per_second <= 0.0 || delta_seconds <= 0.0 || free_capacity <= 0.0 {
        return 0.0;
    }

    let amount = rate_per_second * delta_seconds;
    let bounded = amount.min(available).min(free_capacity);
    if bounded > 0.0 {
        bounded
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{mine_amount, OreKind};

    #[test]
    fn mine_amount_clamps_to_available_and_capacity() {
        let mined = mine_amount(5.0, 10.0, 1.0, 3.0);
        assert_eq!(mined, 3.0);
    }

    #[test]
    fn mine_amount_zero_when_no_time() {
        let mined = mine_amount(5.0, 10.0, 0.0, 10.0);
        assert_eq!(mined, 0.0);
    }

    #[test]
    fn mine_amount_zero_when_no_capacity() {
        let mined = mine_amount(5.0, 10.0, 1.0, 0.0);
        assert_eq!(mined, 0.0);
    }

    #[test]
    fn ore_kind_default_is_common_ore() {
        let kind = OreKind::default();
        assert_eq!(kind, OreKind::CommonOre);
    }

    #[test]
    fn ore_kind_variants_not_equal() {
        assert_ne!(OreKind::CommonOre, OreKind::FuelOre);
    }
}
