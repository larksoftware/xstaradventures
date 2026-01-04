//! World view entity rendering (stations, ore, pirates, ships).

use bevy::asset::LoadState;
use bevy::prelude::*;
use std::path::Path;

use crate::compat::{SpriteBundle, Text2dBundle, TextStyle};
use crate::ore::{OreKind, OreNode};
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::player::PlayerControl;
use crate::ships::{Ship, ShipKind};
use crate::stations::Station;
use crate::world::ZoneId;

use super::components::{
    is_visible_in_zone, ship_kind_short, ship_state_short, station_kind_short, OreSpawnFilter,
    OreVisual, OreVisualMarker, PirateBaseSpawnFilter, PirateBaseVisual, PirateBaseVisualMarker,
    PirateShipSpawnFilter, PirateShipVisual, PirateShipVisualMarker, ShipLabel, ShipSpawnFilter,
    ShipVisual, ShipVisualMarker, StationLabel, StationSpawnFilter, StationVisual,
    StationVisualMarker,
};

// =============================================================================
// Constants
// =============================================================================

const PLAYER_SHIP_ASPECT: f32 = 860.0 / 1065.0;
const PLAYER_SHIP_WIDTH: f32 = 60.0;
const PLAYER_SHIP_SIZE: Vec2 = Vec2::new(PLAYER_SHIP_WIDTH, PLAYER_SHIP_WIDTH * PLAYER_SHIP_ASPECT);

// =============================================================================
// Resources
// =============================================================================

#[derive(Resource)]
pub struct PlayerShipTexture(pub Handle<Image>);

// =============================================================================
// Systems
// =============================================================================

pub fn load_player_ship_texture(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("sprites/player_ship.png");
    commands.insert_resource(PlayerShipTexture(texture));
}

pub fn spawn_station_visuals(
    mut commands: Commands,
    player_query: Query<&ZoneId, With<PlayerControl>>,
    stations: Query<(Entity, &Transform, Option<&ZoneId>), StationSpawnFilter>,
) {
    let player_zone = player_query.single().map(|z| z.0).ok();

    for (entity, transform, zone) in stations.iter() {
        // Mark the station as having a visual (to prevent duplicate spawning)
        commands.entity(entity).insert(StationVisualMarker);

        // Determine initial visibility based on zone
        let visible = match (player_zone, zone) {
            (Some(pz), Some(sz)) => sz.0 == pz,
            (Some(_), None) => true,
            (None, _) => true,
        };

        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.85, 0.8, 0.35),
                custom_size: Some(Vec2::new(10.0, 10.0)),
                ..default()
            },
            transform: *transform,
            visibility: if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            ..default()
        };

        commands.spawn((StationVisual { target: entity }, sprite));
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_station_visuals(
    mut commands: Commands,
    mut params: ParamSet<(
        Query<(Entity, &StationVisual, &mut Transform)>,
        Query<(Entity, &Transform), With<Station>>,
    )>,
) {
    let station_transforms = {
        let stations = params.p1();
        let mut map = std::collections::HashMap::new();

        for (entity, transform) in stations.iter() {
            map.insert(entity, *transform);
        }

        map
    };

    let mut visuals = params.p0();

    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        if let Some(station_transform) = station_transforms.get(&visual.target) {
            *transform = *station_transform;
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}

pub fn update_station_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<&ZoneId, With<PlayerControl>>,
    stations: Query<(
        &Station,
        Option<&crate::stations::StationCrisis>,
        &Transform,
        Option<&ZoneId>,
    )>,
    labels: Query<Entity, With<StationLabel>>,
) {
    for entity in labels.iter() {
        commands.entity(entity).despawn();
    }

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    // Get player zone for filtering
    let player_zone = player_query.single().map(|z| z.0).ok();

    for (station, crisis, transform, zone) in stations.iter() {
        // Skip stations not in player's zone
        if let Some(pz) = player_zone {
            let entity_zone = zone.map(|z| z.0);
            if !is_visible_in_zone(entity_zone, pz) {
                continue;
            }
        }

        let crisis_icon = if crisis.is_some() { "!" } else { "" };
        let label = format!("{}{}", station_kind_short(station.kind), crisis_icon);
        let pos = Vec2::new(transform.translation.x, transform.translation.y + 10.0);

        let mut bundle = Text2dBundle::from_section(
            label,
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.85, 0.8, 0.35),
            },
        );
        bundle.transform = Transform::from_xyz(pos.x, pos.y, 1.0);
        commands.spawn((StationLabel, bundle));
    }
}

