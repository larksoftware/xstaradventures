//! Camera systems, zoom, and pan functionality.

use bevy::camera::{OrthographicProjection, Projection};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::compat::Camera2dBundle;
use crate::plugins::core::{DebugWindow, InputBindings, ViewMode};
use crate::world::{Sector, SystemIntel, SystemNode};

use super::components::map_center;
use super::effects::HomeBeaconEnabled;
use super::map::FocusMarker;

// =============================================================================
// Constants
// =============================================================================

#[allow(dead_code)]
pub const MAP_EXTENT_X: f32 = 600.0;
#[allow(dead_code)]
pub const MAP_EXTENT_Y: f32 = 360.0;

/// Zoom configuration for map view
pub const MAP_ZOOM_MIN: f32 = 0.3;
pub const MAP_ZOOM_MAX: f32 = 2.0;
pub const MAP_ZOOM_DEFAULT: f32 = 0.8;
pub const MAP_ZOOM_STEP: f32 = 0.1;

// =============================================================================
// Resources
// =============================================================================

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

// =============================================================================
// Systems
// =============================================================================

pub fn setup_camera(mut commands: Commands) {
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

pub fn sync_camera_view(
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

pub fn track_player_camera(
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

pub fn center_camera_on_revealed(
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

/// Handle mouse wheel zoom in map view (always available, not just debug mode)
#[allow(deprecated)]
pub fn handle_map_zoom_wheel(
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
pub fn handle_map_pan(
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

// =============================================================================
// Run Conditions
// =============================================================================

pub fn view_is_map(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::Map)
}

pub fn view_is_world(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::World)
}

pub fn debug_window_open(debug_window: Res<DebugWindow>) -> bool {
    debug_window.open
}

// =============================================================================
// Utility Functions
// =============================================================================

pub fn map_scale_for_window(_window: Option<&Window>, zoom: &MapZoomOverride) -> f32 {
    zoom.scale
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
    fn map_zoom_label_shows_scale() {
        let zoom = MapZoomOverride::default();
        assert_eq!(zoom.label(), "0.80");
    }
}
