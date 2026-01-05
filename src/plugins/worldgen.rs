use bevy::prelude::*;

use crate::compat::SpatialBundle;
use crate::factions::Faction;
use crate::fleets::{RiskTolerance, ScoutBehavior};
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::{DebugWindow, EventLog, GameState, InputBindings};
use crate::plugins::player::PlayerControl;
use crate::plugins::sim::{BoundaryWarningState, SimTickCount};
use crate::ships::{
    cargo_capacity, ship_default_role, ship_fuel_capacity, Cargo, Credits, Fleet, Ship,
    ShipFuelAlert, ShipKind, ShipState, Velocity,
};
use crate::stations::{
    station_build_time_seconds, station_fuel_capacity, Station, StationBuild, StationCrisisLog,
    StationKind, StationState,
};
use crate::world::{
    JumpGate, KnowledgeLayer, RouteEdge, Sector, SystemIntel, SystemNode, ZoneId, ZoneModifier,
};

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSeed>()
            .init_resource::<Sector>()
            .add_systems(OnEnter(GameState::Boot), seed_world)
            .add_systems(
                Update,
                (
                    handle_seed_input,
                    handle_modifier_randomize,
                    handle_reveal_adjacent,
                    handle_reveal_all,
                    handle_clear_reveal,
                    handle_debug_spawns,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(debug_window_open),
            );
    }
}

fn debug_window_open(debug_window: Res<DebugWindow>) -> bool {
    debug_window.open
}

/// Check if either Shift key is pressed (for debug key modifiers)
fn shift_pressed(input: &ButtonInput<KeyCode>) -> bool {
    input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
}

#[derive(Resource)]
pub struct WorldSeed {
    pub value: u64,
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self { value: 12345 }
    }
}

fn seed_world(mut commands: Commands, seed: Res<WorldSeed>, mut sector: ResMut<Sector>) {
    apply_seed_world(&mut commands, &mut sector, seed.value);
}

fn apply_seed_world(commands: &mut Commands, sector: &mut Sector, seed: u64) {
    sector.nodes.clear();
    sector.routes.clear();

    let mut rng = seed;
    let node_count = 50;
    let mut nodes = Vec::with_capacity(node_count);

    for index in 0..node_count {
        let node_id = seed_to_node_id(seed.wrapping_add(index as u64 + 1));
        // First node spawns near origin so player starts in safe zone
        let position = if index == 0 {
            next_starting_position(&mut rng)
        } else {
            next_position(&mut rng)
        };
        let modifier = pick_modifier(&mut rng);
        let node = SystemNode {
            id: node_id,
            position,
            modifier,
        };
        let revealed = index == 0;
        let confidence = if revealed { 0.6 } else { 0.0 };

        commands.spawn((
            node.clone(),
            SystemIntel {
                layer: KnowledgeLayer::Existence,
                confidence,
                last_seen_tick: 0,
                revealed,
                revealed_tick: 0,
            },
            Name::new(format!("SystemNode-{}-{}", seed, node_id)),
            Transform::from_xyz(position.x, position.y, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
        ));

        nodes.push(node);
    }

    // Generate routes using MST + random extras (min 1, max 5 connections per node)
    let routes = generate_routes(&nodes, &mut rng);
    sector.routes = routes;
    sector.nodes = nodes.clone();

    // Spawn jump gates for each route (one at each end)
    spawn_jump_gates(commands, &nodes, &sector.routes);

    spawn_starting_entities(commands, sector);
    spawn_stations(commands, sector, &mut rng);
    spawn_pirates(commands, sector, &mut rng);
}

#[allow(clippy::too_many_arguments)]
fn handle_seed_input(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut seed: ResMut<WorldSeed>,
    mut sector: ResMut<Sector>,
    nodes: Query<Entity, With<SystemNode>>,
    stations: Query<Entity, With<Station>>,
    ships: Query<Entity, With<Ship>>,
) {
    if !shift_pressed(&input) {
        return;
    }

    let mut updated = false;

    if input.just_pressed(bindings.seed_up) {
        seed.value = seed.value.saturating_add(1);
        updated = true;
    }

    if input.just_pressed(bindings.seed_down) {
        seed.value = seed.value.saturating_sub(1);
        updated = true;
    }

    if updated {
        for entity in nodes.iter() {
            commands.entity(entity).despawn();
        }
        for entity in stations.iter() {
            commands.entity(entity).despawn();
        }
        for entity in ships.iter() {
            commands.entity(entity).despawn();
        }

        apply_seed_world(&mut commands, &mut sector, seed.value);
        info!("World seed updated: {}", seed.value);
    }
}

fn handle_modifier_randomize(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    ticks: Res<SimTickCount>,
    mut sector: ResMut<Sector>,
    mut nodes: Query<&mut SystemNode>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.randomize_modifiers) {
        return;
    }

    let mut rng = ticks.tick.wrapping_add(sector.nodes.len() as u64);

    for mut node in nodes.iter_mut() {
        let modifier = pick_modifier(&mut rng);
        node.modifier = modifier;
        update_sector_modifier(&mut sector, node.id, modifier);
    }

    info!("Zone modifiers randomized");
}

