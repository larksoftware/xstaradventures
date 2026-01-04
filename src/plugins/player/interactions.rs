//! Player interaction systems: mining, building, refueling, combat.

use bevy::prelude::*;

use crate::compat::SpatialBundle;
use crate::ore::{mine_amount, OreKind, OreNode};
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::{EventLog, InputBindings};
use crate::ships::{Cargo, Ship};
use crate::stations::{
    station_build_time_seconds, station_fuel_capacity, station_ore_capacity, Station, StationBuild,
    StationCrisisLog, StationKind, StationProduction, StationState,
};
use crate::world::{Identified, JumpGate, SystemNode, ZoneId};

use super::components::PlayerControl;

// =============================================================================
// Systems
// =============================================================================

pub fn player_mining(
    time: Res<Time<Fixed>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut Cargo, &mut Ship), With<PlayerControl>>,
    mut ore_nodes: Query<(Entity, &Transform, &mut OreNode)>,
) {
    if !input.pressed(bindings.interact) {
        return;
    }

    let (player_transform, mut cargo, mut ship) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let mut closest = None;
    let mut closest_dist = 0.0;
    let range = 24.0;

    for (entity, transform, ore) in ore_nodes.iter() {
        if ore.remaining <= 0.0 {
            continue;
        }
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let dist = pos.distance(player_pos);
        if dist <= range && (closest.is_none() || dist < closest_dist) {
            closest = Some((entity, dist));
            closest_dist = dist;
        }
    }

    let (target_entity, _) = match closest {
        Some(value) => value,
        None => {
            return;
        }
    };

    let delta_seconds = time.delta_secs();

    if let Ok((_entity, _transform, mut ore)) = ore_nodes.get_mut(target_entity) {
        let _mined = match ore.kind {
            OreKind::CommonOre => {
                let free_capacity = cargo.ore_free_space() as f32;
                let amount = mine_amount(
                    ore.remaining,
                    ore.rate_per_second,
                    delta_seconds,
                    free_capacity,
                );
                if amount > 0.0 {
                    ore.remaining -= amount;
                    cargo.add_ore(amount as u32);
                }
                amount
            }
            OreKind::FuelOre => {
                let free_capacity = (ship.fuel_capacity - ship.fuel).max(0.0);
                let amount = mine_amount(
                    ore.remaining,
                    ore.rate_per_second,
                    delta_seconds,
                    free_capacity,
                );
                if amount > 0.0 {
                    ore.remaining -= amount;
                    ship.fuel += amount;
                }
                amount
            }
        };

        if ore.remaining <= 0.0 {
            commands.entity(target_entity).despawn();
            let kind_str = match ore.kind {
                OreKind::CommonOre => "Ore",
                OreKind::FuelOre => "Fuel",
            };
            log.push(format!("{} node depleted", kind_str));
        }
    }
}

pub fn player_build_outpost(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut Cargo), With<PlayerControl>>,
    nodes: Query<&SystemNode>,
    stations: Query<&Transform, With<Station>>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_transform, mut cargo) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let cost = 18;
    if !can_build_outpost(cargo.ore, cost) {
        return;
    }

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    let mut target_node = None;
    let mut best_dist = 0.0;
    let build_range = 26.0;

    for node in nodes.iter() {
        let dist = node.position.distance(player_pos);
        if dist <= build_range && (target_node.is_none() || dist < best_dist) {
            target_node = Some(node);
            best_dist = dist;
        }
    }

    let node = match target_node {
        Some(node) => node,
        None => {
            return;
        }
    };

    for station_transform in stations.iter() {
        let station_pos = Vec2::new(
            station_transform.translation.x,
            station_transform.translation.y,
        );
        if station_pos.distance(node.position) <= 18.0 {
            return;
        }
    }

    let kind = StationKind::MiningOutpost;
    let capacity = station_fuel_capacity(kind);
    let build_time = station_build_time_seconds(kind);
    let ore_capacity = station_ore_capacity(kind);

    commands.spawn((
        Station {
            kind,
            state: StationState::Deploying,
            fuel: capacity * 0.5,
            fuel_capacity: capacity,
        },
        StationBuild {
            remaining_seconds: build_time,
        },
        StationProduction {
            ore: 0.0,
            ore_capacity,
        },
        StationCrisisLog::default(),
        ZoneId(node.id),
        Name::new(format!("Station-{}", node.id)),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + 12.0,
            node.position.y + 8.0,
            0.5,
        )),
    ));

    cargo.remove_ore(cost);
    log.push(format!("Outpost deployed at zone {}", node.id));
}