pub fn spawn_ore_visuals(
    mut commands: Commands,
    player_query: Query<&ZoneId, With<PlayerControl>>,
    ores: Query<(Entity, &Transform, Option<&ZoneId>), OreSpawnFilter>,
) {
    let player_zone = player_query.single().map(|z| z.0).ok();

    for (entity, transform, zone) in ores.iter() {
        // Mark the ore node as having a visual (to prevent duplicate spawning)
        commands.entity(entity).insert(OreVisualMarker);

        // Determine initial visibility based on zone
        let visible = match (player_zone, zone) {
            (Some(pz), Some(oz)) => oz.0 == pz,
            (Some(_), None) => true,
            (None, _) => true,
        };

        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.75, 0.6, 0.35),
                custom_size: Some(Vec2::new(6.0, 6.0)),
                ..default()
            },
            transform: *transform,
            visibility: if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            ..default()
        };

        commands.spawn((OreVisual { target: entity }, sprite));
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_ore_visuals(
    mut commands: Commands,
    mut params: ParamSet<(
        Query<(Entity, &OreVisual, &mut Transform)>,
        Query<(Entity, &Transform), With<OreNode>>,
    )>,
) {
    let ore_transforms = {
        let ores = params.p1();
        let mut map = std::collections::HashMap::new();
        for (entity, transform) in ores.iter() {
            map.insert(entity, *transform);
        }
        map
    };

    let mut visuals = params.p0();
    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        if let Some(ore_transform) = ore_transforms.get(&visual.target) {
            *transform = *ore_transform;
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}

pub fn update_ore_visuals(
    ores: Query<(Entity, &OreNode)>,
    mut visuals: Query<(&OreVisual, &mut Sprite)>,
) {
    for (visual, mut sprite) in visuals.iter_mut() {
        if let Ok((_entity, ore)) = ores.get(visual.target) {
            let ratio = ore.remaining_ratio();
            sprite.color = match ore.kind {
                OreKind::CommonOre => {
                    if ratio <= 0.2 {
                        Color::srgb(0.5, 0.4, 0.25)
                    } else if ratio <= 0.6 {
                        Color::srgb(0.7, 0.55, 0.3)
                    } else {
                        Color::srgb(0.75, 0.6, 0.35)
                    }
                }
                OreKind::FuelOre => {
                    if ratio <= 0.2 {
                        Color::srgb(0.25, 0.5, 0.6)
                    } else if ratio <= 0.6 {
                        Color::srgb(0.35, 0.65, 0.75)
                    } else {
                        Color::srgb(0.4, 0.7, 0.85)
                    }
                }
            };
        }
    }
}

pub fn spawn_pirate_base_visuals(
    mut commands: Commands,
    player_query: Query<&ZoneId, With<PlayerControl>>,
    bases: Query<(Entity, &Transform, Option<&ZoneId>), PirateBaseSpawnFilter>,
) {
    let player_zone = player_query.single().map(|z| z.0).ok();

    for (entity, transform, zone) in bases.iter() {
        commands.entity(entity).insert(PirateBaseVisualMarker);

        let visible = match (player_zone, zone) {
            (Some(pz), Some(bz)) => bz.0 == pz,
            (Some(_), None) => true,
            (None, _) => true,
        };

        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.85, 0.25, 0.2),
                custom_size: Some(Vec2::new(12.0, 12.0)),
                ..default()
            },
            transform: *transform,
            visibility: if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            ..default()
        };

        commands.spawn((PirateBaseVisual { target: entity }, sprite));
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_pirate_base_visuals(
    mut commands: Commands,
    mut params: ParamSet<(
        Query<(Entity, &PirateBaseVisual, &mut Transform)>,
        Query<(Entity, &Transform), With<PirateBase>>,
    )>,
) {
    let base_transforms = {
        let bases = params.p1();
        let mut map = std::collections::HashMap::new();
        for (entity, transform) in bases.iter() {
            map.insert(entity, *transform);
        }
        map
    };

    let mut visuals = params.p0();
    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        if let Some(base_transform) = base_transforms.get(&visual.target) {
            *transform = *base_transform;
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}

pub fn spawn_pirate_ship_visuals(
    mut commands: Commands,
    player_query: Query<&ZoneId, With<PlayerControl>>,
    ships: Query<(Entity, &Transform, Option<&ZoneId>), PirateShipSpawnFilter>,
) {
    let player_zone = player_query.single().map(|z| z.0).ok();

    for (entity, transform, zone) in ships.iter() {
        commands.entity(entity).insert(PirateShipVisualMarker);

        let visible = match (player_zone, zone) {
            (Some(pz), Some(sz)) => sz.0 == pz,
            (Some(_), None) => true,
            (None, _) => true,
        };

        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.9, 0.35, 0.3),
                custom_size: Some(Vec2::new(8.0, 8.0)),
                ..default()
            },
            transform: *transform,
            visibility: if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            ..default()
        };

        commands.spawn((PirateShipVisual { target: entity }, sprite));
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_pirate_ship_visuals(
    mut commands: Commands,
    mut params: ParamSet<(
        Query<(Entity, &PirateShipVisual, &mut Transform)>,
        Query<(Entity, &Transform), With<PirateShip>>,
    )>,
) {
    let ship_transforms = {
        let ships = params.p1();
        let mut map = std::collections::HashMap::new();
        for (entity, transform) in ships.iter() {
            map.insert(entity, *transform);
        }
        map
    };

    let mut visuals = params.p0();
    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        if let Some(ship_transform) = ship_transforms.get(&visual.target) {
            *transform = *ship_transform;
        } else {
            commands.entity(visual_entity).despawn();
        }
    }
}

