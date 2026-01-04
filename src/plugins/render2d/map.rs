//! Map view rendering: nodes, routes, intel rings, and labels.

use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use bevy::window::PrimaryWindow;
use std::path::Path;

use crate::compat::{SpriteBundle, TextBundle, TextStyle};
use crate::plugins::core::FogConfig;
use crate::plugins::ui::{HoveredNode, MapUi};
use crate::world::{KnowledgeLayer, Sector, SystemIntel, SystemNode};

use super::components::{
    find_node_position, layer_floor, layer_short, modifier_icon, risk_color, NodeLabel, NodeVisual,
    NodeVisualMarker, RouteLabel,
};

// =============================================================================
// Resources
// =============================================================================

#[derive(Resource)]
pub struct RenderToggles {
    pub show_nodes: bool,
    pub show_routes: bool,
    pub show_rings: bool,
    pub show_grid: bool,
    pub show_route_labels: bool,
    pub show_node_labels: bool,
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

#[derive(Resource)]
pub struct IntelRefreshCooldown {
    pub next_allowed_tick: u64,
    pub cooldown_ticks: u64,
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
    pub position: Option<Vec2>,
    pub node_id: Option<u32>,
}

impl FocusMarker {
    pub fn position(&self) -> Option<Vec2> {
        self.position
    }

    pub fn node_id(&self) -> Option<u32> {
        self.node_id
    }
}

// =============================================================================
// Systems
// =============================================================================

pub fn spawn_node_visuals(
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
                custom_size: Some(Vec2::splat(24.0)),
                ..default()
            },
            transform: Transform::from_xyz(node.position.x, node.position.y, 0.0),
            ..default()
        };

        commands.spawn((NodeVisual { target: entity }, sprite));
        commands.entity(entity).insert(NodeVisualMarker);
    }
}

pub fn sync_node_visuals(
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

pub fn update_node_visuals(
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

pub fn draw_intel_rings(
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
        let radius = 20.0 + (1.0 - t) * 12.0;
        gizmos.circle_2d(node.position, radius, color);
    }
}

pub fn draw_routes(
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

pub fn update_route_labels(
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
pub fn update_node_labels(
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
            "{} L{} {:.0}% {}",
            node.id,
            layer_short(intel.layer),
            intel.confidence * 100.0,
            modifier_icon(node.modifier),
        );

        let position = node.position + Vec2::new(0.0, 35.0);
        if let Ok(screen) = camera.world_to_viewport(camera_transform, position.extend(0.0)) {
            let label_pos = Vec2::new(screen.x + 6.0, screen.y - 14.0);
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

pub fn update_hovered_node(
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
    // Hover radius matches intel ring size (20-32 world units)
    let enter_radius = 35.0;
    // Larger radius to keep hovering - prevents edge flicker
    let keep_radius = 40.0;

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

pub fn clear_focus_marker_on_map(mut marker: ResMut<FocusMarker>) {
    marker.position = None;
    marker.node_id = None;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32) {
        let diff = (a - b).abs();
        assert!(diff < 1e-6, "expected {} close to {}", a, b);
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
}
