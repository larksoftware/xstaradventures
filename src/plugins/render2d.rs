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
use crate::plugins::player::{NearbyTargets, PlayerControl};
use crate::plugins::sim::{advance_intel_layer, refresh_intel};
use crate::plugins::ui::{HoveredNode, MapUi};
use crate::ships::{Ship, ShipKind};
use crate::stations::Station;
use crate::world::{KnowledgeLayer, Sector, SystemIntel, SystemNode, ZoneId, ZoneModifier};
use bevy::image::Image;
use bevy::window::PrimaryWindow;

// Type aliases for complex query filter combinations (filters only, not full queries)
type StationSpawnFilter = (With<Station>, Without<StationVisualMarker>);
type OreSpawnFilter = (With<OreNode>, Without<OreVisualMarker>);
type PirateBaseSpawnFilter = (With<PirateBase>, Without<PirateBaseVisualMarker>);
type PirateShipSpawnFilter = (With<PirateShip>, Without<PirateShipVisualMarker>);
type ShipSpawnFilter = (
    Without<ShipVisual>,
    Without<ShipVisualMarker>,
    Without<Sprite>,
);

/// Check if either Shift key is pressed (for debug key modifiers)
fn shift_pressed(input: &ButtonInput<KeyCode>) -> bool {
    input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
}

#[derive(Component)]
struct Starfield {
    #[allow(dead_code)]
    layer: u8,
}

pub struct Render2DPlugin;

impl Plugin for Render2DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderToggles>()
            .init_resource::<IntelRefreshCooldown>()
            .init_resource::<MapZoomOverride>()
            .init_resource::<MapPanOffset>()
            .init_resource::<FocusMarker>()
            .init_resource::<HomeBeaconEnabled>()
            .add_systems(Startup, load_player_ship_texture)
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, spawn_starfield)
            .add_systems(
                Update,
                wrap_starfield
                    .after(track_player_camera)
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_world),
            )
            .add_systems(
                Update,
                toggle_starfield_visibility.run_if(in_state(GameState::InGame)),
            )
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
                    handle_intel_refresh,
                    handle_intel_advance,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_map)
                    .run_if(debug_window_open),
            )
            .add_systems(
                Update,
                (handle_map_zoom_wheel, handle_map_pan)
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_map),
            )
            .add_systems(
                Update,
                (
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
                    sync_zone_visibility,
                    draw_focus_marker,
                    draw_tactical_navigation,
                    draw_home_beacon,
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
                center_camera_on_revealed
                    .after(sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_world),
            )
            .add_systems(
                Update,
                debug_player_components
                    .after(sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(view_is_world)
                    .run_if(debug_window_open),
            );
    }
}

#[allow(dead_code)]
const MAP_EXTENT_X: f32 = 600.0;
#[allow(dead_code)]
const MAP_EXTENT_Y: f32 = 360.0;

/// Zoom configuration for map view
pub const MAP_ZOOM_MIN: f32 = 0.3;
pub const MAP_ZOOM_MAX: f32 = 2.0;
pub const MAP_ZOOM_DEFAULT: f32 = 0.8;
pub const MAP_ZOOM_STEP: f32 = 0.1;

#[derive(Resource)]
pub struct MapZoomOverride {
    /// Current zoom scale (smaller = zoomed in, larger = zoomed out)
    pub scale: f32,
}

impl Default for MapZoomOverride {
    fn default() -> Self {
        Self {
            scale: MAP_ZOOM_DEFAULT,
        }
    }
}

impl MapZoomOverride {
    pub fn label(&self) -> String {
        format!("{:.2}", self.scale)
    }

    /// Zoom in (decrease scale)
    pub fn zoom_in(&mut self, amount: f32) {
        self.scale = (self.scale - amount).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }

    /// Zoom out (increase scale)
    pub fn zoom_out(&mut self, amount: f32) {
        self.scale = (self.scale + amount).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }
}

/// Pan offset for map view (applied as camera translation offset)
#[derive(Resource, Default)]
pub struct MapPanOffset {
    /// Current pan offset from map center
    pub offset: Vec2,
    /// Whether a drag is currently in progress
    dragging: bool,
    /// Last mouse position during drag (in screen coordinates)
    last_drag_pos: Option<Vec2>,
}