fn handle_reveal_adjacent(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    sector: Res<Sector>,
    ticks: Res<SimTickCount>,
    mut nodes: Query<(&SystemNode, &mut SystemIntel)>,
    mut log: ResMut<EventLog>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.reveal_adjacent) {
        return;
    }

    let mut revealed = std::collections::HashSet::new();
    for (node, intel) in nodes.iter() {
        if intel.revealed {
            revealed.insert(node.id);
        }
    }

    let mut to_reveal = revealed.clone();
    for route in &sector.routes {
        if revealed.contains(&route.from) || revealed.contains(&route.to) {
            to_reveal.insert(route.from);
            to_reveal.insert(route.to);
        }
    }

    for (node, mut intel) in nodes.iter_mut() {
        if to_reveal.contains(&node.id) && !intel.revealed {
            intel.revealed = true;
            intel.confidence = 0.4;
            intel.revealed_tick = ticks.tick;
        }
    }

    log.push("Reveal adjacent nodes".to_string());
}

fn handle_debug_spawns(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    if !shift_pressed(&input) {
        return;
    }

    let mut target_node = None;
    for (node, intel) in nodes.iter() {
        if intel.revealed {
            target_node = Some(node);
            break;
        }
    }

    let node = match target_node {
        Some(node) => node,
        None => {
            return;
        }
    };

    // Spawn FuelDepot with Shift+B
    if input.just_pressed(bindings.spawn_station) {
        spawn_station_debug(&mut commands, node, StationKind::FuelDepot);
    }

    // Spawn Refinery with Shift+1
    if input.just_pressed(bindings.spawn_refinery) {
        spawn_station_debug(&mut commands, node, StationKind::Refinery);
    }

    // Spawn Shipyard with Shift+2
    if input.just_pressed(bindings.spawn_shipyard) {
        spawn_station_debug(&mut commands, node, StationKind::Shipyard);
    }

    // Spawn Outpost with Shift+3
    if input.just_pressed(bindings.spawn_outpost) {
        spawn_outpost_debug(&mut commands, node);
    }

    if input.just_pressed(bindings.spawn_ship) {
        spawn_ship_stub(&mut commands, node);
    }

    if input.just_pressed(bindings.spawn_pirate) {
        spawn_pirate(&mut commands, node);
    }
}

fn handle_reveal_all(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    ticks: Res<SimTickCount>,
    mut nodes: Query<&mut SystemIntel>,
    mut log: ResMut<EventLog>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.reveal_all) {
        return;
    }

    for mut intel in nodes.iter_mut() {
        intel.revealed = true;
        if intel.confidence < 0.5 {
            intel.confidence = 0.5;
        }
        intel.revealed_tick = ticks.tick;
    }

    log.push("Reveal all nodes".to_string());
}

fn handle_clear_reveal(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    ticks: Res<SimTickCount>,
    mut nodes: Query<&mut SystemIntel>,
    mut log: ResMut<EventLog>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.clear_reveal) {
        return;
    }

    let mut first = true;
    for mut intel in nodes.iter_mut() {
        if first {
            intel.revealed = true;
            intel.confidence = 0.6;
            intel.revealed_tick = ticks.tick;
            first = false;
        } else {
            intel.revealed = false;
            intel.confidence = 0.0;
            intel.revealed_tick = 0;
        }
    }

    log.push("Clear reveals".to_string());
}

fn update_sector_modifier(sector: &mut Sector, id: u32, modifier: Option<ZoneModifier>) {
    for node in &mut sector.nodes {
        if node.id == id {
            node.modifier = modifier;
            return;
        }
    }
}

fn spawn_starting_entities(commands: &mut Commands, sector: &Sector) {
    let first = match sector.nodes.first() {
        Some(node) => node,
        None => {
            return;
        }
    };

    spawn_player_ship(commands, first);
}

fn spawn_player_ship(commands: &mut Commands, node: &SystemNode) {
    let capacity = ship_fuel_capacity(ShipKind::PlayerShip);
    // Spawn player at zone center - safe from gates (300 units out) and asteroid fields (800+ units out)
    let x = node.position.x;
    let y = node.position.y;

    info!(
        "Spawning player ship at ({:.1}, {:.1}, 0.4) in zone {}",
        x, y, node.id
    );

    commands.spawn((
        Ship {
            kind: ShipKind::PlayerShip,
            state: ShipState::Idle,
            fuel: capacity * 0.9,
            fuel_capacity: capacity,
        },
        Cargo::default(),
        Credits::default(),
        Velocity::default(),
        Faction::Player,
        PlayerControl,
        ShipFuelAlert::default(),
        BoundaryWarningState::default(),
        ZoneId(node.id),
        Name::new("Ship-Player"),
        SpatialBundle::from_transform(Transform::from_xyz(x, y, 0.4)),
    ));
}