pub fn spawn_ship_visuals(
    mut commands: Commands,
    ships: Query<(Entity, &Transform, &Ship), ShipSpawnFilter>,
    player_texture: Res<PlayerShipTexture>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform, ship) in ships.iter() {
        let mut sprite_transform = *transform;
        sprite_transform.translation.z = 1.0;

        let is_player = matches!(ship.kind, ShipKind::PlayerShip);
        let size = if is_player {
            PLAYER_SHIP_SIZE
        } else {
            Vec2::new(10.0, 10.0)
        };
        let color = if is_player {
            Color::WHITE
        } else {
            Color::srgb(0.35, 0.85, 0.55)
        };
        let texture_ready = matches!(
            asset_server.get_load_state(&player_texture.0),
            Some(LoadState::Loaded)
        );
        if is_player && !texture_ready {
            continue;
        }

        let sprite = if is_player {
            SpriteBundle {
                sprite: Sprite {
                    image: player_texture.0.clone(),
                    color,
                    custom_size: Some(size),
                    ..default()
                },
                transform: sprite_transform,
                ..default()
            }
        } else {
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(size),
                    ..default()
                },
                transform: sprite_transform,
                ..default()
            }
        };

        commands.spawn((ShipVisual { target: entity }, sprite));
        commands.entity(entity).insert(ShipVisualMarker);

        info!(
            "Spawned ship visual: kind={:?}, pos=({:.1}, {:.1}, {:.1}), color={:?}, size={:?}",
            ship.kind,
            sprite_transform.translation.x,
            sprite_transform.translation.y,
            sprite_transform.translation.z,
            color,
            size
        );
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_ship_visuals(
    mut commands: Commands,
    mut params: ParamSet<(
        Query<(Entity, &ShipVisual, &mut Transform)>,
        Query<(Entity, &Transform), With<Ship>>,
    )>,
) {
    let ship_transforms = {
        let ships = params.p1();
        let mut map = std::collections::HashMap::new();
        for (entity, transform) in ships.iter() {
            map.insert(entity, *transform);
        }
        map
    };

    let mut visuals = params.p0();
    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        if let Some(ship_transform) = ship_transforms.get(&visual.target) {
            let z = transform.translation.z;
            *transform = *ship_transform;
            transform.translation.z = z;
        } else {
            commands.entity(visual.target).remove::<ShipVisualMarker>();
            commands.entity(visual_entity).despawn();
        }
    }
}

pub fn update_ship_visuals(
    ships: Query<(Entity, &Ship)>,
    mut visuals: Query<(&ShipVisual, &mut Sprite)>,
) {
    for (visual, mut sprite) in visuals.iter_mut() {
        if let Ok((_entity, ship)) = ships.get(visual.target) {
            let ratio = if ship.fuel_capacity > 0.0 {
                ship.fuel / ship.fuel_capacity
            } else {
                0.0
            };

            if matches!(ship.kind, ShipKind::PlayerShip) {
                sprite.color = Color::WHITE;
                continue;
            }

            sprite.color = if ratio <= 0.10 {
                Color::srgb(0.9, 0.25, 0.2)
            } else if ratio <= 0.25 {
                Color::srgb(0.95, 0.7, 0.2)
            } else {
                Color::srgb(0.35, 0.85, 0.55)
            };
        }
    }
}

