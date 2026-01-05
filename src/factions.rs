//! Faction system for entity ownership and relationships.

use bevy::prelude::*;

/// Faction ownership for ships and stations.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
#[allow(dead_code)]
pub enum Faction {
    /// Player-owned entities (player ship, scouts, player-built stations)
    #[default]
    Player,
    /// Pirate entities (pirate ships, pirate bases)
    Pirate,
    /// Neutral NPC entities (NPC stations like Outposts)
    Independent,
}

/// Relationship between two factions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum Relationship {
    /// Will attack on sight
    Hostile,
    /// Will not attack, but not allied
    Neutral,
}

#[allow(dead_code)]
impl Faction {
    /// Get the relationship from this faction toward another.
    pub fn relationship_to(&self, other: Faction) -> Relationship {
        match (self, other) {
            // Same faction is neutral (no friendly fire)
            (Faction::Player, Faction::Player) => Relationship::Neutral,
            (Faction::Pirate, Faction::Pirate) => Relationship::Neutral,
            (Faction::Independent, Faction::Independent) => Relationship::Neutral,

            // Pirates are hostile to everyone
            (Faction::Pirate, _) => Relationship::Hostile,
            (_, Faction::Pirate) => Relationship::Hostile,

            // Player and Independent are neutral
            (Faction::Player, Faction::Independent) => Relationship::Neutral,
            (Faction::Independent, Faction::Player) => Relationship::Neutral,
        }
    }

    /// Check if this faction is hostile toward another.
    pub fn is_hostile_to(&self, other: Faction) -> bool {
        self.relationship_to(other) == Relationship::Hostile
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_faction_is_default() {
        let faction = Faction::default();
        assert_eq!(faction, Faction::Player);
    }

    #[test]
    fn pirates_are_hostile_to_player() {
        assert!(Faction::Pirate.is_hostile_to(Faction::Player));
        assert!(Faction::Player.is_hostile_to(Faction::Pirate));
    }

    #[test]
    fn pirates_are_hostile_to_independent() {
        assert!(Faction::Pirate.is_hostile_to(Faction::Independent));
        assert!(Faction::Independent.is_hostile_to(Faction::Pirate));
    }

    #[test]
    fn independent_is_neutral_to_player() {
        assert!(!Faction::Independent.is_hostile_to(Faction::Player));
        assert!(!Faction::Player.is_hostile_to(Faction::Independent));
        assert_eq!(
            Faction::Independent.relationship_to(Faction::Player),
            Relationship::Neutral
        );
    }

    #[test]
    fn same_faction_is_neutral() {
        assert!(!Faction::Player.is_hostile_to(Faction::Player));
        assert!(!Faction::Pirate.is_hostile_to(Faction::Pirate));
        assert!(!Faction::Independent.is_hostile_to(Faction::Independent));
    }

    #[test]
    fn pirates_not_hostile_to_self() {
        assert_eq!(
            Faction::Pirate.relationship_to(Faction::Pirate),
            Relationship::Neutral
        );
    }
}
