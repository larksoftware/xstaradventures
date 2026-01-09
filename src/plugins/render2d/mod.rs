//! 2D rendering plugin for map and world views.

mod camera;
mod components;
mod debug;
mod effects;
mod entities;
mod map;
mod starfield;

use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;

use crate::plugins::core::{GameState, ViewMode};

// Re-export public types
pub use camera::{MapPanOffset, MapZoomOverride};
pub use components::{
    NodeLabel, NodeVisual, NodeVisualMarker, OreVisual, PirateBaseVisual, PirateShipVisual,
    RouteLabel, ShipLabel, ShipVisual, ShipVisualMarker, StationLabel, StationVisual,
};
pub use map::{FocusMarker, IntelRefreshCooldown, RenderToggles};

// =============================================================================
// Plugin
// =============================================================================

pub struct Render2DPlugin;

impl Plugin for Render2DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderToggles>()
            .init_resource::<IntelRefreshCooldown>()
            .init_resource::<MapZoomOverride>()
            .init_resource::<MapPanOffset>()
            .init_resource::<FocusMarker>()
            .init_resource::<effects::HomeBeaconEnabled>()
            .add_systems(Startup, entities::load_player_ship_texture)
            .add_systems(Startup, camera::setup_camera)
            .add_systems(
                OnEnter(GameState::InGame),
                (
                    camera::track_player_camera,
                    starfield::spawn_starfield,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                starfield::wrap_starfield
                    .after(camera::track_player_camera)
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_world),
            )
            .add_systems(
                Update,
                starfield::toggle_starfield_visibility.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                camera::sync_camera_view.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (
                    map::clear_focus_marker_on_map,
                    map::spawn_node_visuals,
                    map::sync_node_visuals,
                    map::update_node_visuals,
                    map::draw_intel_rings,
                    map::draw_routes,
                    map::update_route_labels,
                    map::update_node_labels,
                    map::update_station_map_labels,
                    map::update_hovered_node,
                    sync_view_entities,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_map),
            )
            .add_systems(
                Update,
                (
                    debug::handle_render_toggles,
                    debug::handle_intel_refresh,
                    debug::handle_intel_advance,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_map)
                    .run_if(camera::debug_window_open),
            )
            .add_systems(
                Update,
                (camera::handle_map_zoom_wheel, camera::handle_map_pan)
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_map),
            )
            .add_systems(
                Update,
                (
                    entities::spawn_station_visuals,
                    entities::sync_station_visuals,
                    entities::update_station_labels,
                    entities::spawn_ore_visuals,
                    entities::sync_ore_visuals,
                    entities::update_ore_visuals,
                    entities::spawn_pirate_base_visuals,
                    entities::sync_pirate_base_visuals,
                    entities::spawn_pirate_ship_visuals,
                    entities::sync_pirate_ship_visuals,
                    entities::spawn_ship_visuals,
                    entities::sync_ship_visuals,
                    entities::update_ship_visuals,
                    entities::update_ship_labels,
                    entities::sync_zone_visibility,
                    effects::draw_focus_marker,
                    effects::draw_tactical_navigation,
                    effects::draw_home_beacon,
                    sync_view_entities,
                )
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_world),
            )
            .add_systems(
                Update,
                camera::track_player_camera
                    .after(entities::sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_world),
            )
            .add_systems(
                Update,
                camera::center_camera_on_revealed
                    .after(entities::sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_world),
            )
            .add_systems(
                Update,
                debug::debug_player_components
                    .after(entities::sync_ship_visuals)
                    .run_if(in_state(GameState::InGame))
                    .run_if(camera::view_is_world)
                    .run_if(camera::debug_window_open),
            );
    }
}