pub fn update_ship_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<&ZoneId, With<PlayerControl>>,
    ships: Query<(&Ship, &Transform, Option<&ZoneId>)>,
    labels: Query<Entity, With<ShipLabel>>,
) {
    for entity in labels.iter() {
        commands.entity(entity).despawn();
    }

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    // Get player zone for filtering
    let player_zone = player_query.single().map(|z| z.0).ok();

    for (ship, transform, zone) in ships.iter() {
        // Skip ships not in player's zone
        if let Some(pz) = player_zone {
            let entity_zone = zone.map(|z| z.0);
            if !is_visible_in_zone(entity_zone, pz) {
                continue;
            }
        }

        let fuel_pct = if ship.fuel_capacity > 0.0 {
            (ship.fuel / ship.fuel_capacity) * 100.0
        } else {
            0.0
        };
        let label = format!(
            "{} {} {:.0}%",
            ship_kind_short(ship.kind),
            ship_state_short(ship.state),
            fuel_pct
        );
        let pos = Vec2::new(transform.translation.x, transform.translation.y + 10.0);

        let mut bundle = Text2dBundle::from_section(
            label,
            TextStyle {
                font: font.clone(),
                font_size: 11.0,
                color: Color::srgb(0.35, 0.85, 0.55),
            },
        );
        bundle.transform = Transform::from_xyz(pos.x, pos.y, 1.0);
        commands.spawn((ShipLabel, bundle));
    }
}

/// Synchronizes entity visibility based on zone matching with the player.
/// Entities in different zones than the player are hidden.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn sync_zone_visibility(
    player_query: Query<&ZoneId, With<PlayerControl>>,
    // Query zones for simulation entities
    stations: Query<(Entity, Option<&ZoneId>), With<Station>>,
    ores: Query<(Entity, Option<&ZoneId>), With<OreNode>>,
    pirate_bases: Query<(Entity, Option<&ZoneId>), With<PirateBase>>,
    pirate_ships: Query<(Entity, Option<&ZoneId>), With<PirateShip>>,
    ships: Query<(Entity, Option<&ZoneId>), With<Ship>>,
    // Query visuals to update visibility
    mut station_visuals: Query<(&StationVisual, &mut Visibility)>,
    mut ore_visuals: Query<(&OreVisual, &mut Visibility), Without<StationVisual>>,
    mut pirate_base_visuals: Query<
        (&PirateBaseVisual, &mut Visibility),
        (Without<StationVisual>, Without<OreVisual>),
    >,
    mut pirate_ship_visuals: Query<
        (&PirateShipVisual, &mut Visibility),
        (
            Without<StationVisual>,
            Without<OreVisual>,
            Without<PirateBaseVisual>,
        ),
    >,
    mut ship_visuals: Query<
        (&ShipVisual, &mut Visibility),
        (
            Without<StationVisual>,
            Without<OreVisual>,
            Without<PirateBaseVisual>,
            Without<PirateShipVisual>,
        ),
    >,
) {
    // Get player's current zone
    let player_zone = match player_query.single() {
        Ok(zone) => zone.0,
        Err(_) => return, // No player, skip
    };

    // Build lookup maps for entity zones
    let station_zones: std::collections::HashMap<Entity, Option<u32>> =
        stations.iter().map(|(e, z)| (e, z.map(|z| z.0))).collect();

    let ore_zones: std::collections::HashMap<Entity, Option<u32>> =
        ores.iter().map(|(e, z)| (e, z.map(|z| z.0))).collect();

    let pirate_base_zones: std::collections::HashMap<Entity, Option<u32>> = pirate_bases
        .iter()
        .map(|(e, z)| (e, z.map(|z| z.0)))
        .collect();

    let pirate_ship_zones: std::collections::HashMap<Entity, Option<u32>> = pirate_ships
        .iter()
        .map(|(e, z)| (e, z.map(|z| z.0)))
        .collect();

    let ship_zones: std::collections::HashMap<Entity, Option<u32>> =
        ships.iter().map(|(e, z)| (e, z.map(|z| z.0))).collect();

    // Update station visuals
    for (visual, mut visibility) in station_visuals.iter_mut() {
        let visible = match station_zones.get(&visual.target) {
            Some(&Some(zone)) => zone == player_zone,
            Some(&None) => true, // Entity exists but has no zone - show it
            None => false,       // Entity not found - hide visual
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Update ore visuals
    for (visual, mut visibility) in ore_visuals.iter_mut() {
        let visible = match ore_zones.get(&visual.target) {
            Some(&Some(zone)) => zone == player_zone,
            Some(&None) => true,
            None => false,
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Update pirate base visuals
    for (visual, mut visibility) in pirate_base_visuals.iter_mut() {
        let visible = match pirate_base_zones.get(&visual.target) {
            Some(&Some(zone)) => zone == player_zone,
            Some(&None) => true,
            None => false,
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Update pirate ship visuals
    for (visual, mut visibility) in pirate_ship_visuals.iter_mut() {
        let visible = match pirate_ship_zones.get(&visual.target) {
            Some(&Some(zone)) => zone == player_zone,
            Some(&None) => true,
            None => false,
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Update ship visuals (non-player ships)
    for (visual, mut visibility) in ship_visuals.iter_mut() {
        let visible = match ship_zones.get(&visual.target) {
            Some(&Some(zone)) => zone == player_zone,
            Some(&None) => true,
            None => false,
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