impl MapPanOffset {
    /// Start a new drag operation
    pub fn start_drag(&mut self, pos: Vec2) {
        self.dragging = true;
        self.last_drag_pos = Some(pos);
    }

    /// Update drag with current mouse position, returns the delta to apply
    pub fn update_drag(&mut self, pos: Vec2, camera_scale: f32) -> Vec2 {
        if !self.dragging {
            return Vec2::ZERO;
        }

        match self.last_drag_pos {
            Some(last) => {
                let delta = pos - last;
                self.last_drag_pos = Some(pos);
                // Invert and scale by camera scale for 1:1 feel
                Vec2::new(-delta.x, delta.y) * camera_scale
            }
            None => {
                self.last_drag_pos = Some(pos);
                Vec2::ZERO
            }
        }
    }

    /// End the current drag operation
    pub fn end_drag(&mut self) {
        self.dragging = false;
        self.last_drag_pos = None;
    }

    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Reset pan offset to zero
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.offset = Vec2::ZERO;
    }
}

#[derive(Resource)]
pub struct RenderToggles {
    show_nodes: bool,
    show_routes: bool,
    show_rings: bool,
    show_grid: bool,
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
struct NodeVisualMarker;

#[derive(Component)]
struct StationVisual {
    target: Entity,
}

#[derive(Component)]
struct StationVisualMarker;

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
struct OreVisualMarker;

#[derive(Component)]
struct PirateBaseVisual {
    target: Entity,
}

#[derive(Component)]
struct PirateBaseVisualMarker;

#[derive(Component)]
struct PirateShipVisual {
    target: Entity,
}

#[derive(Component)]
struct PirateShipVisualMarker;

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

#[derive(Resource, Default)]
struct HomeBeaconEnabled {
    enabled: bool,
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

fn spawn_starfield(mut commands: Commands) {
    let mut rng_state = 42u64; // Simple LCG seed

    // Helper function to generate pseudo-random values
    let mut next_random = || -> f32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = (rng_state >> 33) as u32;
        (value as f32) / (u32::MAX as f32)
    };

    // Spawn stars across entire game world area
    // Game world is -600 to 600, -360 to 360
    // Spawn in much wider area to ensure coverage everywhere
    let min_x = -1200.0;
    let max_x = 1200.0;
    let min_y = -700.0;
    let max_y = 700.0;

    // Layer 1: Distant stars (smallest, dimmest, slowest parallax)
    for _ in 0..200 {
        let x = min_x + next_random() * (max_x - min_x);
        let y = min_y + next_random() * (max_y - min_y);
        let brightness = 0.3 + next_random() * 0.3; // 0.3-0.6
        let size = 1.0 + next_random() * 1.0; // 1.0-2.0 pixels

        commands.spawn((
            Starfield { layer: 1 },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(brightness, brightness, brightness * 1.1, 1.0),
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, -10.0),
                ..default()
            },
            Name::new("Star-Distant"),
        ));
    }

    // Layer 2: Mid-distance stars (medium size, medium brightness)
    for _ in 0..150 {
        let x = min_x + next_random() * (max_x - min_x);
        let y = min_y + next_random() * (max_y - min_y);
        let brightness = 0.5 + next_random() * 0.4; // 0.5-0.9
        let size = 1.5 + next_random() * 1.5; // 1.5-3.0 pixels

        // Vary color slightly (white to light blue)
        let blue_tint = next_random() * 0.2;
        let color = Color::srgba(brightness, brightness, brightness + blue_tint, 1.0);

        commands.spawn((
            Starfield { layer: 2 },
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, -9.0),
                ..default()
            },
            Name::new("Star-Mid"),
        ));
    }

    // Layer 3: Close stars (larger, brighter, more parallax)
    for _ in 0..80 {
        let x = min_x + next_random() * (max_x - min_x);
        let y = min_y + next_random() * (max_y - min_y);
        let brightness = 0.7 + next_random() * 0.3; // 0.7-1.0
        let size = 2.0 + next_random() * 2.0; // 2.0-4.0 pixels

        // Some stars have color variation (blue, yellow-white)
        let color_type = next_random();
        let color = if color_type < 0.7 {
            // White/blue-white
            Color::srgba(brightness, brightness, brightness * 1.2, 1.0)
        } else {
            // Yellow-white
            Color::srgba(brightness, brightness * 0.95, brightness * 0.8, 1.0)
        };

        commands.spawn((
            Starfield { layer: 3 },
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(size, size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, -8.0),
                ..default()
            },
            Name::new("Star-Close"),
        ));
    }

    info!("Starfield spawned with 430 stars across 3 layers");
}