// =============================================================================
// View Sync System
// =============================================================================

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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Sector, SystemNode};

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
        let center = components::map_center(&sector);
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

        let center = components::map_center(&sector);
        assert_close(center.x, 20.0);
        assert_close(center.y, 30.0);
    }

    #[test]
    fn map_center_single_node_equals_position() {
        let mut sector = Sector::default();
        sector.nodes.push(SystemNode {
            id: 1,
            position: Vec2::new(-12.0, 48.0),
            modifier: None,
        });

        let center = components::map_center(&sector);
        assert_close(center.x, -12.0);
        assert_close(center.y, 48.0);
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

        let center = components::map_center(&sector);
        assert_close(center.x, 3.0);
        assert_close(center.y, 4.0);
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

        let center = components::map_center(&sector);
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

        let center = components::map_center(&sector);
        assert_close(center.x, 0.0);
        assert_close(center.y, 0.0);
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

        let center = components::map_center(&sector);
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

        let center = components::map_center(&sector);
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

        let center = components::map_center(&sector);
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

        let center = components::map_center(&sector);
        assert_close(center.x, 0.0);
        assert_close(center.y, 0.0);
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

        let center = components::map_center(&sector);
        assert_close(center.x, 4.0);
        assert_close(center.y, 6.0);
    }

    // Zone visibility tests

    #[test]
    fn entity_in_same_zone_is_visible() {
        assert!(components::is_visible_in_zone(Some(100), 100));
    }

    #[test]
    fn entity_in_different_zone_is_not_visible() {
        assert!(!components::is_visible_in_zone(Some(200), 100));
    }

    #[test]
    fn entity_without_zone_is_always_visible() {
        assert!(components::is_visible_in_zone(None, 100));
        assert!(components::is_visible_in_zone(None, 999));
    }

    #[test]
    fn entity_visibility_changes_with_player_zone() {
        // Entity in zone 100
        let entity_zone = Some(100);

        // Visible when player in zone 100
        assert!(components::is_visible_in_zone(entity_zone, 100));

        // Not visible when player moves to zone 200
        assert!(!components::is_visible_in_zone(entity_zone, 200));
    }

    #[test]
    fn multiple_entities_different_zones() {
        let player_zone = 100;

        // Entity in player's zone - visible
        assert!(components::is_visible_in_zone(Some(100), player_zone));

        // Entity in adjacent zone - not visible
        assert!(!components::is_visible_in_zone(Some(101), player_zone));

        // Entity in distant zone - not visible
        assert!(!components::is_visible_in_zone(Some(999), player_zone));
    }

    // risk_color tests

    #[test]
    fn risk_color_midpoint_green_between_low_high() {
        let mid = components::risk_color(0.5);
        let low = components::risk_color(0.0);
        let high = components::risk_color(1.0);

        let mid_linear = LinearRgba::from(mid);
        let low_linear = LinearRgba::from(low);
        let high_linear = LinearRgba::from(high);

        assert!(mid_linear.green < low_linear.green);
        assert!(mid_linear.green > high_linear.green);
    }

    #[test]
    fn risk_color_clamps_below_zero() {
        let below = components::risk_color(-0.5);
        let low = components::risk_color(0.0);
        assert_close(below.linear_r(), low.linear_r());
        assert_close(below.linear_g(), low.linear_g());
        assert_close(below.linear_b(), low.linear_b());
    }

    #[test]
    fn risk_color_clamps_above_one() {
        let above = components::risk_color(1.5);
        let high = components::risk_color(1.0);
        assert_close(above.linear_r(), high.linear_r());
        assert_close(above.linear_g(), high.linear_g());
        assert_close(above.linear_b(), high.linear_b());
    }

    #[test]
    fn risk_color_midpoint_components_between_extremes() {
        let mid = components::risk_color(0.5);
        let low = components::risk_color(0.0);
        let high = components::risk_color(1.0);

        assert!(mid.linear_r() >= low.linear_r() && mid.linear_r() <= high.linear_r());
        assert!(mid.linear_g() <= low.linear_g() && mid.linear_g() >= high.linear_g());
        assert!(mid.linear_b() <= low.linear_b() && mid.linear_b() >= high.linear_b());
    }

    #[test]
    fn risk_color_midpoint_matches_linear_mix() {
        let mid = components::risk_color(0.5);
        let low = components::risk_color(0.0);
        let high = components::risk_color(1.0);

        let expected_r = (low.linear_r() + high.linear_r()) * 0.5;
        let expected_g = (low.linear_g() + high.linear_g()) * 0.5;
        let expected_b = (low.linear_b() + high.linear_b()) * 0.5;

        assert_close(mid.linear_r(), expected_r);
        assert_close(mid.linear_g(), expected_g);
        assert_close(mid.linear_b(), expected_b);
    }

    #[test]
    fn risk_color_midpoint_is_avg_of_endpoints() {
        let low = components::risk_color(0.0);
        let high = components::risk_color(1.0);
        let mid = components::risk_color(0.5);

        assert_close(mid.linear_r(), (low.linear_r() + high.linear_r()) * 0.5);
        assert_close(mid.linear_g(), (low.linear_g() + high.linear_g()) * 0.5);
        assert_close(mid.linear_b(), (low.linear_b() + high.linear_b()) * 0.5);
    }

    #[test]
    fn risk_color_midpoint_red_gt_low() {
        let mid = components::risk_color(0.5);
        let low = components::risk_color(0.0);

        assert!(mid.linear_r() > low.linear_r());
    }

    #[test]
    fn risk_color_midpoint_green_lt_low() {
        let mid = components::risk_color(0.5);
        let low = components::risk_color(0.0);

        assert!(mid.linear_g() < low.linear_g());
    }

    #[test]
    fn risk_color_midpoint_blue_lt_low() {
        let mid = components::risk_color(0.5);
        let low = components::risk_color(0.0);

        assert!(mid.linear_b() < low.linear_b());
    }

    #[test]
    fn risk_color_midpoint_red_lt_high() {
        let mid = components::risk_color(0.5);
        let high = components::risk_color(1.0);

        assert!(mid.linear_r() < high.linear_r());
    }

    #[test]
    fn risk_color_midpoint_green_gt_high() {
        let mid = components::risk_color(0.5);
        let high = components::risk_color(1.0);

        assert!(mid.linear_g() > high.linear_g());
    }

    #[test]
    fn risk_color_midpoint_blue_gt_high() {
        let mid = components::risk_color(0.5);
        let high = components::risk_color(1.0);

        assert!(mid.linear_b() > high.linear_b());
    }

    #[test]
    fn risk_color_low_matches_constants() {
        let low = components::risk_color(0.0);
        assert_close(low.linear_r(), 0.2);
        assert_close(low.linear_g(), 0.7);
        assert_close(low.linear_b(), 0.4);
    }

    #[test]
    fn risk_color_high_matches_constants() {
        let high = components::risk_color(1.0);
        assert_close(high.linear_r(), 0.9);
        assert_close(high.linear_g(), 0.25);
        assert_close(high.linear_b(), 0.2);
    }

    #[test]
    fn risk_color_low_green_component_max() {
        let low = components::risk_color(0.0);
        assert!(low.linear_g() > low.linear_r());
        assert!(low.linear_g() > low.linear_b());
    }

    #[test]
    fn risk_color_high_red_component_max() {
        let high = components::risk_color(1.0);
        assert!(high.linear_r() > high.linear_g());
        assert!(high.linear_r() > high.linear_b());
    }

    #[test]
    fn risk_color_low_blue_component_gt_red() {
        let low = components::risk_color(0.0);
        assert!(low.linear_b() > low.linear_r());
    }

    #[test]
    fn risk_color_high_green_component_lt_blue() {
        let high = components::risk_color(1.0);
        assert!(high.linear_g() > high.linear_b());
    }
}