pub fn player_refuel_station(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut log: ResMut<EventLog>,
    mut player_query: Query<(&Transform, &mut Ship, &mut Cargo), With<PlayerControl>>,
    mut stations: Query<(&Transform, &mut Station, Option<&mut StationProduction>)>,
) {
    if !input.just_pressed(bindings.interact) {
        return;
    }

    let (player_transform, mut ship, mut cargo) = match player_query.single_mut() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let range = 22.0;
    let fuel_transfer = 10.0;
    let ore_transfer = 8.0;
    let mut refueled = false;
    let mut supplied_ore = false;

    for (_transform, mut station, production_opt) in stations.iter_mut() {
        let station_pos = Vec2::new(_transform.translation.x, _transform.translation.y);
        if station_pos.distance(player_pos) > range {
            continue;
        }

        if ship.fuel > 0.0 {
            let (new_ship_fuel, new_station_fuel, did_refuel) = transfer_fuel(
                ship.fuel,
                station.fuel,
                station.fuel_capacity,
                fuel_transfer,
            );
            ship.fuel = new_ship_fuel;
            station.fuel = new_station_fuel;
            refueled = did_refuel;
        }

        if cargo.ore > 0 {
            if let Some(mut production) = production_opt {
                let available = (cargo.ore as f32).min(ore_transfer);
                let free_capacity = (production.ore_capacity - production.ore).max(0.0);
                let transferred = available.min(free_capacity);

                if transferred > 0.0 {
                    cargo.remove_ore(transferred as u32);
                    production.ore += transferred;
                    supplied_ore = true;
                }
            }
        }

        break;
    }

    if refueled && supplied_ore {
        log.push("Transferred fuel and ore to station".to_string());
    } else if refueled {
        log.push("Transferred fuel to station".to_string());
    } else if supplied_ore {
        log.push("Transferred ore to station".to_string());
    }
}

pub fn player_fire(
    input: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut log: ResMut<EventLog>,
    player_query: Query<&Transform, With<PlayerControl>>,
    pirates: Query<(Entity, &Transform), With<PirateShip>>,
) {
    if !input.just_pressed(MouseButton::Left) {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let origin = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );
    let range = 24.0;
    let mut targets = Vec::new();
    let mut entities = Vec::new();

    for (entity, transform) in pirates.iter() {
        targets.push(Vec2::new(transform.translation.x, transform.translation.y));
        entities.push(entity);
    }

    if let Some(index) = closest_in_range(origin, &targets, range) {
        if let Some(target) = entities.get(index) {
            commands.entity(*target).despawn();
            log.push("Pirate ship destroyed".to_string());
        }
    }
}

/// Range at which player automatically identifies contacts
const PLAYER_IDENTIFY_RANGE: f32 = 150.0;

/// Automatically identifies entities when player gets close enough.
#[allow(clippy::type_complexity)]
pub fn player_identify_nearby(
    mut commands: Commands,
    player_query: Query<&Transform, With<PlayerControl>>,
    unidentified: Query<
        (Entity, &Transform),
        (
            Without<Identified>,
            Or<(
                With<OreNode>,
                With<Station>,
                With<PirateShip>,
                With<PirateBase>,
                With<JumpGate>,
            )>,
        ),
    >,
) {
    let player_transform = match player_query.single() {
        Ok(value) => value,
        Err(_) => return,
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    for (entity, transform) in unidentified.iter() {
        let entity_pos = Vec2::new(transform.translation.x, transform.translation.y);
        if entity_pos.distance(player_pos) <= PLAYER_IDENTIFY_RANGE {
            commands.entity(entity).insert(Identified);
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

pub fn can_build_outpost(ore: u32, cost: u32) -> bool {
    ore >= cost
}

pub fn transfer_fuel(
    ship_fuel: f32,
    station_fuel: f32,
    station_capacity: f32,
    amount: f32,
) -> (f32, f32, bool) {
    if ship_fuel <= 0.0 || station_capacity <= 0.0 || amount <= 0.0 {
        return (ship_fuel, station_fuel, false);
    }

    let available = ship_fuel.min(amount);
    let room = (station_capacity - station_fuel).max(0.0);
    let transfer = available.min(room);
    if transfer <= 0.0 {
        return (ship_fuel, station_fuel, false);
    }

    (ship_fuel - transfer, station_fuel + transfer, true)
}

pub fn closest_in_range(origin: Vec2, targets: &[Vec2], range: f32) -> Option<usize> {
    let mut closest = None;
    let mut best_dist = 0.0;
    for (index, pos) in targets.iter().enumerate() {
        let dist = pos.distance(origin);
        if dist <= range && (closest.is_none() || dist < best_dist) {
            closest = Some(index);
            best_dist = dist;
        }
    }
    closest
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_build_outpost_requires_enough_ore() {
        assert!(can_build_outpost(18, 18));
        assert!(!can_build_outpost(10, 18));
    }

    #[test]
    fn transfer_fuel_respects_capacity() {
        let (ship, station, did) = transfer_fuel(5.0, 8.0, 10.0, 5.0);
        assert!(did);
        assert_eq!(ship, 3.0);
        assert_eq!(station, 10.0);
    }

    #[test]
    fn closest_in_range_picks_nearest() {
        let origin = Vec2::new(0.0, 0.0);
        let targets = vec![Vec2::new(10.0, 0.0), Vec2::new(5.0, 0.0)];
        let index = closest_in_range(origin, &targets, 12.0);
        assert_eq!(index, Some(1));
    }
}