fn load_player_ship_texture(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handle = asset_server.load("sprites/player_ship.png");
    commands.insert_resource(PlayerShipTexture(handle));
}

fn sync_camera_view(
    view: Res<ViewMode>,
    zoom: Res<MapZoomOverride>,
    pan: Res<MapPanOffset>,
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
        for mut transform in transforms.iter_mut() {
            transform.translation.x = center.x + pan.offset.x;
            transform.translation.y = center.y + pan.offset.y;
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

/// Handle mouse wheel zoom in map view (always available, not just debug mode)
#[allow(deprecated)]
fn handle_map_zoom_wheel(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut zoom: ResMut<MapZoomOverride>,
) {
    for event in scroll_events.read() {
        // Scroll up = zoom in (decrease scale), scroll down = zoom out (increase scale)
        if event.y > 0.0 {
            zoom.zoom_in(MAP_ZOOM_STEP);
        } else if event.y < 0.0 {
            zoom.zoom_out(MAP_ZOOM_STEP);
        }
    }
}

/// Handle right-click drag panning in map view
fn handle_map_pan(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut pan: ResMut<MapPanOffset>,
    zoom: Res<MapZoomOverride>,
) {
    let window = match windows.single() {
        Ok(w) => w,
        Err(_) => return,
    };

    let cursor_pos = match window.cursor_position() {
        Some(pos) => pos,
        None => {
            // Cursor left window, end drag
            if pan.is_dragging() {
                pan.end_drag();
            }
            return;
        }
    };

    // Right-click to pan (avoids conflict with left-click selection)
    if mouse_button.just_pressed(MouseButton::Right) {
        pan.start_drag(cursor_pos);
    } else if mouse_button.pressed(MouseButton::Right) && pan.is_dragging() {
        let delta = pan.update_drag(cursor_pos, zoom.scale);
        pan.offset += delta;
    } else if mouse_button.just_released(MouseButton::Right) {
        pan.end_drag();
    }
}

fn spawn_node_visuals(
    mut commands: Commands,
    fog: Res<FogConfig>,
    toggles: Res<RenderToggles>,
    nodes: Query<(Entity, &SystemNode, &SystemIntel), Without<NodeVisualMarker>>,
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
        commands.entity(entity).insert(NodeVisualMarker);
    }
}

fn sync_node_visuals(
    mut commands: Commands,
    toggles: Res<RenderToggles>,
    mut visuals: Query<(Entity, &NodeVisual, &mut Transform)>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    if !toggles.show_nodes {
        return;
    }

    for (visual_entity, visual, mut transform) in visuals.iter_mut() {
        match nodes.get(visual.target) {
            Ok((node, intel)) => {
                // Despawn visual if node is no longer revealed
                if !intel.revealed {
                    commands.entity(visual.target).remove::<NodeVisualMarker>();
                    commands.entity(visual_entity).despawn();
                    continue;
                }
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
fn sync_zone_visibility(
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

#[allow(clippy::type_complexity)]
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
            camera_transform.translation.x = player_transform.translation.x;
            camera_transform.translation.y = player_transform.translation.y;
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
    mut beacon: ResMut<HomeBeaconEnabled>,
    mut log: ResMut<crate::plugins::core::EventLog>,
) {
    if !input.just_pressed(bindings.center_camera) {
        return;
    }

    beacon.enabled = !beacon.enabled;

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
        let beacon_status = if beacon.enabled {
            " | Beacon ON"
        } else {
            " | Beacon OFF"
        };
        log.push(format!(
            "World camera centered on node {}{}",
            node_id, beacon_status
        ));
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

fn draw_tactical_navigation(
    mut gizmos: Gizmos,
    targets: Res<NearbyTargets>,
    player_query: Query<&Transform, With<PlayerControl>>,
) {
    if targets.entities.is_empty() {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(transform) => transform,
        Err(_) => {
            return;
        }
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    let selected = match targets.entities.get(targets.selected_index) {
        Some(target) => target,
        None => {
            return;
        }
    };

    let target_pos = selected.1;
    let distance = player_pos.distance(target_pos);

    // Different colors: cyan = unconfirmed, green = confirmed (Tab pressed)
    let (arrow_color, target_color) = if targets.manually_selected {
        (
            Color::srgba(0.2, 1.0, 0.3, 0.9), // Green - locked on
            Color::srgba(0.2, 1.0, 0.3, 0.8),
        )
    } else {
        (
            Color::srgba(0.0, 1.0, 1.0, 0.8), // Cyan - not confirmed
            Color::srgba(0.0, 1.0, 1.0, 0.5),
        )
    };

    // Always draw targeting reticle on the target itself
    let reticle_size = if targets.manually_selected {
        18.0
    } else {
        14.0
    };
    let inner_gap = reticle_size * 0.4;
    // Draw crosshair lines with gap in center
    gizmos.line_2d(
        target_pos + Vec2::new(-reticle_size, 0.0),
        target_pos + Vec2::new(-inner_gap, 0.0),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(inner_gap, 0.0),
        target_pos + Vec2::new(reticle_size, 0.0),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(0.0, -reticle_size),
        target_pos + Vec2::new(0.0, -inner_gap),
        target_color,
    );
    gizmos.line_2d(
        target_pos + Vec2::new(0.0, inner_gap),
        target_pos + Vec2::new(0.0, reticle_size),
        target_color,
    );
    // Draw corner brackets when confirmed
    if targets.manually_selected {
        let bracket_size = reticle_size + 6.0;
        let corner_len = 6.0;
        // Top-left
        gizmos.line_2d(
            target_pos + Vec2::new(-bracket_size, bracket_size),
            target_pos + Vec2::new(-bracket_size + corner_len, bracket_size),
            target_color,
        );
        gizmos.line_2d(
            target_pos + Vec2::new(-bracket_size, bracket_size),
            target_pos + Vec2::new(-bracket_size, bracket_size - corner_len),
            target_color,
        );
        // Top-right
        gizmos.line_2d(
            target_pos + Vec2::new(bracket_size, bracket_size),
            target_pos + Vec2::new(bracket_size - corner_len, bracket_size),
            target_color,
        );
        gizmos.line_2d(
            target_pos + Vec2::new(bracket_size, bracket_size),
            target_pos + Vec2::new(bracket_size, bracket_size - corner_len),
            target_color,
        );
        // Bottom-left
        gizmos.line_2d(
            target_pos + Vec2::new(-bracket_size, -bracket_size),
            target_pos + Vec2::new(-bracket_size + corner_len, -bracket_size),
            target_color,
        );
        gizmos.line_2d(
            target_pos + Vec2::new(-bracket_size, -bracket_size),
            target_pos + Vec2::new(-bracket_size, -bracket_size + corner_len),
            target_color,
        );
        // Bottom-right
        gizmos.line_2d(
            target_pos + Vec2::new(bracket_size, -bracket_size),
            target_pos + Vec2::new(bracket_size - corner_len, -bracket_size),
            target_color,
        );
        gizmos.line_2d(
            target_pos + Vec2::new(bracket_size, -bracket_size),
            target_pos + Vec2::new(bracket_size, -bracket_size + corner_len),
            target_color,
        );
    }

    const NEAR_THRESHOLD: f32 = 150.0;

    // Draw directional arrow near player when target is far
    if distance > NEAR_THRESHOLD {
        let direction = (target_pos - player_pos).normalize_or_zero();
        if direction.length_squared() < 0.01 {
            return;
        }

        let arrow_offset = 40.0;
        let arrow_length = 40.0;
        let arrow_start = player_pos + direction * arrow_offset;
        let arrow_end = arrow_start + direction * arrow_length;

        gizmos.line_2d(arrow_start, arrow_end, arrow_color);

        let tip_size = 8.0;
        let perpendicular = Vec2::new(-direction.y, direction.x);
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * tip_size + perpendicular * (tip_size * 0.5),
            arrow_color,
        );
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * tip_size - perpendicular * (tip_size * 0.5),
            arrow_color,
        );

        // Draw lock-on brackets when confirmed
        if targets.manually_selected {
            let bracket_size = 6.0;
            let bracket_offset = arrow_offset - 10.0;
            let bracket_pos = player_pos + direction * bracket_offset;
            gizmos.line_2d(
                bracket_pos + perpendicular * bracket_size,
                bracket_pos + perpendicular * bracket_size * 0.5,
                arrow_color,
            );
            gizmos.line_2d(
                bracket_pos - perpendicular * bracket_size,
                bracket_pos - perpendicular * bracket_size * 0.5,
                arrow_color,
            );
        }
    }
    // Target reticle already drawn above regardless of distance
}

const BEACON_ARROW_OFFSET: f32 = 40.0;
const BEACON_ARROW_LENGTH: f32 = 40.0;
const BEACON_TIP_SIZE: f32 = 8.0;

fn draw_home_beacon(
    mut gizmos: Gizmos,
    beacon: Res<HomeBeaconEnabled>,
    player_query: Query<&Transform, With<PlayerControl>>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
) {
    if !beacon.enabled {
        return;
    }

    let player_transform = match player_query.single() {
        Ok(t) => t,
        Err(_) => return,
    };

    let player_pos = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.y,
    );

    let mut nearest: Option<Vec2> = None;
    let mut best_dist = f32::MAX;

    for (node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let dist = node.position.distance(player_pos);
        if dist < best_dist {
            best_dist = dist;
            nearest = Some(node.position);
        }
    }

    if let Some(target) = nearest {
        let direction = (target - player_pos).normalize_or_zero();
        if direction.length_squared() < 0.01 {
            return;
        }

        let arrow_start = player_pos + direction * BEACON_ARROW_OFFSET;
        let arrow_end = arrow_start + direction * BEACON_ARROW_LENGTH;

        let beacon_color = Color::srgba(0.0, 1.0, 1.0, 0.8);
        gizmos.line_2d(arrow_start, arrow_end, beacon_color);

        let perpendicular = Vec2::new(-direction.y, direction.x);
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * BEACON_TIP_SIZE + perpendicular * (BEACON_TIP_SIZE * 0.5),
            beacon_color,
        );
        gizmos.line_2d(
            arrow_end,
            arrow_end - direction * BEACON_TIP_SIZE - perpendicular * (BEACON_TIP_SIZE * 0.5),
            beacon_color,
        );
    }
}

fn clear_focus_marker_on_map(mut marker: ResMut<FocusMarker>) {
    marker.position = None;
    marker.node_id = None;
}

fn update_station_labels(
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

#[allow(clippy::too_many_arguments)]
fn sync_view_entities(
    view: Res<ViewMode>,
    mut commands: Commands,
    node_visuals: Query<(Entity, &NodeVisual)>,
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
            for (entity, visual) in node_visuals.iter() {
                commands.entity(visual.target).remove::<NodeVisualMarker>();
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

fn toggle_starfield_visibility(
    view: Res<ViewMode>,
    mut stars: Query<&mut Visibility, With<Starfield>>,
) {
    let visible = matches!(*view, ViewMode::World);
    for mut visibility in stars.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn wrap_starfield(
    camera: Query<&Transform, With<Camera2d>>,
    mut stars: Query<&mut Transform, (With<Starfield>, Without<Camera2d>)>,
) {
    let camera_transform = match camera.iter().next() {
        Some(transform) => transform,
        None => return,
    };

    let camera_x = camera_transform.translation.x;
    let camera_y = camera_transform.translation.y;

    // Wrap distance - teleport stars when they get this far from camera
    // Viewport is ~768x432 at scale 0.6, so wrap at ~1.5x viewport to ensure coverage
    let wrap_x = 600.0;
    let wrap_y = 350.0;
    // Tile size should be 2x wrap distance for seamless wrapping
    let tile_x = 1200.0;
    let tile_y = 700.0;

    for mut star_transform in stars.iter_mut() {
        let dx = star_transform.translation.x - camera_x;
        let dy = star_transform.translation.y - camera_y;

        // If star is too far left, move it to the right
        if dx < -wrap_x {
            star_transform.translation.x += tile_x;
        }
        // If star is too far right, move it to the left
        else if dx > wrap_x {
            star_transform.translation.x -= tile_x;
        }

        // If star is too far down, move it up
        if dy < -wrap_y {
            star_transform.translation.y += tile_y;
        }
        // If star is too far up, move it down
        else if dy > wrap_y {
            star_transform.translation.y -= tile_y;
        }
    }
}

fn map_scale_for_window(_window: Option<&Window>, zoom: &MapZoomOverride) -> f32 {
    zoom.scale
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

#[allow(clippy::too_many_arguments)]
fn update_node_labels(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    toggles: Res<RenderToggles>,
    debug_window: Res<crate::plugins::core::DebugWindow>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    labels: Query<Entity, With<NodeLabel>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    for entity in labels.iter() {
        commands.entity(entity).despawn();
    }

    if !toggles.show_nodes || !toggles.show_node_labels || debug_window.open {
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
        let label = format!(
            "L{} {:.0}% {}",
            layer_short(intel.layer),
            intel.confidence * 100.0,
            modifier_icon(node.modifier),
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
    if !shift_pressed(&input) {
        return;
    }

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
    if !shift_pressed(&input) || !input.just_pressed(bindings.refresh_intel) {
        return;
    }

    if ticks.tick < cooldown.next_allowed_tick {
        return;
    }

    for mut intel in intel_query.iter_mut() {
        refresh_intel(&mut intel, ticks.tick);
    }
    cooldown.next_allowed_tick = ticks.tick.saturating_add(cooldown.cooldown_ticks);
    info!("Intel refreshed");
}

fn handle_intel_advance(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut intel_query: Query<&mut SystemIntel>,
) {
    if !shift_pressed(&input) || !input.just_pressed(bindings.advance_intel) {
        return;
    }

    for mut intel in intel_query.iter_mut() {
        advance_intel_layer(&mut intel);
    }
    info!("Intel layer advanced");
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
        None => "",
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
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
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
    let enter_radius = 14.0;
    // Larger radius to keep hovering - prevents edge flicker
    let keep_radius = 20.0;

    for (node, intel) in nodes.iter() {
        if !intel.revealed {
            continue;
        }
        let dist = node.position.distance(world_pos);
        // Use larger radius if we're already hovering this node (hysteresis)
        let effective_radius = if hovered.id == Some(node.id) {
            keep_radius
        } else {
            enter_radius
        };
        if dist <= effective_radius && dist < closest_dist {
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
    // Only set screen_pos when actually hovering a node
    if closest_id.is_some() {
        hovered.screen_pos = Some(cursor);
        hovered.screen_size = Vec2::new(window.width(), window.height());
    } else {
        hovered.screen_pos = None;
    }
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

/// Determines if an entity should be visible based on zone matching.
/// An entity is visible if it's in the same zone as the player.
/// Entities without a zone are always visible (backwards compatibility).
fn is_visible_in_zone(entity_zone: Option<u32>, player_zone: u32) -> bool {
    match entity_zone {
        Some(zone) => zone == player_zone,
        None => true, // Entities without zones are always visible
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_visible_in_zone, map_center, risk_color, MapPanOffset, MapZoomOverride,
        MAP_ZOOM_DEFAULT, MAP_ZOOM_MAX, MAP_ZOOM_MIN, MAP_ZOOM_STEP,
    };
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
    fn map_zoom_default_is_point_eight() {
        let zoom = MapZoomOverride::default();
        assert_close(zoom.scale, MAP_ZOOM_DEFAULT);
    }

    #[test]
    fn map_zoom_in_decreases_scale() {
        let mut zoom = MapZoomOverride::default();
        let before = zoom.scale;
        zoom.zoom_in(MAP_ZOOM_STEP);
        assert!(zoom.scale < before);
    }

    #[test]
    fn map_zoom_out_increases_scale() {
        let mut zoom = MapZoomOverride::default();
        let before = zoom.scale;
        zoom.zoom_out(MAP_ZOOM_STEP);
        assert!(zoom.scale > before);
    }

    #[test]
    fn map_zoom_clamps_at_min() {
        let mut zoom = MapZoomOverride::default();
        for _ in 0..50 {
            zoom.zoom_in(MAP_ZOOM_STEP);
        }
        assert_close(zoom.scale, MAP_ZOOM_MIN);
    }

    #[test]
    fn map_zoom_clamps_at_max() {
        let mut zoom = MapZoomOverride::default();
        for _ in 0..50 {
            zoom.zoom_out(MAP_ZOOM_STEP);
        }
        assert_close(zoom.scale, MAP_ZOOM_MAX);
    }

    #[test]
    fn map_pan_default_is_zero() {
        let pan = MapPanOffset::default();
        assert_close(pan.offset.x, 0.0);
        assert_close(pan.offset.y, 0.0);
    }

    #[test]
    fn map_pan_start_drag_sets_dragging() {
        let mut pan = MapPanOffset::default();
        assert!(!pan.is_dragging());
        pan.start_drag(Vec2::new(100.0, 100.0));
        assert!(pan.is_dragging());
    }

    #[test]
    fn map_pan_end_drag_clears_dragging() {
        let mut pan = MapPanOffset::default();
        pan.start_drag(Vec2::new(100.0, 100.0));
        pan.end_drag();
        assert!(!pan.is_dragging());
    }

    #[test]
    fn map_pan_update_returns_delta() {
        let mut pan = MapPanOffset::default();
        pan.start_drag(Vec2::new(100.0, 100.0));
        let delta = pan.update_drag(Vec2::new(110.0, 90.0), 1.0);
        // Delta is inverted x, normal y: (-10, -10) for input delta (10, -10)
        assert_close(delta.x, -10.0);
        assert_close(delta.y, -10.0);
    }

    #[test]
    fn map_pan_update_scales_by_camera_scale() {
        let mut pan = MapPanOffset::default();
        pan.start_drag(Vec2::new(100.0, 100.0));
        let delta = pan.update_drag(Vec2::new(110.0, 100.0), 2.0);
        // With scale 2.0, delta should be doubled
        assert_close(delta.x, -20.0);
    }

    #[test]
    fn map_pan_not_dragging_returns_zero() {
        let mut pan = MapPanOffset::default();
        let delta = pan.update_drag(Vec2::new(110.0, 100.0), 1.0);
        assert_close(delta.x, 0.0);
        assert_close(delta.y, 0.0);
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
    fn map_zoom_label_shows_scale() {
        let zoom = MapZoomOverride::default();
        assert_eq!(zoom.label(), "0.80");
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
    fn risk_color_low_blue_component_gt_red() {
        let low = risk_color(0.0);
        assert!(low.linear_b() > low.linear_r());
    }

    #[test]
    fn risk_color_high_green_component_lt_blue() {
        let high = risk_color(1.0);
        assert!(high.linear_g() > high.linear_b());
    }

    // Zone visibility tests

    #[test]
    fn entity_in_same_zone_is_visible() {
        assert!(is_visible_in_zone(Some(100), 100));
    }

    #[test]
    fn entity_in_different_zone_is_not_visible() {
        assert!(!is_visible_in_zone(Some(200), 100));
    }

    #[test]
    fn entity_without_zone_is_always_visible() {
        assert!(is_visible_in_zone(None, 100));
        assert!(is_visible_in_zone(None, 999));
    }

    #[test]
    fn entity_visibility_changes_with_player_zone() {
        // Entity in zone 100
        let entity_zone = Some(100);

        // Visible when player in zone 100
        assert!(is_visible_in_zone(entity_zone, 100));

        // Not visible when player moves to zone 200
        assert!(!is_visible_in_zone(entity_zone, 200));
    }

    #[test]
    fn multiple_entities_different_zones() {
        let player_zone = 100;

        // Entity in player's zone - visible
        assert!(is_visible_in_zone(Some(100), player_zone));

        // Entity in adjacent zone - not visible
        assert!(!is_visible_in_zone(Some(101), player_zone));

        // Entity in distant zone - not visible
        assert!(!is_visible_in_zone(Some(999), player_zone));
    }
}
