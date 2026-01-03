use bevy::asset::LoadState;
use bevy::camera::{OrthographicProjection, Projection};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use std::path::Path;

use crate::compat::{Camera2dBundle, SpriteBundle, Text2dBundle, TextBundle, TextStyle};
use crate::ore::{OreKind, OreNode};
use crate::pirates::{PirateBase, PirateShip};
use crate::plugins::core::{DebugWindow, FogConfig, GameState, InputBindings, ViewMode};
use crate::plugins::sim::{advance_intel_layer, refresh_intel};
use crate::plugins::ui::{HoveredNode, MapUi};
use crate::ships::{Ship, ShipKind};
use crate::stations::Station;
use crate::world::{KnowledgeLayer, Sector, SystemIntel, SystemNode, ZoneModifier};
use bevy::image::Image;
use bevy::window::PrimaryWindow;

pub struct Render2DPlugin;

impl Plugin for Render2DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderToggles>()
            .init_resource::<IntelRefreshCooldown>()
            .init_resource::<MapZoomOverride>()
            .init_resource::<FocusMarker>()
            .add_systems(Startup, load_player_ship_texture)
            .add_systems(Startup, setup_camera)
            .add_systems(Update, sync_camera_view.run_if(in_state(GameState::InGame)))
            .add_systems(
                Update,
                (
                    clear_focus_marker_on_map,
                    spawn_node_visuals,
                    sync_node_visuals,
                    update_node_visuals,
                    draw_intel_rings,
                    draw_routes,
                    update_route_labels,
                    update_node_labels,
                    update_hovered_node,
                    sync_view_entities,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_map),
            )
            .add_systems(
                Update,
                (
                    handle_render_toggles,
                    handle_map_zoom,
                    handle_intel_refresh,
                    handle_intel_advance,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_map)
                    .run_if(debug_window_open),
            )
            .add_systems(
                Update,
                (
                    draw_world_backdrop,
                    spawn_station_visuals,
                    sync_station_visuals,
                    update_station_labels,
                    spawn_ore_visuals,
                    sync_ore_visuals,
                    update_ore_visuals,
                    spawn_pirate_base_visuals,
                    sync_pirate_base_visuals,
                    spawn_pirate_ship_visuals,
                    sync_pirate_ship_visuals,
                    spawn_ship_visuals,
                    sync_ship_visuals,
                    update_ship_visuals,
                    update_ship_labels,
                    draw_focus_marker,
                    sync_view_entities,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_world),
            )
            .add_systems(
                Update,
                track_player_camera
                    .after(sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_world),
            )
            .add_systems(
                Update,
                (center_camera_on_revealed, debug_player_components)
                    .after(sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_world)
                    .run_if(debug_window_open),
            );
    }
}

const MAP_EXTENT_X: f32 = 600.0;
const MAP_EXTENT_Y: f32 = 360.0;

#[derive(Resource)]
pub struct MapZoomOverride {
    enabled: bool,
    index: usize,
}

impl Default for MapZoomOverride {
    fn default() -> Self {
        Self {
            enabled: false,
            index: 0,
        }
    }
}

impl MapZoomOverride {
    pub fn label(&self) -> String {
        if !self.enabled {
            return "Auto".to_string();
        }

        let presets = map_zoom_presets();
        let index = self.index.min(presets.len().saturating_sub(1));
        format!("{:.2}", presets[index])
    }
}

#[derive(Resource)]
pub struct RenderToggles {
    show_nodes: bool,
    show_routes: bool,
    show_rings: bool,
    show_grid: bool,
    show_backdrop: bool,
    show_route_labels: bool,
    show_node_labels: bool,
}

impl Default for RenderToggles {
    fn default() -> Self {
        Self {
            show_nodes: true,
            show_routes: true,
            show_rings: true,
            show_grid: true,
            show_backdrop: true,
            show_route_labels: true,
            show_node_labels: true,
        }
    }
}

impl RenderToggles {
    pub fn rings_enabled(&self) -> bool {
        self.show_rings
    }

    pub fn grid_enabled(&self) -> bool {
        self.show_grid
    }

    pub fn route_labels_enabled(&self) -> bool {
        self.show_route_labels
    }

    pub fn node_labels_enabled(&self) -> bool {
        self.show_node_labels
    }
}

#[derive(Component)]
struct NodeVisual {
    target: Entity,
}

#[derive(Component)]
struct RouteLabel;

#[derive(Component)]
struct NodeLabel;

#[derive(Component)]
struct StationVisual {
    target: Entity,
}

#[derive(Component)]
struct StationLabel;

#[derive(Component)]
struct ShipVisual {
    target: Entity,
}

#[derive(Component)]
struct ShipVisualMarker;
#[derive(Component)]
struct ShipLabel;

#[derive(Component)]
struct OreVisual {
    target: Entity,
}

#[derive(Component)]
struct PirateBaseVisual {
    target: Entity,
}

#[derive(Component)]
struct PirateShipVisual {
    target: Entity,
}

#[derive(Resource)]
pub struct IntelRefreshCooldown {
    next_allowed_tick: u64,
    cooldown_ticks: u64,
}

impl Default for IntelRefreshCooldown {
    fn default() -> Self {
        Self {
            next_allowed_tick: 0,
            cooldown_ticks: 20,
        }
    }
}

impl IntelRefreshCooldown {
    pub fn remaining_ticks(&self, current: u64) -> u64 {
        self.next_allowed_tick.saturating_sub(current)
    }
}

#[derive(Resource, Default)]
pub struct FocusMarker {
    position: Option<Vec2>,
    node_id: Option<u32>,
}