fn spawn_ship_stub(commands: &mut Commands, node: &SystemNode) {
    let scout_capacity = ship_fuel_capacity(ShipKind::Scout);

    commands.spawn((
        Ship {
            kind: ShipKind::Scout,
            state: ShipState::Idle,
            fuel: scout_capacity * 0.7,
            fuel_capacity: scout_capacity,
        },
        Cargo {
            ore: 0,
            ore_capacity: cargo_capacity(ShipKind::Scout) as u32,
            fuel: 0.0,
            fuel_capacity: 20.0,
        },
        Fleet {
            role: ship_default_role(ShipKind::Scout),
        },
        Faction::Player,
        ScoutBehavior::new(node.id, RiskTolerance::Cautious),
        ShipFuelAlert::default(),
        ZoneId(node.id),
        Name::new("Ship-Scout"),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x - 24.0,
            node.position.y - 10.0,
            0.4,
        )),
    ));
}

/// Spawn a station of the given kind at the given node (debug command)
fn spawn_station_debug(commands: &mut Commands, node: &SystemNode, kind: StationKind) {
    let capacity = station_fuel_capacity(kind);
    let build_time = station_build_time_seconds(kind);

    commands.spawn((
        Station {
            kind,
            state: StationState::Deploying,
            fuel: capacity * 0.5,
            fuel_capacity: capacity,
        },
        Faction::Player,
        StationBuild {
            remaining_seconds: build_time,
        },
        StationCrisisLog::default(),
        ZoneId(node.id),
        Name::new(format!("Station-{:?}-{}", kind, node.id)),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + 40.0,
            node.position.y + 20.0,
            0.5,
        )),
    ));
}

/// Spawn an NPC Outpost at the given node (debug command)
fn spawn_outpost_debug(commands: &mut Commands, node: &SystemNode) {
    commands.spawn((
        Station {
            kind: StationKind::Outpost,
            state: StationState::Operational,
            fuel: 0.0,
            fuel_capacity: 0.0,
        },
        Faction::Independent,
        StationCrisisLog::default(),
        ZoneId(node.id),
        Name::new(format!("Outpost-Debug-{}", node.id)),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + 30.0,
            node.position.y - 50.0,
            0.5,
        )),
    ));
}

fn spawn_pirate(commands: &mut Commands, node: &SystemNode) {
    commands.spawn((
        PirateShip {
            speed: 70.0,
            behavior: crate::pirates::PirateShipBehavior::default(),
        },
        Faction::Pirate,
        ZoneId(node.id),
        Name::new("Pirate-Ship"),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + 24.0,
            node.position.y + 10.0,
            0.4,
        )),
    ));
}

/// Spawn refineries and shipyards on non-starter zones
fn spawn_stations(commands: &mut Commands, sector: &Sector, rng: &mut u64) {
    let safe_zones = get_safe_zones(sector);

    // Get eligible zones (not starter zone, not adjacent to starter)
    let eligible_nodes: Vec<&SystemNode> = sector
        .nodes
        .iter()
        .filter(|n| !safe_zones.contains(&n.id))
        .collect();

    if eligible_nodes.is_empty() {
        return;
    }

    // Spawn 2-3 Refineries
    let refinery_count = 2 + (next_unit(rng) * 2.0) as usize; // 2-3
    let refinery_count = refinery_count.min(eligible_nodes.len());

    let mut used_zones = std::collections::HashSet::new();

    for i in 0..refinery_count {
        // Pick a random eligible node that hasn't been used
        let available: Vec<&SystemNode> = eligible_nodes
            .iter()
            .filter(|n| !used_zones.contains(&n.id))
            .copied()
            .collect();

        if available.is_empty() {
            break;
        }

        let index = (next_unit(rng) * available.len() as f32) as usize;
        let index = index.min(available.len().saturating_sub(1));
        let node = available[index];
        used_zones.insert(node.id);

        let kind = StationKind::Refinery;
        let capacity = station_fuel_capacity(kind);
        let offset_x = 50.0 + next_unit(rng) * 50.0;
        let offset_y = 30.0 + next_unit(rng) * 40.0;

        commands.spawn((
            Station {
                kind,
                state: StationState::Operational,
                fuel: capacity * 0.5,
                fuel_capacity: capacity,
            },
            Faction::Player,
            StationCrisisLog::default(),
            ZoneId(node.id),
            Name::new(format!("Refinery-{}-{}", node.id, i)),
            SpatialBundle::from_transform(Transform::from_xyz(
                node.position.x + offset_x,
                node.position.y + offset_y,
                0.5,
            )),
        ));
    }

    // Spawn 1-2 Shipyards
    let shipyard_count = 1 + (next_unit(rng) * 2.0) as usize; // 1-2
    let shipyard_count = shipyard_count.min(eligible_nodes.len().saturating_sub(used_zones.len()));

    for i in 0..shipyard_count {
        let available: Vec<&SystemNode> = eligible_nodes
            .iter()
            .filter(|n| !used_zones.contains(&n.id))
            .copied()
            .collect();

        if available.is_empty() {
            break;
        }

        let index = (next_unit(rng) * available.len() as f32) as usize;
        let index = index.min(available.len().saturating_sub(1));
        let node = available[index];
        used_zones.insert(node.id);

        let kind = StationKind::Shipyard;
        let capacity = station_fuel_capacity(kind);
        let offset_x = -50.0 - next_unit(rng) * 50.0;
        let offset_y = 30.0 + next_unit(rng) * 40.0;

        commands.spawn((
            Station {
                kind,
                state: StationState::Operational,
                fuel: capacity * 0.5,
                fuel_capacity: capacity,
            },
            Faction::Player,
            StationCrisisLog::default(),
            ZoneId(node.id),
            Name::new(format!("Shipyard-{}-{}", node.id, i)),
            SpatialBundle::from_transform(Transform::from_xyz(
                node.position.x + offset_x,
                node.position.y + offset_y,
                0.5,
            )),
        ));
    }

    // Spawn Outposts (~40-50% of zones, including safe zones)
    spawn_outposts(commands, sector, rng);
}

