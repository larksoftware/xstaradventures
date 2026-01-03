use bevy::prelude::*;

use crate::compat::SpatialBundle;

use crate::fleets::{RiskTolerance, ScoutBehavior};
use crate::pirates::PirateBase;
use crate::plugins::core::{DebugWindow, EventLog, GameState, InputBindings};
use crate::plugins::player::PlayerControl;
use crate::plugins::sim::{BoundaryWarningState, SimTickCount};
use crate::ships::{
    cargo_capacity, ship_default_role, ship_fuel_capacity, Cargo, Fleet, Ship, ShipFuelAlert,
    ShipKind, ShipState, Velocity,
};
use crate::stations::{
    station_build_time_seconds, station_fuel_capacity, Station, StationBuild, StationCrisisLog,
    StationKind, StationState,
};
use crate::world::{KnowledgeLayer, RouteEdge, Sector, SystemIntel, SystemNode, ZoneModifier};

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
    let node_count = 5;
    let mut nodes = Vec::with_capacity(node_count);

    for index in 0..node_count {
        let node_id = seed_to_node_id(seed.wrapping_add(index as u64 + 1));
        let position = next_position(&mut rng);
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

    for index in 0..nodes.len() {
        let a = &nodes[index];
        let b = &nodes[(index + 1) % nodes.len()];
        let distance = a.position.distance(b.position);
        let risk = next_unit(&mut rng);

        sector.routes.push(RouteEdge {
            from: a.id,
            to: b.id,
            distance,
            risk,
        });
    }

    sector.nodes = nodes;

    spawn_starting_entities(commands, sector);
    spawn_pirate_base(commands, sector);
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
    if !input.just_pressed(bindings.randomize_modifiers) {
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
    if !input.just_pressed(bindings.reveal_adjacent) {
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

    if input.just_pressed(bindings.spawn_station) {
        let kind = StationKind::FuelDepot;
        let capacity = station_fuel_capacity(kind);
        let build_time = station_build_time_seconds(kind);

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
            StationCrisisLog::default(),
            Name::new(format!("Station-Spawned-{}", node.id)),
            SpatialBundle::from_transform(Transform::from_xyz(
                node.position.x + 40.0,
                node.position.y + 20.0,
                0.5,
            )),
        ));
    }

    if input.just_pressed(bindings.spawn_ship) {
        spawn_ship_stub(&mut commands, node);
    }
}

fn handle_reveal_all(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    ticks: Res<SimTickCount>,
    mut nodes: Query<&mut SystemIntel>,
    mut log: ResMut<EventLog>,
) {
    if !input.just_pressed(bindings.reveal_all) {
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
    if !input.just_pressed(bindings.clear_reveal) {
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
    let x = node.position.x - 8.0;
    let y = node.position.y - 24.0;

    info!("Spawning player ship at ({:.1}, {:.1}, 0.4)", x, y);

    commands.spawn((
        Ship {
            kind: ShipKind::PlayerShip,
            state: ShipState::Idle,
            fuel: capacity * 0.9,
            fuel_capacity: capacity,
        },
        Cargo {
            common_ore: 0.0,
            capacity: cargo_capacity(ShipKind::PlayerShip),
        },
        Velocity::default(),
        PlayerControl,
        ShipFuelAlert::default(),
        BoundaryWarningState::default(),
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
            common_ore: 0.0,
            capacity: cargo_capacity(ShipKind::Scout),
        },
        Fleet {
            role: ship_default_role(ShipKind::Scout),
        },
        ScoutBehavior {
            risk: RiskTolerance::Balanced,
            current_node: node.id,
            target_node: None,
            next_decision_tick: 0,
        },
        ShipFuelAlert::default(),
        Name::new("Ship-Scout"),
        SpatialBundle::from_transform(Transform::from_xyz(
            node.position.x - 24.0,
            node.position.y - 10.0,
            0.4,
        )),
    ));
}

fn spawn_pirate_base(commands: &mut Commands, sector: &Sector) {
    let target = match sector.nodes.last() {
        Some(node) => node,
        None => {
            return;
        }
    };

    commands.spawn((
        PirateBase {
            launch_interval_ticks: 300,
            next_launch_tick: 120,
        },
        Name::new("Pirate-Base"),
        SpatialBundle::from_transform(Transform::from_xyz(
            target.position.x + 50.0,
            target.position.y - 30.0,
            0.45,
        )),
    ));
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

    Vec2::new(
        scale_to_range(x, -1500.0, 1500.0),
        scale_to_range(y, -1000.0, 1000.0),
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
            assert!(position.x >= -1500.0);
            assert!(position.x <= 1500.0);
            assert!(position.y >= -1000.0);
            assert!(position.y <= 1000.0);
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

        assert_eq!(sector.nodes.len(), 5);
        assert_eq!(sector.routes.len(), 5);
    }
}