impl FocusMarker {
    pub fn position(&self) -> Option<Vec2> {
        self.position
    }

    pub fn node_id(&self) -> Option<u32> {
        self.node_id
    }
}

const PLAYER_SHIP_ASPECT: f32 = 860.0 / 1065.0;
const PLAYER_SHIP_WIDTH: f32 = 60.0;
const PLAYER_SHIP_SIZE: Vec2 = Vec2::new(PLAYER_SHIP_WIDTH, PLAYER_SHIP_WIDTH * PLAYER_SHIP_ASPECT);

#[derive(Resource)]
pub struct PlayerShipTexture(pub Handle<Image>);

fn setup_camera(mut commands: Commands) {
    info!("Setting up camera with scale 0.75 at origin");

    commands.spawn((
        Camera2dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 0.75,
                ..OrthographicProjection::default_2d()
            }),
            camera: Camera {
                order: 0,
                ..default()
            },
            ..default()
        },
        Name::new("MainCamera"),
    ));

    info!("Camera spawned with render layers");
}

fn load_player_ship_texture(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handle = asset_server.load("sprites/player_ship.png");
    commands.insert_resource(PlayerShipTexture(handle));
}

fn sync_camera_view(
    view: Res<ViewMode>,
    zoom: Res<MapZoomOverride>,
    mut projections: Query<&mut Projection, With<Camera2d>>,
    mut transforms: Query<&mut Transform, With<Camera2d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    sector: Res<Sector>,
) {
    let scale = match *view {
        ViewMode::World => 0.6,
        ViewMode::Map => map_scale_for_window(windows.single().ok(), &zoom),
    };

    for mut projection in projections.iter_mut() {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            if orthographic.scale != scale {
                info!(
                    "sync_camera_view: Setting camera scale to {:.2} for {:?}",
                    scale, *view
                );
                orthographic.scale = scale;
            }
        }
    }

    if matches!(*view, ViewMode::Map) {
        let center = map_center(&sector);
        info!(
            "sync_camera_view: In Map mode, centering camera at ({:.1}, {:.1})",
            center.x, center.y
        );
        for mut transform in transforms.iter_mut() {
            transform.translation.x = center.x;
            transform.translation.y = center.y;
        }
    }
}

fn view_is_map(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::Map)
}

fn view_is_world(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::World)
}

fn debug_window_open(debug_window: Res<DebugWindow>) -> bool {
    debug_window.open
}

fn handle_map_zoom(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut zoom: ResMut<MapZoomOverride>,
) {
    if !input.just_pressed(bindings.map_zoom) {
        return;
    }

    if !zoom.enabled {
        zoom.enabled = true;
        zoom.index = 0;
        return;
    }

    zoom.index += 1;
    if zoom.index >= map_zoom_presets().len() {
        zoom.enabled = false;
        zoom.index = 0;
    }
}

fn spawn_node_visuals(
    mut commands: Commands,
    fog: Res<FogConfig>,
    toggles: Res<RenderToggles>,
    nodes: Query<(Entity, &SystemNode, &SystemIntel), Without<NodeVisual>>,
) {
    if !toggles.show_nodes {
        return;
    }

    for (entity, node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let alpha = intel.confidence.clamp(layer_floor(intel.layer, &fog), 1.0);
        let intensity = match intel.layer {
            KnowledgeLayer::Existence => 0.45,
            KnowledgeLayer::Geography => 0.55,
            KnowledgeLayer::Resources => 0.65,
            KnowledgeLayer::Threats => 0.75,
            KnowledgeLayer::Stability => 0.85,
        };

        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(0.2, 0.75, 0.9, alpha * intensity),
                custom_size: Some(Vec2::splat(12.0)),
                ..default()
            },
            transform: Transform::from_xyz(node.position.x, node.position.y, 0.0),
            ..default()
        };

        commands.spawn((NodeVisual { target: entity }, sprite));
    }
}

fn sync_node_visuals(
    mut commands: Commands,
    toggles: Res<RenderToggles>,
    mut visuals: Query<(Entity, &NodeVisual, &mut Transform)>,
    nodes: Query<&SystemNode>,
) {
    if !toggles.show_nodes {
        return;
    }

    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        match nodes.get(visual.target) {
            Ok(node) => {
                transform.translation.x = node.position.x;
                transform.translation.y = node.position.y;
            }
            Err(_) => {
                commands.entity(visual_entity).despawn();
            }
        }
    }
}

fn spawn_station_visuals(
    mut commands: Commands,
    stations: Query<(Entity, &Transform), (With<Station>, Without<StationVisual>)>,
) {
    for (entity, transform) in stations.iter() {
        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.85, 0.8, 0.35),
                custom_size: Some(Vec2::new(10.0, 10.0)),
                ..default()
            },
            transform: *transform,
            ..default()
        };

        commands.spawn((StationVisual { target: entity }, sprite));
    }
}

fn sync_station_visuals(
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

fn spawn_ore_visuals(
    mut commands: Commands,
    ores: Query<(Entity, &Transform), (With<OreNode>, Without<OreVisual>)>,
) {
    for (entity, transform) in ores.iter() {
        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.75, 0.6, 0.35),
                custom_size: Some(Vec2::new(6.0, 6.0)),
                ..default()
            },
            transform: *transform,
            ..default()
        };

        commands.spawn((OreVisual { target: entity }, sprite));
    }
}