/// Spawn NPC Outposts across the sector
fn spawn_outposts(commands: &mut Commands, sector: &Sector, rng: &mut u64) {
    let safe_zones = get_safe_zones(sector);

    // Guarantee at least one Outpost in a safe zone (starter area)
    let safe_nodes: Vec<&SystemNode> = sector
        .nodes
        .iter()
        .filter(|n| safe_zones.contains(&n.id))
        .collect();

    let mut outpost_zones = std::collections::HashSet::new();

    // Spawn one guaranteed Outpost near starter
    if let Some(&node) = safe_nodes.first() {
        spawn_outpost_at(commands, node, 0, rng);
        outpost_zones.insert(node.id);
    }

    // ~40-50% of remaining zones get Outposts
    let target_ratio = 0.4 + next_unit(rng) * 0.1; // 0.4-0.5
    let target_count = (sector.nodes.len() as f32 * target_ratio) as usize;
    let remaining_count = target_count.saturating_sub(1); // Already placed one

    let other_nodes: Vec<&SystemNode> = sector
        .nodes
        .iter()
        .filter(|n| !outpost_zones.contains(&n.id))
        .collect();

    for (i, node) in other_nodes.iter().enumerate() {
        if outpost_zones.len() >= target_count {
            break;
        }

        // Random chance to place an Outpost
        if next_unit(rng) < 0.45 {
            spawn_outpost_at(commands, node, i + 1, rng);
            outpost_zones.insert(node.id);
        }
    }

    // Ensure we hit minimum target if we haven't yet
    if outpost_zones.len() < remaining_count {
        for node in other_nodes.iter() {
            if outpost_zones.len() >= target_count {
                break;
            }
            if !outpost_zones.contains(&node.id) {
                spawn_outpost_at(commands, node, outpost_zones.len(), rng);
                outpost_zones.insert(node.id);
            }
        }
    }
}

fn spawn_outpost_at(commands: &mut Commands, node: &SystemNode, index: usize, rng: &mut u64) {
    // Outposts are always Operational and don't consume fuel
    let offset_x = -30.0 + next_unit(rng) * 60.0;
    let offset_y = -80.0 - next_unit(rng) * 40.0;

    commands.spawn((
        Station {
            kind: StationKind::Outpost,
            state: StationState::Operational,
            fuel: 0.0,
            fuel_capacity: 0.0,
        },
        Faction::Independent,
        StationCrisisLog::default(),
        ZoneId(node.id),
        Name::new(format!("Outpost-{}", index)),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x + offset_x,
            node.position.y + offset_y,
            0.5,
        )),
    ));
}

/// Returns the set of zone IDs that are within 1 jump of the starter zone
fn get_safe_zones(sector: &Sector) -> std::collections::HashSet<u32> {
    let mut safe_zones = std::collections::HashSet::new();

    let starter_id = match sector.nodes.first() {
        Some(node) => node.id,
        None => return safe_zones,
    };

    safe_zones.insert(starter_id);

    for route in &sector.routes {
        if route.from == starter_id {
            safe_zones.insert(route.to);
        }
        if route.to == starter_id {
            safe_zones.insert(route.from);
        }
    }

    safe_zones
}

