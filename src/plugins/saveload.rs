use bevy::prelude::*;

use crate::plugins::core::{EventLog, GameState, InputBindings};
use crate::ships::{ship_default_role, Fleet, FleetRole, Ship, ShipFuelAlert, ShipKind, ShipState};
use crate::world::{
    KnowledgeLayer, RouteEdge, Sector, SystemIntel, SystemNode, ZoneModifier,
};
use crate::stations::{
    CrisisStage, CrisisType, Station, StationBuild, StationCrisis, StationKind, StationState,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_save_request, handle_load_request).run_if(in_state(GameState::InGame)),
        );
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SaveSector {
    nodes: Vec<SaveNode>,
    routes: Vec<RouteEdge>,
    #[serde(default)]
    intel: Vec<SaveIntel>,
    #[serde(default)]
    stations: Vec<SaveStation>,
    #[serde(default)]
    ships: Vec<SaveShip>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SaveNode {
    id: u32,
    x: f32,
    y: f32,
    #[serde(default)]
    modifier: Option<ZoneModifier>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SaveIntel {
    id: u32,
    layer: KnowledgeLayer,
    confidence: f32,
    last_seen_tick: u64,
    #[serde(default)]
    revealed: bool,
    #[serde(default)]
    revealed_tick: u64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SaveStation {
    kind: StationKind,
    state: StationState,
    x: f32,
    y: f32,
    fuel: f32,
    fuel_capacity: f32,
    #[serde(default)]
    build_remaining: f32,
    #[serde(default)]
    crisis_type: Option<CrisisType>,
    #[serde(default)]
    crisis_stage: Option<CrisisStage>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SaveShip {
    kind: ShipKind,
    state: ShipState,
    #[serde(default)]
    role: FleetRole,
    x: f32,
    y: f32,
    fuel: f32,
    fuel_capacity: f32,
}

impl SaveSector {
    fn from_sector(
        sector: &Sector,
        intel_map: &HashMap<u32, &SystemIntel>,
        stations: &[SaveStation],
        ships: &[SaveShip],
    ) -> Self {
        let nodes = sector
            .nodes
            .iter()
            .map(|node| SaveNode {
                id: node.id,
                x: node.position.x,
                y: node.position.y,
                modifier: node.modifier,
            })
            .collect();

        let intel = sector
            .nodes
            .iter()
            .map(|node| {
                match intel_map.get(&node.id) {
                    Some(intel) => SaveIntel {
                        id: node.id,
                        layer: intel.layer,
                        confidence: intel.confidence,
                        last_seen_tick: intel.last_seen_tick,
                        revealed: intel.revealed,
                        revealed_tick: intel.revealed_tick,
                    },
                    None => SaveIntel {
                        id: node.id,
                        layer: KnowledgeLayer::Existence,
                        confidence: 0.5,
                        last_seen_tick: 0,
                        revealed: false,
                        revealed_tick: 0,
                    },
                }
            })
            .collect();

        Self {
            nodes,
            routes: sector.routes.clone(),
            intel,
            stations: stations.to_vec(),
            ships: ships.to_vec(),
        }
    }
}

fn handle_save_request(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    sector: Res<Sector>,
    intel_query: Query<(&SystemNode, &SystemIntel)>,
    station_query: Query<(&Station, &Transform, Option<&StationBuild>, Option<&StationCrisis>)>,
    ship_query: Query<(&Ship, &Transform, Option<&Fleet>)>,
    mut log: ResMut<EventLog>,
) {
    if input.just_pressed(bindings.save) {
        let mut intel_map = HashMap::new();

        for (node, intel) in intel_query.iter() {
            intel_map.insert(node.id, intel);
        }

        let stations = station_query
            .iter()
            .map(|(station, transform, build, crisis)| SaveStation {
                kind: station.kind,
                state: station.state,
                x: transform.translation.x,
                y: transform.translation.y,
                fuel: station.fuel,
                fuel_capacity: station.fuel_capacity,
                build_remaining: build.map_or(0.0, |build| build.remaining_seconds),
                crisis_type: crisis.map(|crisis| crisis.crisis_type),
                crisis_stage: crisis.map(|crisis| crisis.stage),
            })
            .collect::<Vec<_>>();

        let ships = ship_query
            .iter()
            .map(|(ship, transform, fleet)| SaveShip {
                kind: ship.kind,
                state: ship.state,
                role: fleet.map_or(ship_default_role(ship.kind), |fleet| fleet.role),
                x: transform.translation.x,
                y: transform.translation.y,
                fuel: ship.fuel,
                fuel_capacity: ship.fuel_capacity,
            })
            .collect::<Vec<_>>();

        let payload = SaveSector::from_sector(&sector, &intel_map, &stations, &ships);
        let config = ron::ser::PrettyConfig::default();
        let modifier_summary = summarize_modifiers(&sector);

        match ron::ser::to_string_pretty(&payload, config) {
            Ok(serialized) => {
                info!("Save stub created ({} bytes)", serialized.len());
                log.push(format!("Save stub created ({} bytes)", serialized.len()));
                log.push(format!("Modifiers: {}", modifier_summary));
                if let Err(error) = write_save_file(&serialized) {
                    error!("Save write failed: {}", error);
                    log.push(format!("Save write failed: {}", error));
                } else {
                    log.push(format!("Saved to {}", SAVE_PATH));
                }
            }
            Err(error) => {
                error!("Save failed: {}", error);
                log.push(format!("Save failed: {}", error));
            }
        }
    }
}

fn summarize_modifiers(sector: &Sector) -> String {
    let mut counts = std::collections::BTreeMap::new();

    for node in &sector.nodes {
        let key = match node.modifier {
            Some(ZoneModifier::HighRadiation) => "RAD",
            Some(ZoneModifier::NebulaInterference) => "NEB",
            Some(ZoneModifier::RichOreVeins) => "ORE",
            Some(ZoneModifier::ClearSignals) => "CLR",
            None => "NONE",
        };

        let entry = counts.entry(key).or_insert(0u32);
        *entry += 1;
    }

    counts
        .iter()
        .map(|(key, count)| format!("{}:{}", key, count))
        .collect::<Vec<_>>()
        .join(" ")
}

const SAMPLE_RON: &str = r#"
(
    nodes: [
        (id: 1, x: 0.0, y: 0.0, modifier: HighRadiation),
        (id: 2, x: 220.0, y: 0.0, modifier: ClearSignals),
        (id: 3, x: -180.0, y: 120.0, modifier: NebulaInterference),
    ],
    routes: [
        (from: 1, to: 2, distance: 220.0, risk: 0.25),
        (from: 1, to: 3, distance: 215.0, risk: 0.4),
    ],
    intel: [
        (id: 1, layer: Existence, confidence: 0.6, last_seen_tick: 0, revealed: true, revealed_tick: 0),
        (id: 2, layer: Existence, confidence: 0.0, last_seen_tick: 0, revealed: false, revealed_tick: 0),
        (id: 3, layer: Existence, confidence: 0.0, last_seen_tick: 0, revealed: false, revealed_tick: 0),
    ],
    stations: [
        (
            kind: MiningOutpost,
            state: Deploying,
            x: 24.0,
            y: 12.0,
            fuel: 18.0,
            fuel_capacity: 30.0,
            build_remaining: 120.0,
        ),
    ],
    ships: [
        (
            kind: Scout,
            state: Idle,
            role: Scout,
            x: -24.0,
            y: -10.0,
            fuel: 21.0,
            fuel_capacity: 30.0,
        ),
        (
            kind: Miner,
            state: Idle,
            role: Mining,
            x: -36.0,
            y: 8.0,
            fuel: 27.0,
            fuel_capacity: 45.0,
        ),
    ],
)
"#;

const SAVE_PATH: &str = "saves/sector.ron";

fn handle_load_request(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut commands: Commands,
    mut sector: ResMut<Sector>,
    nodes: Query<Entity, With<SystemNode>>,
    stations: Query<Entity, With<Station>>,
    ships: Query<Entity, With<Ship>>,
    mut log: ResMut<EventLog>,
) {
    if input.just_pressed(bindings.load) {
        match load_sector_from_file() {
            Ok(Some(loaded)) => {
                apply_loaded_sector(
                    &mut commands,
                    &mut sector,
                    &loaded,
                    &nodes,
                    &stations,
                    &ships,
                );
                info!(
                    "Loaded sector from {} (nodes: {}, routes: {})",
                    SAVE_PATH,
                    loaded.nodes.len(),
                    loaded.routes.len()
                );
                log.push(format!(
                    "Loaded sector from {} (nodes: {}, routes: {})",
                    SAVE_PATH,
                    loaded.nodes.len(),
                    loaded.routes.len()
                ));
            }
            Ok(None) => match ron::de::from_str::<SaveSector>(SAMPLE_RON) {
                Ok(loaded) => {
                    apply_loaded_sector(
                        &mut commands,
                        &mut sector,
                        &loaded,
                        &nodes,
                        &stations,
                        &ships,
                    );
                    info!(
                        "Loaded stub sector (nodes: {}, routes: {})",
                        loaded.nodes.len(),
                        loaded.routes.len()
                    );
                    log.push(format!(
                        "Loaded stub sector (nodes: {}, routes: {})",
                        loaded.nodes.len(),
                        loaded.routes.len()
                    ));
                }
                Err(error) => {
                    error!("Load failed: {}", error);
                    log.push(format!("Load failed: {}", error));
                }
            },
            Err(error) => {
                error!("Load failed: {}", error);
                log.push(format!("Load failed: {}", error));
            }
        };
    }
}

fn load_sector_from_file() -> Result<Option<SaveSector>, String> {
    let path = Path::new(SAVE_PATH);

    if !path.exists() {
        return Ok(None);
    }

    match fs::read_to_string(path) {
        Ok(contents) => match ron::de::from_str::<SaveSector>(&contents) {
            Ok(loaded) => Ok(Some(loaded)),
            Err(error) => Err(format!("RON parse error: {}", error)),
        },
        Err(error) => Err(format!("Read error: {}", error)),
    }
}

fn write_save_file(contents: &str) -> Result<(), String> {
    let path = Path::new(SAVE_PATH);
    let dir = path.parent().unwrap_or_else(|| Path::new("saves"));

    if let Err(error) = fs::create_dir_all(dir) {
        return Err(format!("Create dir error: {}", error));
    }

    match fs::write(path, contents) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("Write error: {}", error)),
    }
}

fn apply_loaded_sector(
    commands: &mut Commands,
    sector: &mut Sector,
    loaded: &SaveSector,
    nodes: &Query<Entity, With<SystemNode>>,
    stations: &Query<Entity, With<Station>>,
    ships: &Query<Entity, With<Ship>>,
) {
    for entity in nodes.iter() {
        commands.entity(entity).despawn();
    }
    for entity in stations.iter() {
        commands.entity(entity).despawn();
    }
    for entity in ships.iter() {
        commands.entity(entity).despawn();
    }

    sector.nodes.clear();
    sector.routes.clear();

    let mut intel_map = HashMap::new();

    for intel in &loaded.intel {
        intel_map.insert(intel.id, intel);
    }

    for node in &loaded.nodes {
        let intel = match intel_map.get(&node.id) {
            Some(intel) => SystemIntel {
                layer: intel.layer,
                confidence: intel.confidence,
                last_seen_tick: intel.last_seen_tick,
                revealed: intel.revealed,
                revealed_tick: intel.revealed_tick,
            },
            None => SystemIntel {
                layer: KnowledgeLayer::Existence,
                confidence: 0.0,
                last_seen_tick: 0,
                revealed: false,
                revealed_tick: 0,
            },
        };

                let system_node = SystemNode {
                    id: node.id,
                    position: Vec2::new(node.x, node.y),
                    modifier: node.modifier,
                };
        sector.nodes.push(system_node.clone());
        commands.spawn((
            system_node,
            intel,
            Name::new(format!("SystemNode-{}", node.id)),
            SpatialBundle::from_transform(Transform::from_xyz(node.x, node.y, 0.0)),
        ));
    }

    sector.routes = loaded.routes.clone();

    for station in &loaded.stations {
        let mut entity_commands = commands.spawn((
            Station {
                kind: station.kind,
                state: station.state,
                fuel: station.fuel,
                fuel_capacity: station.fuel_capacity,
            },
            Name::new(format!("Station-{:?}-{:?}", station.kind, station.state)),
            SpatialBundle::from_transform(Transform::from_xyz(station.x, station.y, 0.5)),
        ));

        if station.build_remaining > 0.0 {
            entity_commands.insert(StationBuild {
                remaining_seconds: station.build_remaining,
            });
        }

        if let (Some(crisis_type), Some(crisis_stage)) = (station.crisis_type, station.crisis_stage)
        {
            entity_commands.insert(StationCrisis {
                crisis_type,
                stage: crisis_stage,
            });
        }
    }

    for ship in &loaded.ships {
        commands.spawn((
            Ship {
                kind: ship.kind,
                state: ship.state,
                fuel: ship.fuel,
                fuel_capacity: ship.fuel_capacity,
            },
            Fleet {
                role: ship.role,
            },
            ShipFuelAlert::default(),
            Name::new(format!("Ship-{:?}-{:?}", ship.kind, ship.state)),
            SpatialBundle::from_transform(Transform::from_xyz(ship.x, ship.y, 0.4)),
        ));
    }
}