fn sync_ore_visuals(
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

fn update_ore_visuals(
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

fn spawn_pirate_base_visuals(
    mut commands: Commands,
    bases: Query<(Entity, &Transform), (With<PirateBase>, Without<PirateBaseVisual>)>,
) {
    for (entity, transform) in bases.iter() {
        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.85, 0.25, 0.2),
                custom_size: Some(Vec2::new(12.0, 12.0)),
                ..default()
            },
            transform: *transform,
            ..default()
        };

        commands.spawn((PirateBaseVisual { target: entity }, sprite));
    }
}

fn sync_pirate_base_visuals(
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

fn spawn_pirate_ship_visuals(
    mut commands: Commands,
    ships: Query<(Entity, &Transform), (With<PirateShip>, Without<PirateShipVisual>)>,
) {
    for (entity, transform) in ships.iter() {
        let sprite = SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.9, 0.35, 0.3),
                custom_size: Some(Vec2::new(8.0, 8.0)),
                ..default()
            },
            transform: *transform,
            ..default()
        };

        commands.spawn((PirateShipVisual { target: entity }, sprite));
    }
}

fn sync_pirate_ship_visuals(
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

fn spawn_ship_visuals(
    mut commands: Commands,
    ships: Query<
        (Entity, &Transform, &Ship),
        (
            Without<ShipVisual>,
            Without<ShipVisualMarker>,
            Without<Sprite>,
        ),
    >,
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

fn sync_ship_visuals(
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

fn update_ship_visuals(
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

fn update_ship_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ships: Query<(&Ship, &Transform)>,
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

    for (ship, transform) in ships.iter() {
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

fn debug_player_components(
    player: Query<
        (
            &Transform,
            Option<&Sprite>,
            Option<&Visibility>,
            Option<&ViewVisibility>,
            Option<&InheritedVisibility>,
        ),
        With<crate::plugins::player::PlayerControl>,
    >,
) {
    static mut LOGGED: bool = false;
    unsafe {
        if !LOGGED {
            if let Ok((transform, sprite, vis, view_vis, inherited_vis)) = player.single() {
                info!("=== PLAYER SHIP COMPONENTS ===");
                info!(
                    "  Transform: pos=({:.1}, {:.1}, {:.1})",
                    transform.translation.x, transform.translation.y, transform.translation.z
                );
                info!("  Sprite: {}", if sprite.is_some() { "YES" } else { "NO" });
                info!("  Visibility: {:?}", vis);
                info!("  ViewVisibility: {:?}", view_vis);
                info!("  InheritedVisibility: {:?}", inherited_vis);
                LOGGED = true;
            }
        }
    }
}

fn track_player_camera(
    player: Query<
        &Transform,
        (
            With<crate::plugins::player::PlayerControl>,
            Without<Camera2d>,
        ),
    >,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    if let Ok(player_transform) = player.single() {
        for mut camera_transform in cameras.iter_mut() {
            let old_x = camera_transform.translation.x;
            let old_y = camera_transform.translation.y;
            camera_transform.translation.x = player_transform.translation.x;
            camera_transform.translation.y = player_transform.translation.y;

            if old_x != camera_transform.translation.x || old_y != camera_transform.translation.y {
                info!(
                    "Camera tracking player: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                    old_x, old_y, camera_transform.translation.x, camera_transform.translation.y
                );
            }
        }
    } else {
        info!("track_player_camera: No player found!");
    }
}

fn center_camera_on_revealed(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
    mut marker: ResMut<FocusMarker>,
    mut log: ResMut<crate::plugins::core::EventLog>,
) {
    if !input.just_pressed(bindings.center_camera) {
        return;
    }

    let mut target = None;
    for (node, intel) in nodes.iter() {
        if intel.revealed {
            target = Some((node.position, node.id));
            break;
        }
    }

    if let Some((target, node_id)) = target {
        for mut transform in cameras.iter_mut() {
            transform.translation.x = target.x;
            transform.translation.y = target.y;
        }
        marker.position = Some(target);
        marker.node_id = Some(node_id);
        log.push(format!("World camera centered on node {}", node_id));
    } else {
        marker.position = None;
        marker.node_id = None;
        log.push("World camera center failed: no revealed nodes".to_string());
    }
}

fn draw_focus_marker(mut gizmos: Gizmos, marker: Res<FocusMarker>) {
    let position = match marker.position() {
        Some(position) => position,
        None => {
            return;
        }
    };

    let color = Color::srgba(0.85, 0.9, 1.0, 0.6);
    let size = 10.0;

    gizmos.line_2d(
        position + Vec2::new(-size, 0.0),
        position + Vec2::new(size, 0.0),
        color,
    );
    gizmos.line_2d(
        position + Vec2::new(0.0, -size),
        position + Vec2::new(0.0, size),
        color,
    );
    gizmos.circle_2d(position, size * 0.6, Color::srgba(0.7, 0.85, 0.95, 0.35));
}

fn clear_focus_marker_on_map(mut marker: ResMut<FocusMarker>) {
    marker.position = None;
    marker.node_id = None;
}

fn update_station_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    stations: Query<(
        &Station,
        Option<&crate::stations::StationCrisis>,
        &Transform,
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

    for (station, crisis, transform) in stations.iter() {
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

#[allow(clippy::too_many_arguments)]
fn sync_view_entities(
    view: Res<ViewMode>,
    mut commands: Commands,
    node_visuals: Query<Entity, With<NodeVisual>>,
    node_labels: Query<Entity, With<NodeLabel>>,
    route_labels: Query<Entity, With<RouteLabel>>,
    station_visuals: Query<Entity, With<StationVisual>>,
    station_labels: Query<Entity, With<StationLabel>>,
    ore_visuals: Query<Entity, With<OreVisual>>,
    pirate_base_visuals: Query<Entity, With<PirateBaseVisual>>,
    pirate_ship_visuals: Query<Entity, With<PirateShipVisual>>,
    ship_visuals: Query<(Entity, &ShipVisual)>,
    ship_labels: Query<Entity, With<ShipLabel>>,
) {
    match *view {
        ViewMode::World => {
            for entity in node_visuals.iter() {
                commands.entity(entity).despawn();
            }
            for entity in node_labels.iter() {
                commands.entity(entity).despawn();
            }
            for entity in route_labels.iter() {
                commands.entity(entity).despawn();
            }
        }
        ViewMode::Map => {
            for entity in station_visuals.iter() {
                commands.entity(entity).despawn();
            }
            for entity in station_labels.iter() {
                commands.entity(entity).despawn();
            }
            for entity in ore_visuals.iter() {
                commands.entity(entity).despawn();
            }
            for entity in pirate_base_visuals.iter() {
                commands.entity(entity).despawn();
            }
            for entity in pirate_ship_visuals.iter() {
                commands.entity(entity).despawn();
            }
            for (entity, visual) in ship_visuals.iter() {
                commands.entity(visual.target).remove::<ShipVisualMarker>();
                commands.entity(entity).despawn();
            }
            for entity in ship_labels.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn draw_world_backdrop(mut gizmos: Gizmos, toggles: Res<RenderToggles>) {
    if !toggles.show_backdrop {
        return;
    }

    let edge = 800.0;
    let color = Color::srgba(0.1, 0.15, 0.2, 0.25);

    gizmos.line_2d(Vec2::new(-edge, 0.0), Vec2::new(edge, 0.0), color);
    gizmos.line_2d(Vec2::new(0.0, -edge), Vec2::new(0.0, edge), color);

    gizmos.line_2d(Vec2::new(-edge, -edge), Vec2::new(edge, -edge), color);
    gizmos.line_2d(Vec2::new(edge, -edge), Vec2::new(edge, edge), color);
    gizmos.line_2d(Vec2::new(edge, edge), Vec2::new(-edge, edge), color);
    gizmos.line_2d(Vec2::new(-edge, edge), Vec2::new(-edge, -edge), color);

    let stars = [
        Vec2::new(-320.0, 140.0),
        Vec2::new(-210.0, -220.0),
        Vec2::new(180.0, 160.0),
        Vec2::new(260.0, -140.0),
        Vec2::new(60.0, -40.0),
        Vec2::new(-60.0, 220.0),
    ];

    for star in stars {
        gizmos.circle_2d(star, 2.0, Color::srgba(0.8, 0.85, 0.95, 0.4));
    }
}

fn map_scale_for_window(window: Option<&Window>, zoom: &MapZoomOverride) -> f32 {
    if zoom.enabled {
        let presets = map_zoom_presets();
        let index = zoom.index.min(presets.len().saturating_sub(1));
        return presets[index];
    }

    let (width, height) = match window {
        Some(window) => (window.width(), window.height()),
        None => (1280.0, 720.0),
    };

    let scale_x = (MAP_EXTENT_X * 2.0) / width;
    let scale_y = (MAP_EXTENT_Y * 2.0) / height;
    let scale = scale_x.max(scale_y) * 1.05;

    scale.clamp(0.6, 2.0)
}

fn map_center(sector: &Sector) -> Vec2 {
    if sector.nodes.is_empty() {
        return Vec2::ZERO;
    }

    let mut sum = Vec2::ZERO;
    let mut count = 0.0;

    for node in &sector.nodes {
        sum += node.position;
        count += 1.0;
    }

    if count > 0.0 {
        sum / count
    } else {
        Vec2::ZERO
    }
}

fn map_zoom_presets() -> [f32; 3] {
    [0.6, 0.8, 1.0]
}

fn update_node_visuals(
    toggles: Res<RenderToggles>,
    fog: Res<FogConfig>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut visuals: Query<(&NodeVisual, &mut Sprite)>,
) {
    if !toggles.show_nodes {
        return;
    }

    for (visual, mut sprite) in visuals.iter_mut() {
        if let Ok((_node, intel)) = nodes.get(visual.target) {
            if !intel.revealed {
                continue;
            }
            let alpha = intel.confidence.clamp(layer_floor(intel.layer, &fog), 1.0);
            let intensity = match intel.layer {
                KnowledgeLayer::Existence => 0.45,
                KnowledgeLayer::Geography => 0.55,
                KnowledgeLayer::Resources => 0.65,
                KnowledgeLayer::Threats => 0.75,
                KnowledgeLayer::Stability => 0.85,
            };
            sprite.color = Color::srgba(0.2, 0.75, 0.9, alpha * intensity);
        }
    }
}

fn draw_intel_rings(
    mut gizmos: Gizmos,
    toggles: Res<RenderToggles>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    if !toggles.show_nodes || !toggles.show_rings {
        return;
    }

    for (node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let t = intel.confidence.clamp(0.0, 1.0);
        let color = Color::srgba(0.9 * (1.0 - t), 0.8 * t, 0.3, 0.6);
        let radius = 10.0 + (1.0 - t) * 6.0;
        gizmos.circle_2d(node.position, radius, color);
    }
}

fn draw_routes(
    mut gizmos: Gizmos,
    sector: Res<Sector>,
    toggles: Res<RenderToggles>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    if !toggles.show_routes {
        return;
    }

    let mut revealed = std::collections::HashMap::new();
    for (node, intel) in nodes.iter() {
        revealed.insert(node.id, intel.revealed);
    }

    for route in &sector.routes {
        let start_known = revealed.get(&route.from).copied().unwrap_or(false);
        let end_known = revealed.get(&route.to).copied().unwrap_or(false);
        if !start_known || !end_known {
            continue;
        }

        let start = find_node_position(&sector.nodes, route.from);
        let end = find_node_position(&sector.nodes, route.to);

        if let (Some(start), Some(end)) = (start, end) {
            let color = risk_color(route.risk);
            gizmos.line_2d(start, end, color);
        }
    }
}

fn update_route_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    toggles: Res<RenderToggles>,
    sector: Res<Sector>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    labels: Query<Entity, With<RouteLabel>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    for entity in labels.iter() {
        commands.entity(entity).despawn();
    }

    if !toggles.show_routes || !toggles.show_route_labels {
        return;
    }

    let (camera, camera_transform) = match cameras.single() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let mut revealed = std::collections::HashMap::new();
    for (node, intel) in nodes.iter() {
        revealed.insert(node.id, intel.revealed);
    }

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    for route in &sector.routes {
        let start_known = revealed.get(&route.from).copied().unwrap_or(false);
        let end_known = revealed.get(&route.to).copied().unwrap_or(false);
        if !start_known || !end_known {
            continue;
        }

        let start = find_node_position(&sector.nodes, route.from);
        let end = find_node_position(&sector.nodes, route.to);

        if let (Some(start), Some(end)) = (start, end) {
            let mid = (start + end) * 0.5;
            let label = format!("{:.0} r{:.2}", route.distance, route.risk);
            if let Ok(screen) = camera.world_to_viewport(camera_transform, mid.extend(0.0)) {
                let position = Vec2::new(screen.x + 6.0, screen.y - 10.0);
                let label_color = risk_color(route.risk);
                commands.spawn((
                    RouteLabel,
                    MapUi,
                    TextBundle::from_section(
                        label,
                        TextStyle {
                            font: font.clone(),
                            font_size: 18.0,
                            color: label_color,
                        },
                    )
                    .with_node(UiNode {
                        position_type: PositionType::Absolute,
                        left: Val::Px(position.x),
                        top: Val::Px(position.y),
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    })
                    .with_background_color(Color::srgba(0.05, 0.08, 0.12, 0.6)),
                ));
            }
        }
    }
}

fn update_node_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    toggles: Res<RenderToggles>,
    ticks: Res<crate::plugins::sim::SimTickCount>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    labels: Query<Entity, With<NodeLabel>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    for entity in labels.iter() {
        commands.entity(entity).despawn();
    }

    if !toggles.show_nodes || !toggles.show_node_labels {
        return;
    }

    let (camera, camera_transform) = match cameras.single() {
        Ok(value) => value,
        Err(_) => {
            return;
        }
    };

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    for (node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let age = ticks.tick.saturating_sub(intel.revealed_tick);
        let label = format!(
            "L{} {:.0}% {} t{}",
            layer_short(intel.layer),
            intel.confidence * 100.0,
            modifier_icon(node.modifier),
            age
        );

        let position = node.position + Vec2::new(0.0, 14.0);
        if let Ok(screen) = camera.world_to_viewport(camera_transform, position.extend(0.0)) {
            let label_pos = Vec2::new(screen.x + 6.0, screen.y - 12.0);
            let alpha = 0.4 + 0.6 * intel.confidence.clamp(0.0, 1.0);
            commands.spawn((
                NodeLabel,
                MapUi,
                TextBundle::from_section(
                    label,
                    TextStyle {
                        font: font.clone(),
                        font_size: 14.0,
                        color: Color::srgba(0.82, 0.9, 0.96, alpha),
                    },
                )
                .with_node(UiNode {
                    position_type: PositionType::Absolute,
                    left: Val::Px(label_pos.x),
                    top: Val::Px(label_pos.y),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                })
                .with_background_color(Color::srgba(0.05, 0.08, 0.12, 0.6)),
            ));
        }
    }
}

fn handle_render_toggles(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut toggles: ResMut<RenderToggles>,
    mut commands: Commands,
    visuals: Query<Entity, With<NodeVisual>>,
) {
    let mut updated = false;

    if input.just_pressed(bindings.toggle_nodes) {
        toggles.show_nodes = !toggles.show_nodes;
        updated = true;

        if !toggles.show_nodes {
            for entity in visuals.iter() {
                commands.entity(entity).despawn();
            }
        }

        info!("Render nodes: {}", toggles.show_nodes);
    }

    if input.just_pressed(bindings.toggle_routes) {
        toggles.show_routes = !toggles.show_routes;
        updated = true;
        info!("Render routes: {}", toggles.show_routes);
    }

    if input.just_pressed(bindings.toggle_rings) {
        toggles.show_rings = !toggles.show_rings;
        updated = true;
        info!("Render rings: {}", toggles.show_rings);
    }

    if input.just_pressed(bindings.toggle_grid) {
        toggles.show_grid = !toggles.show_grid;
        updated = true;
        info!("Render grid: {}", toggles.show_grid);
    }

    if input.just_pressed(bindings.toggle_backdrop) {
        toggles.show_backdrop = !toggles.show_backdrop;
        updated = true;
        info!("Render backdrop: {}", toggles.show_backdrop);
    }

    if input.just_pressed(bindings.toggle_route_labels) {
        toggles.show_route_labels = !toggles.show_route_labels;
        updated = true;
        info!("Render route labels: {}", toggles.show_route_labels);
    }

    if input.just_pressed(bindings.toggle_node_labels) {
        toggles.show_node_labels = !toggles.show_node_labels;
        updated = true;
        info!("Render node labels: {}", toggles.show_node_labels);
    }

    if updated {
        // Leave toggles updated; drawing uses current flags.
    }
}

fn handle_intel_refresh(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    ticks: Res<crate::plugins::sim::SimTickCount>,
    mut cooldown: ResMut<IntelRefreshCooldown>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    if input.just_pressed(bindings.refresh_intel) {
        if ticks.tick < cooldown.next_allowed_tick {
            return;
        }

        for mut intel in intel_query.iter_mut() {
            refresh_intel(&mut intel, ticks.tick);
        }
        cooldown.next_allowed_tick = ticks.tick.saturating_add(cooldown.cooldown_ticks);
        info!("Intel refreshed");
    }
}

fn handle_intel_advance(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    if input.just_pressed(bindings.advance_intel) {
        for mut intel in intel_query.iter_mut() {
            advance_intel_layer(&mut intel);
        }
        info!("Intel layer advanced");
    }
}

fn layer_short(layer: KnowledgeLayer) -> &'static str {
    match layer {
        KnowledgeLayer::Existence => "0",
        KnowledgeLayer::Geography => "1",
        KnowledgeLayer::Resources => "2",
        KnowledgeLayer::Threats => "3",
        KnowledgeLayer::Stability => "4",
    }
}

fn modifier_icon(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "R",
        Some(ZoneModifier::NebulaInterference) => "N",
        Some(ZoneModifier::RichOreVeins) => "O",
        Some(ZoneModifier::ClearSignals) => "C",
        None => ".",
    }
}

fn station_kind_short(kind: crate::stations::StationKind) -> &'static str {
    match kind {
        crate::stations::StationKind::MiningOutpost => "M",
        crate::stations::StationKind::FuelDepot => "F",
        crate::stations::StationKind::SensorStation => "S",
    }
}

fn ship_kind_short(kind: ShipKind) -> &'static str {
    match kind {
        ShipKind::PlayerShip => "P",
        ShipKind::Scout => "S",
        ShipKind::Miner => "M",
        ShipKind::Security => "G",
    }
}

fn ship_state_short(state: crate::ships::ShipState) -> &'static str {
    match state {
        crate::ships::ShipState::Idle => "I",
        crate::ships::ShipState::InTransit => "T",
        crate::ships::ShipState::Executing => "E",
        crate::ships::ShipState::Returning => "R",
        crate::ships::ShipState::Refueling => "F",
        crate::ships::ShipState::Damaged => "D",
        crate::ships::ShipState::Disabled => "X",
    }
}

fn update_hovered_node(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut hovered: ResMut<HoveredNode>,
) {
    let window = match windows.single() {
        Ok(window) => window,
        Err(_) => {
            hovered.id = None;
            hovered.modifier = None;
            hovered.screen_pos = None;
            return;
        }
    };

    let cursor = match window.cursor_position() {
        Some(cursor) => cursor,
        None => {
            hovered.id = None;
            hovered.modifier = None;
            hovered.screen_pos = None;
            return;
        }
    };

    let (camera, camera_transform) = match cameras.single() {
        Ok(camera_pair) => camera_pair,
        Err(_) => {
            hovered.id = None;
            hovered.modifier = None;
            hovered.screen_pos = None;
            return;
        }
    };

    let world_pos = match camera.viewport_to_world_2d(camera_transform, cursor) {
        Ok(world_pos) => world_pos,
        Err(_) => {
            hovered.id = None;
            hovered.modifier = None;
            hovered.screen_pos = None;
            return;
        }
    };

    let mut closest_id = None;
    let mut closest_layer = None;
    let mut closest_confidence = 0.0;
    let mut closest_modifier = None;
    let mut closest_dist = 9999.0;
    let radius = 14.0;

    for (node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let dist = node.position.distance(world_pos);
        if dist <= radius && dist < closest_dist {
            closest_dist = dist;
            closest_id = Some(node.id);
            closest_layer = Some(intel.layer);
            closest_confidence = intel.confidence;
            closest_modifier = node.modifier;
        }
    }

    hovered.id = closest_id;
    hovered.layer = closest_layer;
    hovered.confidence = closest_confidence;
    hovered.modifier = closest_modifier;
    hovered.screen_pos = Some(cursor);
    hovered.screen_size = Vec2::new(window.width(), window.height());
}

fn layer_floor(layer: KnowledgeLayer, fog: &FogConfig) -> f32 {
    match layer {
        KnowledgeLayer::Existence => fog.floor_existence,
        KnowledgeLayer::Geography => fog.floor_geography,
        KnowledgeLayer::Resources => fog.floor_resources,
        KnowledgeLayer::Threats => fog.floor_threats,
        KnowledgeLayer::Stability => fog.floor_stability,
    }
}

fn find_node_position(nodes: &[SystemNode], id: u32) -> Option<Vec2> {
    for node in nodes {
        if node.id == id {
            return Some(node.position);
        }
    }
    None
}

fn risk_color(risk: f32) -> Color {
    let t = risk.clamp(0.0, 1.0);
    let low = LinearRgba::new(0.2, 0.7, 0.4, 1.0);
    let high = LinearRgba::new(0.9, 0.25, 0.2, 1.0);
    Color::linear_rgba(
        low.red + (high.red - low.red) * t,
        low.green + (high.green - low.green) * t,
        low.blue + (high.blue - low.blue) * t,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::{map_center, map_zoom_presets, risk_color};
    use crate::world::{Sector, SystemNode};
    use bevy::prelude::{Color, LinearRgba, Vec2};

    fn assert_close(a: f32, b: f32) {
        let diff = (a - b).abs();
        assert!(diff < 1e-6, "expected {} close to {}", a, b);
    }

    #[allow(dead_code)]
    fn linear_rgb(color: Color) -> (f32, f32, f32) {
        let linear = color.to_linear();
        (linear.red, linear.green, linear.blue)
    }

    trait LinearColorExt {
        fn linear_r(self) -> f32;
        fn linear_g(self) -> f32;
        fn linear_b(self) -> f32;
    }

    impl LinearColorExt for Color {
        fn linear_r(self) -> f32 {
            self.to_linear().red
        }

        fn linear_g(self) -> f32 {
            self.to_linear().green
        }

        fn linear_b(self) -> f32 {
            self.to_linear().blue
        }
    }

    #[test]
    fn map_center_empty_is_zero() {
        let sector = Sector::default();
        let center = map_center(&sector);
        assert_eq!(center, Vec2::ZERO);
    }

    #[test]
    fn map_center_averages_nodes() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(10.0, 20.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(30.0, 40.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 20.0);
        assert_close(center.y, 30.0);
    }

    #[test]
    fn map_zoom_presets_values() {
        let presets = map_zoom_presets();
        assert_eq!(presets, [0.6, 0.8, 1.0]);
    }

    #[test]
    fn risk_color_low_is_greenish() {
        let color = risk_color(0.0);
        let linear = LinearRgba::from(color);
        assert_close(linear.red, 0.2);
        assert_close(linear.green, 0.7);
        assert_close(linear.blue, 0.4);
    }

    #[test]
    fn risk_color_high_is_reddish() {
        let color = risk_color(1.0);
        let linear = LinearRgba::from(color);
        assert_close(linear.red, 0.9);
        assert_close(linear.green, 0.25);
        assert_close(linear.blue, 0.2);
    }

    #[test]
    fn map_zoom_presets_sorted_low_to_high() {
        let presets = map_zoom_presets();
        assert!(presets[0] < presets[1]);
        assert!(presets[1] < presets[2]);
    }

    #[test]
    fn map_zoom_presets_monotonic_spacing() {
        let presets = map_zoom_presets();
        let step_low = presets[1] - presets[0];
        let step_high = presets[2] - presets[1];

        assert_close(step_low, step_high);
    }

    #[test]
    fn map_zoom_presets_mid_is_avg_of_extremes() {
        let presets = map_zoom_presets();
        let expected_mid = (presets[0] + presets[2]) * 0.5;

        assert_close(presets[1], expected_mid);
    }

    #[test]
    fn risk_color_midpoint_is_between_low_high() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);
        let high = risk_color(1.0);
        let mid_linear = LinearRgba::from(mid);
        let low_linear = LinearRgba::from(low);
        let high_linear = LinearRgba::from(high);
        assert!(
            mid_linear.red > low_linear.red && mid_linear.red < high_linear.red,
            "red component not between"
        );
        assert!(
            mid_linear.green < low_linear.green && mid_linear.green > high_linear.green,
            "green component not between"
        );
        assert!(
            mid_linear.blue < low_linear.blue && mid_linear.blue > high_linear.blue,
            "blue component not between"
        );
    }

    #[test]
    fn risk_color_midpoint_green_between_low_high() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);
        let high = risk_color(1.0);

        let mid_linear = LinearRgba::from(mid);
        let low_linear = LinearRgba::from(low);
        let high_linear = LinearRgba::from(high);

        assert!(mid_linear.green < low_linear.green);
        assert!(mid_linear.green > high_linear.green);
    }

    #[test]
    fn map_center_single_node_equals_position() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(-12.0, 48.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, -12.0);
        assert_close(center.y, 48.0);
    }

    #[test]
    fn risk_color_clamps_below_zero() {
        let below = risk_color(-0.5);
        let low = risk_color(0.0);
        assert_close(below.linear_r(), low.linear_r());
        assert_close(below.linear_g(), low.linear_g());
        assert_close(below.linear_b(), low.linear_b());
    }

    #[test]
    fn risk_color_clamps_above_one() {
        let above = risk_color(1.5);
        let high = risk_color(1.0);
        assert_close(above.linear_r(), high.linear_r());
        assert_close(above.linear_g(), high.linear_g());
        assert_close(above.linear_b(), high.linear_b());
    }

    #[test]
    fn map_center_averages_three_nodes() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(0.0, 0.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(6.0, 3.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 3,
            position: Vec2::new(3.0, 9.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 3.0);
        assert_close(center.y, 4.0);
    }

    #[test]
    fn risk_color_midpoint_components_between_extremes() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);
        let high = risk_color(1.0);

        assert!(mid.linear_r() >= low.linear_r() && mid.linear_r() <= high.linear_r());
        assert!(mid.linear_g() <= low.linear_g() && mid.linear_g() >= high.linear_g());
        assert!(mid.linear_b() <= low.linear_b() && mid.linear_b() >= high.linear_b());
    }

    #[test]
    fn map_center_handles_negative_positions() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(-10.0, -20.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(-30.0, -40.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, -20.0);
        assert_close(center.y, -30.0);
    }

    #[test]
    fn map_zoom_presets_len_is_three() {
        let presets = map_zoom_presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn map_center_is_zero_for_two_opposite_nodes() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(5.0, 5.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(-5.0, -5.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 0.0);
        assert_close(center.y, 0.0);
    }

    #[test]
    fn risk_color_midpoint_matches_linear_mix() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);
        let high = risk_color(1.0);

        let expected_r = (low.linear_r() + high.linear_r()) * 0.5;
        let expected_g = (low.linear_g() + high.linear_g()) * 0.5;
        let expected_b = (low.linear_b() + high.linear_b()) * 0.5;

        assert_close(mid.linear_r(), expected_r);
        assert_close(mid.linear_g(), expected_g);
        assert_close(mid.linear_b(), expected_b);
    }

    #[test]
    fn risk_color_midpoint_red_gt_low() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);

        assert!(mid.linear_r() > low.linear_r());
    }

    #[test]
    fn risk_color_midpoint_green_lt_low() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);

        assert!(mid.linear_g() < low.linear_g());
    }

    #[test]
    fn risk_color_midpoint_blue_lt_low() {
        let mid = risk_color(0.5);
        let low = risk_color(0.0);

        assert!(mid.linear_b() < low.linear_b());
    }

    #[test]
    fn risk_color_midpoint_red_lt_high() {
        let mid = risk_color(0.5);
        let high = risk_color(1.0);

        assert!(mid.linear_r() < high.linear_r());
    }

    #[test]
    fn risk_color_midpoint_green_gt_high() {
        let mid = risk_color(0.5);
        let high = risk_color(1.0);

        assert!(mid.linear_g() > high.linear_g());
    }

    #[test]
    fn risk_color_midpoint_blue_gt_high() {
        let mid = risk_color(0.5);
        let high = risk_color(1.0);

        assert!(mid.linear_b() > high.linear_b());
    }

    #[test]
    fn map_center_two_nodes_midpoint() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(2.0, 6.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(10.0, 14.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 6.0);
        assert_close(center.y, 10.0);
    }

    #[test]
    fn map_center_matches_average_of_all_nodes_again() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(4.0, 8.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(10.0, -4.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 3,
            position: Vec2::new(-2.0, 6.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 4.0);
        assert_close(center.y, 10.0 / 3.0);
    }

    #[test]
    fn map_center_all_nodes_same_position_is_that_position() {
        let mut sector = Sector::default();
        let position = Vec2::new(7.5, -3.25);
        sector.nodes.push(SystemNode {
            id: 1,
            position,
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position,
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 3,
            position,
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, position.x);
        assert_close(center.y, position.y);
    }

    #[test]
    fn map_center_four_nodes_quadrant_average() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(4.0, 4.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(-4.0, 4.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 3,
            position: Vec2::new(-4.0, -4.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 4,
            position: Vec2::new(4.0, -4.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 0.0);
        assert_close(center.y, 0.0);
    }

    #[test]
    fn map_zoom_presets_values_are_unique() {
        let presets = map_zoom_presets();
        assert!(presets[0] != presets[1]);
        assert!(presets[1] != presets[2]);
        assert!(presets[0] != presets[2]);
    }

    #[test]
    fn risk_color_midpoint_is_avg_of_endpoints() {
        let low = risk_color(0.0);
        let high = risk_color(1.0);
        let mid = risk_color(0.5);

        assert_close(mid.linear_r(), (low.linear_r() + high.linear_r()) * 0.5);
        assert_close(mid.linear_g(), (low.linear_g() + high.linear_g()) * 0.5);
        assert_close(mid.linear_b(), (low.linear_b() + high.linear_b()) * 0.5);
    }

    #[test]
    fn map_center_three_nodes_midpoint_check() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(3.0, 6.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 2,
            position: Vec2::new(9.0, 0.0),
            modifier: None,
        });
        sector.nodes.push(SystemNode {
            id: 3,
            position: Vec2::new(0.0, 12.0),
            modifier: None,
        });

        let center = map_center(&sector);
        assert_close(center.x, 4.0);
        assert_close(center.y, 6.0);
    }

    #[test]
    fn map_zoom_presets_sorted_unique() {
        let presets = map_zoom_presets();
        assert!(presets[0] < presets[1]);
        assert!(presets[1] < presets[2]);
    }

    #[test]
    fn risk_color_low_matches_constants() {
        let low = risk_color(0.0);
        assert_close(low.linear_r(), 0.2);
        assert_close(low.linear_g(), 0.7);
        assert_close(low.linear_b(), 0.4);
    }

    #[test]
    fn risk_color_high_matches_constants() {
        let high = risk_color(1.0);
        assert_close(high.linear_r(), 0.9);
        assert_close(high.linear_g(), 0.25);
        assert_close(high.linear_b(), 0.2);
    }

    #[test]
    fn map_zoom_presets_equal_constants() {
        let presets = map_zoom_presets();
        assert_eq!(presets, [0.6, 0.8, 1.0]);
    }

    #[test]
    fn risk_color_low_green_component_max() {
        let low = risk_color(0.0);
        assert!(low.linear_g() > low.linear_r());
        assert!(low.linear_g() > low.linear_b());
    }

    #[test]
    fn risk_color_high_red_component_max() {
        let high = risk_color(1.0);
        assert!(high.linear_r() > high.linear_g());
        assert!(high.linear_r() > high.linear_b());
    }

    #[test]
    fn map_zoom_presets_first_is_point_six() {
        let presets = map_zoom_presets();
        assert_close(presets[0], 0.6);
    }

    #[test]
    fn risk_color_low_blue_component_gt_red() {
        let low = risk_color(0.0);
        assert!(low.linear_b() > low.linear_r());
    }

    #[test]
    fn risk_color_high_green_component_lt_blue() {
        let high = risk_color(1.0);
        assert!(high.linear_g() > high.linear_b());
    }

    #[test]
    fn map_zoom_presets_second_is_point_eight() {
        let presets = map_zoom_presets();
        assert_close(presets[1], 0.8);
    }
}