fn spawn_pirates(commands: &mut Commands, sector: &Sector, rng: &mut u64) {
    let safe_zones = get_safe_zones(sector);

    // Get eligible zones (2+ jumps from starter)
    let eligible_nodes: Vec<&SystemNode> = sector
        .nodes
        .iter()
        .filter(|n| !safe_zones.contains(&n.id))
        .collect();

    if eligible_nodes.is_empty() {
        return;
    }

    // ~10% of eligible zones should have pirates
    let target_pirate_zone_count = (eligible_nodes.len() as f32 * 0.10).ceil() as usize;
    let target_pirate_zone_count = target_pirate_zone_count.max(1);

    // Randomly select zones for pirate presence
    let mut selected_zones: Vec<&SystemNode> = Vec::new();
    for node in &eligible_nodes {
        if selected_zones.len() >= target_pirate_zone_count {
            break;
        }
        let roll = next_unit(rng);
        if roll < 0.15 {
            // Slightly higher chance to reach ~10% target
            selected_zones.push(node);
        }
    }

    // If we didn't get enough, pick some deterministically
    if selected_zones.is_empty() && !eligible_nodes.is_empty() {
        selected_zones.push(eligible_nodes[0]);
    }

    // Spawn pirates in selected zones
    for node in selected_zones {
        // Spawn 0-5 pirates per zone
        let pirate_count = (next_unit(rng) * 6.0) as usize;

        for i in 0..pirate_count {
            let angle = next_unit(rng) * std::f32::consts::TAU;
            // 5x zone scale: 150-400 radius
            let radius = 150.0 + next_unit(rng) * 250.0;
            let offset_x = angle.cos() * radius;
            let offset_y = angle.sin() * radius;

            commands.spawn((
                PirateShip {
                    speed: 70.0,
                    behavior: crate::pirates::PirateShipBehavior::default(),
                },
                Faction::Pirate,
                ZoneId(node.id),
                Name::new(format!("Pirate-Ship-{}-{}", node.id, i)),
                SpatialBundle::from_transform(Transform::from_xyz(
                    node.position.x + offset_x,
                    node.position.y + offset_y,
                    0.4,
                )),
            ));
        }

        // Max 1 pirate base per zone, ~50% chance for a zone with pirates to have a base
        let spawn_base = next_unit(rng) < 0.5;
        if spawn_base {
            commands.spawn((
                PirateBase {
                    launch_interval_ticks: 300,
                    next_launch_tick: 120,
                },
                Faction::Pirate,
                ZoneId(node.id),
                Name::new(format!("Pirate-Base-{}", node.id)),
                // 5x zone scale: offset 250, -150
                SpatialBundle::from_transform(Transform::from_xyz(
                    node.position.x + 250.0,
                    node.position.y - 150.0,
                    0.45,
                )),
            ));
        }
    }
}

/// Distance from node center to place gate (scaled for 5x zone size)
const GATE_OFFSET: f32 = 300.0;

fn spawn_jump_gates(commands: &mut Commands, nodes: &[SystemNode], routes: &[RouteEdge]) {
    // Create a map of node id -> node for quick lookup
    let node_map: std::collections::HashMap<u32, &SystemNode> =
        nodes.iter().map(|n| (n.id, n)).collect();

    for route in routes {
        let Some(from_node) = node_map.get(&route.from) else {
            continue;
        };
        let Some(to_node) = node_map.get(&route.to) else {
            continue;
        };

        // Calculate direction from source to destination
        let direction = (to_node.position - from_node.position).normalize_or_zero();

        // Spawn gate at source node (pointing to destination)
        let from_gate_pos = from_node.position + direction * GATE_OFFSET;
        commands.spawn((
            JumpGate {
                source_zone: route.from,
                destination_zone: route.to,
            },
            ZoneId(route.from),
            Name::new(format!("JumpGate-{}-to-{}", route.from, route.to)),
            SpatialBundle::from_transform(Transform::from_xyz(
                from_gate_pos.x,
                from_gate_pos.y,
                0.3,
            )),
        ));

        // Spawn gate at destination node (pointing back to source)
        let to_gate_pos = to_node.position - direction * GATE_OFFSET;
        commands.spawn((
            JumpGate {
                source_zone: route.to,
                destination_zone: route.from,
            },
            ZoneId(route.to),
            Name::new(format!("JumpGate-{}-to-{}", route.to, route.from)),
            SpatialBundle::from_transform(Transform::from_xyz(to_gate_pos.x, to_gate_pos.y, 0.3)),
        ));
    }
}

fn seed_to_node_id(seed: u64) -> u32 {
    if seed > u64::from(u32::MAX) {
        (seed % u64::from(u32::MAX)) as u32
    } else {
        seed as u32
    }
}

fn pick_modifier(state: &mut u64) -> Option<ZoneModifier> {
    let roll = next_unit(state);

    if roll < 0.35 {
        return None;
    }

    let select = next_unit(state);

    if select < 0.25 {
        Some(ZoneModifier::HighRadiation)
    } else if select < 0.5 {
        Some(ZoneModifier::NebulaInterference)
    } else if select < 0.75 {
        Some(ZoneModifier::RichOreVeins)
    } else {
        Some(ZoneModifier::ClearSignals)
    }
}

fn next_position(state: &mut u64) -> Vec2 {
    let x = next_unit(state);
    let y = next_unit(state);

    // 5x zone scale: ±7500 x ±5000
    Vec2::new(
        scale_to_range(x, -7500.0, 7500.0),
        scale_to_range(y, -5000.0, 5000.0),
    )
}

/// Generate a position near origin for the starting node
/// Keeps player spawn well within safe boundaries (warning at 6000)
fn next_starting_position(state: &mut u64) -> Vec2 {
    let x = next_unit(state);
    let y = next_unit(state);

    // 5x zone scale: ±2000 x ±1500
    Vec2::new(
        scale_to_range(x, -2000.0, 2000.0),
        scale_to_range(y, -1500.0, 1500.0),
    )
}

fn next_unit(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let value = (*state >> 33) as u32;
    (value as f32) / (u32::MAX as f32)
}

fn scale_to_range(value: f32, min: f32, max: f32) -> f32 {
    min + (max - min) * value
}

// =============================================================================
// Route Generation (MST + Random Extras)
// =============================================================================

const MAX_CONNECTIONS_PER_NODE: usize = 5;

/// Generate routes ensuring all nodes are connected (MST) with random extra connections.
/// - Minimum 1 connection per node (guaranteed by MST)
/// - Maximum 5 connections per node
/// - Deterministic based on RNG state
fn generate_routes(nodes: &[SystemNode], rng: &mut u64) -> Vec<RouteEdge> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let mut routes = Vec::new();
    let mut connection_count: std::collections::HashMap<u32, usize> =
        std::collections::HashMap::new();

    // Initialize connection counts
    for node in nodes {
        connection_count.insert(node.id, 0);
    }

    // Build all possible edges with distances
    let mut all_edges: Vec<(usize, usize, f32)> = Vec::new();
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let dist = nodes[i].position.distance(nodes[j].position);
            all_edges.push((i, j, dist));
        }
    }

    // Sort edges by distance for MST (Kruskal's algorithm)
    all_edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    // Union-Find for MST
    let mut parent: Vec<usize> = (0..nodes.len()).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let root_a = find(parent, a);
        let root_b = find(parent, b);
        if root_a != root_b {
            parent[root_a] = root_b;
        }
    }

    // Phase 1: Build MST (ensures all nodes connected with minimum edges)
    let mut mst_edges = Vec::new();
    for &(i, j, dist) in &all_edges {
        if find(&mut parent, i) != find(&mut parent, j) {
            union(&mut parent, i, j);
            mst_edges.push((i, j, dist));

            let id_a = nodes[i].id;
            let id_b = nodes[j].id;
            *connection_count.get_mut(&id_a).unwrap() += 1;
            *connection_count.get_mut(&id_b).unwrap() += 1;

            let risk = next_unit(rng);
            routes.push(RouteEdge {
                from: id_a,
                to: id_b,
                distance: dist,
                risk,
            });

            if mst_edges.len() == nodes.len() - 1 {
                break;
            }
        }
    }

    // Phase 2: Add random extra edges (respecting max connections)
    // Collect non-MST edges that could be added
    let mst_set: std::collections::HashSet<(usize, usize)> = mst_edges
        .iter()
        .map(|&(i, j, _)| if i < j { (i, j) } else { (j, i) })
        .collect();

    let mut extra_candidates: Vec<(usize, usize, f32)> = all_edges
        .iter()
        .filter(|&&(i, j, _)| {
            let key = if i < j { (i, j) } else { (j, i) };
            !mst_set.contains(&key)
        })
        .copied()
        .collect();

    // Shuffle candidates using Fisher-Yates with seeded RNG
    for i in (1..extra_candidates.len()).rev() {
        let j = (next_unit(rng) * (i + 1) as f32) as usize;
        extra_candidates.swap(i, j.min(i));
    }

    // Decide how many extra edges to add (randomized, typically 20-60% of node count)
    let extra_count_base = (nodes.len() as f32 * (0.2 + next_unit(rng) * 0.4)) as usize;
    let mut extras_added = 0;

    for (i, j, dist) in extra_candidates {
        if extras_added >= extra_count_base {
            break;
        }

        let id_a = nodes[i].id;
        let id_b = nodes[j].id;

        let count_a = *connection_count.get(&id_a).unwrap();
        let count_b = *connection_count.get(&id_b).unwrap();

        // Only add if both nodes have room for more connections
        if count_a < MAX_CONNECTIONS_PER_NODE && count_b < MAX_CONNECTIONS_PER_NODE {
            *connection_count.get_mut(&id_a).unwrap() += 1;
            *connection_count.get_mut(&id_b).unwrap() += 1;

            let risk = next_unit(rng);
            routes.push(RouteEdge {
                from: id_a,
                to: id_b,
                distance: dist,
                risk,
            });

            extras_added += 1;
        }
    }

    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;

    #[test]
    fn seed_to_node_id_wraps_large_values() {
        let seed = u64::from(u32::MAX) + 1;
        assert_eq!(seed_to_node_id(seed), 1);
    }

    #[test]
    fn next_unit_returns_value_in_range() {
        let mut state = 0u64;

        for _ in 0..5 {
            let value = next_unit(&mut state);
            assert!(value >= 0.0);
            assert!(value <= 1.0);
        }
    }

    #[test]
    fn next_position_stays_within_bounds() {
        let mut state = 42u64;

        for _ in 0..10 {
            let position = next_position(&mut state);
            assert!(position.x >= -7500.0);
            assert!(position.x <= 7500.0);
            assert!(position.y >= -5000.0);
            assert!(position.y <= 5000.0);
        }
    }

    #[test]
    fn seed_to_node_id_keeps_small_values() {
        let seed = 42u64;
        assert_eq!(seed_to_node_id(seed), 42);
    }

    #[test]
    fn apply_seed_world_generates_nodes_and_routes() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        assert_eq!(sector.nodes.len(), 50);
        // MST has n-1 = 49 routes minimum, plus random extras
        assert!(sector.routes.len() >= 49, "Need at least MST routes");
        assert!(
            sector.routes.len() <= 100,
            "Should not exceed reasonable route count"
        );
    }

    #[test]
    fn spawn_pirate_creates_pirate_ship_entity() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let node = SystemNode {
            id: 1,
            position: Vec2::new(100.0, 200.0),
            modifier: None,
        };

        spawn_pirate(&mut commands, &node);
        queue.apply(&mut world);

        // Verify a PirateShip entity was created
        let mut query = world.query::<&PirateShip>();
        let pirates: Vec<_> = query.iter(&world).collect();
        assert_eq!(pirates.len(), 1);
        assert_eq!(pirates[0].speed, 70.0);
    }

    #[test]
    fn sector_generates_with_50_nodes() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        assert_eq!(sector.nodes.len(), 50);
    }

    #[test]
    fn starter_zone_has_no_pirates() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        let starter_zone_id = sector.nodes.first().unwrap().id;

        // Check for pirate ships in starter zone
        let mut pirate_query = world.query::<(&PirateShip, &ZoneId)>();
        for (_pirate, zone_id) in pirate_query.iter(&world) {
            assert_ne!(
                zone_id.0, starter_zone_id,
                "Starter zone should have no pirates"
            );
        }

        // Check for pirate bases in starter zone
        let mut base_query = world.query::<(&PirateBase, &ZoneId)>();
        for (_base, zone_id) in base_query.iter(&world) {
            assert_ne!(
                zone_id.0, starter_zone_id,
                "Starter zone should have no pirate bases"
            );
        }
    }

    #[test]
    fn zones_one_jump_from_starter_have_no_pirates() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        let starter_zone_id = sector.nodes.first().unwrap().id;

        // Find zones directly connected to starter
        let mut adjacent_zones: std::collections::HashSet<u32> = std::collections::HashSet::new();
        adjacent_zones.insert(starter_zone_id);
        for route in &sector.routes {
            if route.from == starter_zone_id {
                adjacent_zones.insert(route.to);
            }
            if route.to == starter_zone_id {
                adjacent_zones.insert(route.from);
            }
        }

        // Check for pirate ships in adjacent zones
        let mut pirate_query = world.query::<(&PirateShip, &ZoneId)>();
        for (_pirate, zone_id) in pirate_query.iter(&world) {
            assert!(
                !adjacent_zones.contains(&zone_id.0),
                "Zone {} is adjacent to starter and should have no pirates",
                zone_id.0
            );
        }

        // Check for pirate bases in adjacent zones
        let mut base_query = world.query::<(&PirateBase, &ZoneId)>();
        for (_base, zone_id) in base_query.iter(&world) {
            assert!(
                !adjacent_zones.contains(&zone_id.0),
                "Zone {} is adjacent to starter and should have no pirate bases",
                zone_id.0
            );
        }
    }

    #[test]
    fn no_zone_has_more_than_one_pirate_base() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        let mut base_counts: std::collections::HashMap<u32, usize> =
            std::collections::HashMap::new();
        let mut base_query = world.query::<(&PirateBase, &ZoneId)>();
        for (_base, zone_id) in base_query.iter(&world) {
            *base_counts.entry(zone_id.0).or_insert(0) += 1;
        }

        for (zone_id, count) in base_counts {
            assert!(
                count <= 1,
                "Zone {} has {} pirate bases, max is 1",
                zone_id,
                count
            );
        }
    }

    #[test]
    fn pirate_zones_have_zero_to_five_pirates() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        let mut pirate_counts: std::collections::HashMap<u32, usize> =
            std::collections::HashMap::new();
        let mut pirate_query = world.query::<(&PirateShip, &ZoneId)>();
        for (_pirate, zone_id) in pirate_query.iter(&world) {
            *pirate_counts.entry(zone_id.0).or_insert(0) += 1;
        }

        for (zone_id, count) in pirate_counts {
            assert!(
                count <= 5,
                "Zone {} has {} pirates, max is 5",
                zone_id,
                count
            );
        }
    }

    #[test]
    fn approximately_ten_percent_of_eligible_zones_have_pirates() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        let starter_zone_id = sector.nodes.first().unwrap().id;

        // Find zones within 1 jump of starter (ineligible for pirates)
        let mut safe_zones: std::collections::HashSet<u32> = std::collections::HashSet::new();
        safe_zones.insert(starter_zone_id);
        for route in &sector.routes {
            if route.from == starter_zone_id {
                safe_zones.insert(route.to);
            }
            if route.to == starter_zone_id {
                safe_zones.insert(route.from);
            }
        }

        let eligible_zone_count = sector.nodes.len() - safe_zones.len();

        // Count zones with pirates
        let mut zones_with_pirates: std::collections::HashSet<u32> =
            std::collections::HashSet::new();
        let mut pirate_query = world.query::<(&PirateShip, &ZoneId)>();
        for (_pirate, zone_id) in pirate_query.iter(&world) {
            zones_with_pirates.insert(zone_id.0);
        }
        let mut base_query = world.query::<(&PirateBase, &ZoneId)>();
        for (_base, zone_id) in base_query.iter(&world) {
            zones_with_pirates.insert(zone_id.0);
        }

        let pirate_zone_count = zones_with_pirates.len();

        // Allow some variance: expect ~10% but accept 5-20% range
        let expected_min = (eligible_zone_count as f32 * 0.05) as usize;
        let expected_max = (eligible_zone_count as f32 * 0.20) as usize;

        assert!(
            pirate_zone_count >= expected_min && pirate_zone_count <= expected_max,
            "Expected {}-{} pirate zones (10% of {} eligible), got {}",
            expected_min,
            expected_max,
            eligible_zone_count,
            pirate_zone_count
        );
    }

    #[test]
    fn every_node_has_one_to_five_connections() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut sector = Sector::default();

        apply_seed_world(&mut commands, &mut sector, 1337);
        queue.apply(&mut world);

        // Count connections per node
        let mut connection_counts: std::collections::HashMap<u32, usize> =
            std::collections::HashMap::new();

        for node in &sector.nodes {
            connection_counts.insert(node.id, 0);
        }

        for route in &sector.routes {
            *connection_counts.get_mut(&route.from).unwrap() += 1;
            *connection_counts.get_mut(&route.to).unwrap() += 1;
        }

        for (node_id, count) in &connection_counts {
            assert!(
                *count >= 1,
                "Node {} has {} connections, minimum is 1",
                node_id,
                count
            );
            assert!(
                *count <= 5,
                "Node {} has {} connections, maximum is 5",
                node_id,
                count
            );
        }
    }

    #[test]
    fn same_seed_produces_identical_routes() {
        let seed = 42u64;

        let mut world1 = World::default();
        let mut queue1 = CommandQueue::default();
        let mut commands1 = Commands::new(&mut queue1, &world1);
        let mut sector1 = Sector::default();
        apply_seed_world(&mut commands1, &mut sector1, seed);
        queue1.apply(&mut world1);

        let mut world2 = World::default();
        let mut queue2 = CommandQueue::default();
        let mut commands2 = Commands::new(&mut queue2, &world2);
        let mut sector2 = Sector::default();
        apply_seed_world(&mut commands2, &mut sector2, seed);
        queue2.apply(&mut world2);

        assert_eq!(
            sector1.routes.len(),
            sector2.routes.len(),
            "Same seed should produce same number of routes"
        );

        for (r1, r2) in sector1.routes.iter().zip(sector2.routes.iter()) {
            assert_eq!(r1.from, r2.from, "Route 'from' should match");
            assert_eq!(r1.to, r2.to, "Route 'to' should match");
        }
    }

    #[test]
    fn different_seeds_produce_different_route_counts() {
        let mut route_counts = std::collections::HashSet::new();

        for seed in [100u64, 200, 300, 400, 500] {
            let mut world = World::default();
            let mut queue = CommandQueue::default();
            let mut commands = Commands::new(&mut queue, &world);
            let mut sector = Sector::default();
            apply_seed_world(&mut commands, &mut sector, seed);
            queue.apply(&mut world);

            route_counts.insert(sector.routes.len());
        }

        // With different seeds, we should see at least 2 different route counts
        assert!(
            route_counts.len() >= 2,
            "Different seeds should produce varying route counts, got {:?}",
            route_counts
        );
    }
}
