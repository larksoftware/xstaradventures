use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use bevy::ui::Node as UiNode;
use std::path::Path;

use crate::compat::{NodeBundle, TextBundle, TextStyle};
use crate::fleets::{RiskTolerance, ScoutBehavior};
use crate::plugins::core::DebugWindow;
use crate::plugins::core::EventLog;
use crate::plugins::core::GameState;
use crate::plugins::core::SimConfig;
use crate::plugins::core::ViewMode;
use crate::plugins::player::{NearbyTargets, PlayerControl};
use crate::plugins::render2d::FocusMarker;
use crate::plugins::render2d::IntelRefreshCooldown;
use crate::plugins::render2d::MapZoomOverride;
use crate::plugins::render2d::RenderToggles;
use crate::plugins::sim::SimTickCount;
use crate::plugins::worldgen::WorldSeed;
use crate::ships::{Cargo, Ship};
use crate::stations::Station;
use crate::world::ZoneId;
use crate::world::{
    zone_modifier_effect, KnowledgeLayer, Sector, SystemIntel, SystemNode, ZoneModifier,
};
pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_map_grid, setup_hud, setup_debug_panel))
            .add_systems(
                Update,
                (ui_root, update_hud, update_debug_panel).run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (
                    update_log_panel,
                    update_cooldown_panel,
                    update_player_panel,
                    update_fleet_panel,
                    update_focus_panel,
                    sync_map_ui_visibility,
                    sync_map_grid_visibility,
                ),
            )
            .add_systems(
                Update,
                (
                    update_node_panel,
                    update_hover_panel,
                    update_risk_panel,
                    update_modifier_panel,
                )
                    .run_if(view_is_map),
            )
            .add_systems(
                Update,
                (
                    update_tactical_panel,
                    handle_contact_clicks,
                    update_contact_item_styles,
                    update_intel_panel,
                    handle_fleet_clicks,
                    update_fleet_item_styles,
                    update_fleet_detail,
                    handle_panel_scroll,
                )
                    .run_if(view_is_world),
            )
            .init_resource::<HoveredNode>()
            .init_resource::<SelectedFleetUnit>();
    }
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct LogPanelMarker;

#[derive(Component)]
struct LogContentText;

#[derive(Component)]
struct NodeListText;

#[derive(Component)]
struct HoverText;

#[derive(Resource, Default)]
pub struct HoveredNode {
    pub id: Option<u32>,
    pub layer: Option<KnowledgeLayer>,
    pub confidence: f32,
    pub modifier: Option<ZoneModifier>,
    pub screen_pos: Option<Vec2>,
    pub screen_size: Vec2,
}

#[derive(Component)]
struct RiskText;

#[derive(Component)]
struct ModifierPanelText;

/// Position anchor for panels on the screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PanelPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Configuration for creating a panel
#[derive(Debug, Clone)]
pub struct PanelConfig {
    pub position: PanelPosition,
    pub margin: f32,
    pub background_color: Option<Color>,
    #[allow(dead_code)]
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub padding: f32,
    #[allow(dead_code)]
    pub title: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub overflow_scroll: bool,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            position: PanelPosition::TopLeft,
            margin: 14.0,
            background_color: None,
            border_color: None,
            border_width: 0.0,
            padding: 0.0,
            title: None,
            width: None,
            height: None,
            overflow_scroll: false,
        }
    }
}

impl PanelConfig {
    /// Creates a new panel config with the given position
    pub fn at(position: PanelPosition) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Sets the margin from screen edges
    pub fn with_margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the background color (None for transparent)
    pub fn with_background(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Sets the padding inside the panel
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the border color and width
    #[allow(dead_code)]
    pub fn with_border(mut self, color: Color, width: f32) -> Self {
        self.border_color = Some(color);
        self.border_width = width;
        self
    }

    /// Sets the panel title
    #[allow(dead_code)]
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Sets the panel width and height
    #[allow(dead_code)]
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Enables vertical scrolling when content exceeds height
    #[allow(dead_code)]
    pub fn with_scroll(mut self) -> Self {
        self.overflow_scroll = true;
        self
    }

    /// Applies this config to a UiNode, setting position properties
    pub fn apply_to_node(&self, node: &mut UiNode) {
        node.position_type = PositionType::Absolute;

        match self.position {
            PanelPosition::TopLeft => {
                node.left = Val::Px(self.margin);
                node.top = Val::Px(self.margin);
            }
            PanelPosition::TopRight => {
                node.right = Val::Px(self.margin);
                node.top = Val::Px(self.margin);
            }
            PanelPosition::BottomLeft => {
                node.left = Val::Px(self.margin);
                node.bottom = Val::Px(self.margin);
            }
            PanelPosition::BottomRight => {
                node.right = Val::Px(self.margin);
                node.bottom = Val::Px(self.margin);
            }
        }

        if self.padding > 0.0 {
            node.padding = UiRect::all(Val::Px(self.padding));
        }

        if self.border_width > 0.0 {
            node.border = UiRect::all(Val::Px(self.border_width));
        }

        if let Some(width) = self.width {
            node.width = Val::Px(width);
        }

        if let Some(height) = self.height {
            node.height = Val::Px(height);
        }

        if self.overflow_scroll {
            node.overflow.y = OverflowAxis::Scroll;
        }
    }
}

#[derive(Component)]
struct CooldownText;

#[derive(Component)]
struct FocusText;

#[derive(Component)]
struct FleetPanelMarker;

#[derive(Component)]
struct FleetListContainer;

#[derive(Component)]
struct FleetDetailText;

/// Component marking a clickable fleet item in the Fleet panel
#[derive(Component)]
pub struct FleetItem {
    pub index: usize,
}

/// Marker for empty state text in Fleet panel
#[derive(Component)]
struct FleetEmptyText;

/// Marker for the divider between fleet list and detail
#[derive(Component)]
struct FleetDetailDivider;

/// Tracks which fleet unit is selected for detail view
#[derive(Resource, Default)]
pub struct SelectedFleetUnit {
    pub index: Option<usize>,
}

#[derive(Component)]
struct PlayerPanelText;

#[derive(Component)]
struct TacticalPanelText;

/// Component marking the contacts list container
#[derive(Component)]
struct ContactsListContainer;

/// Component marking the Intel panel
#[derive(Component)]
struct IntelPanelText;

/// Component marking the Intel panel content text
#[derive(Component)]
struct IntelContentText;

/// Information about a targeted entity for the Intel panel
#[derive(Debug, Clone)]
pub struct IntelInfo {
    #[allow(dead_code)]
    pub entity: Entity,
    pub label: String,
    pub position: Vec2,
    pub distance: f32,
}

/// Component marking a clickable contact item in the Contacts panel
#[derive(Component)]
pub struct ContactItem {
    pub index: usize,
}

/// Returns the color for a contact item based on selection and hover state
pub fn contact_item_color(is_selected: bool, is_hovered: bool) -> Color {
    match (is_selected, is_hovered) {
        (true, _) => Color::srgb(1.0, 1.0, 1.0), // Selected: white
        (false, true) => Color::srgb(0.5, 0.9, 0.9), // Hovered: bright cyan
        (false, false) => Color::srgb(0.0, 1.0, 1.0), // Default: cyan
    }
}

#[derive(Component)]
struct DebugPanelText;

#[derive(Component)]
pub struct MapUi;

#[derive(Component)]
struct WorldUi;

#[derive(Component)]
struct MapGridRoot;

#[derive(Component)]
struct MapGridLine;
fn ui_root() {
    // Placeholder: delegation panels and problems feed will render here.
}

fn setup_map_grid(mut commands: Commands) {
    let grid_color = Color::srgba(0.2, 0.25, 0.3, 0.35);
    let line_thickness = 1.0;
    let divisions = 12;

    commands
        .spawn((
            MapGridRoot,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: Color::NONE.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            for i in 1..divisions {
                let percent = (i as f32) * 100.0 / (divisions as f32);

                parent.spawn((
                    MapGridLine,
                    NodeBundle {
                        node: UiNode {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(percent),
                            top: Val::Px(0.0),
                            width: Val::Px(line_thickness),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: grid_color.into(),
                        ..default()
                    },
                ));

                parent.spawn((
                    MapGridLine,
                    NodeBundle {
                        node: UiNode {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Percent(percent),
                            width: Val::Percent(100.0),
                            height: Val::Px(line_thickness),
                            ..default()
                        },
                        background_color: grid_color.into(),
                        ..default()
                    },
                ));
            }
        });
}

fn setup_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        info!("HUD font not found at {}", font_on_disk.display());
        return;
    }

    let font = asset_server.load(font_path);

    commands.spawn((
        HudText,
        TextBundle::from_section(
            "Seed: -- | Tick: --",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::srgb(0.9, 0.9, 0.95),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        }),
    ));

    commands.spawn((
        PlayerPanelText,
        WorldUi,
        TextBundle::from_section(
            "Player: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.82, 0.88, 0.95),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(36.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Icons: R Rad | N Neb | O Ore | C Clear | . None",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.6, 0.65, 0.72),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(118.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Map: G grid | R routes | T route labels | Y node labels | V reveal adj | A reveal all",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.6, 0.65, 0.72),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(136.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "Route label: distance + risk",
            TextStyle {
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.6, 0.65, 0.72),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(154.0),
            ..default()
        }),
    ));

    // Subspace Transmissions panel (log) at bottom-left
    commands
        .spawn((
            LogPanelMarker,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    left: Val::Px(14.0),
                    bottom: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    min_width: Val::Px(280.0),
                    max_height: Val::Px(160.0),
                    overflow: Overflow {
                        y: OverflowAxis::Scroll,
                        ..default()
                    },
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                border_color: Color::srgb(0.6, 0.4, 0.8).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Subspace Transmissions",
                TextStyle {
                    font: font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.8, 0.6, 1.0),
                },
            ));

            // Real divider line
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                background_color: Color::srgb(0.4, 0.25, 0.5).into(),
                ..default()
            });

            // Content
            parent.spawn((
                LogContentText,
                TextBundle::from_section(
                    "Awaiting signal...",
                    TextStyle {
                        font: font.clone(),
                        font_size: 12.0,
                        color: Color::srgb(0.7, 0.75, 0.82),
                    },
                ),
            ));
        });

    commands.spawn((
        NodeListText,
        MapUi,
        TextBundle::from_section(
            "Nodes: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.7, 0.75, 0.82),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(10.0),
            ..default()
        }),
    ));

    commands.spawn((
        HoverText,
        MapUi,
        TextBundle::from_section(
            "",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.7, 0.75, 0.82),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        }),
    ));

    commands.spawn((
        RiskText,
        MapUi,
        TextBundle::from_section(
            "Risk: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.7, 0.75, 0.82),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(220.0),
            ..default()
        }),
    ));

    commands.spawn((
        ModifierPanelText,
        MapUi,
        TextBundle::from_section(
            "Modifiers: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.7, 0.75, 0.82),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(260.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "N",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Px(8.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "S",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(8.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "W",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            top: Val::Percent(50.0),
            ..default()
        }),
    ));

    commands.spawn((
        MapUi,
        TextBundle::from_section(
            "E",
            TextStyle {
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.65, 0.7, 0.78),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            top: Val::Percent(50.0),
            ..default()
        }),
    ));

    commands.spawn((
        CooldownText,
        TextBundle::from_section(
            "Intel refresh: --",
            TextStyle {
                font: font.clone(),
                font_size: 14.0,
                color: Color::srgb(0.7, 0.75, 0.82),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(300.0),
            ..default()
        }),
    ));

    commands.spawn((
        FocusText,
        WorldUi,
        TextBundle::from_section(
            "Focus: --",
            TextStyle {
                font: font.clone(),
                font_size: 13.0,
                color: Color::srgb(0.7, 0.8, 0.9),
            },
        )
        .with_node(UiNode {
            position_type: PositionType::Absolute,
            right: Val::Px(14.0),
            top: Val::Px(460.0),
            ..default()
        }),
    ));

    // Fleet panel at top-right
    commands
        .spawn((
            FleetPanelMarker,
            WorldUi,
            Interaction::None,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    top: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    min_width: Val::Px(200.0),
                    max_height: Val::Px(180.0),
                    overflow: Overflow {
                        y: OverflowAxis::Scroll,
                        ..default()
                    },
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Fleet",
                TextStyle {
                    font: font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.0, 1.0, 1.0),
                },
            ));

            // Real divider line
            parent.spawn(NodeBundle {
                node: UiNode {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                background_color: Color::srgb(0.0, 0.5, 0.5).into(),
                ..default()
            });

            // List container for fleet items (populated dynamically)
            parent.spawn((
                FleetListContainer,
                NodeBundle {
                    node: UiNode {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                },
            ));

            // Detail divider (hidden by default, shown when item selected)
            parent.spawn((
                FleetDetailDivider,
                NodeBundle {
                    node: UiNode {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    },
                    background_color: Color::srgb(0.0, 0.4, 0.4).into(),
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));

            // Detail section (shown when a fleet unit is selected)
            parent.spawn((
                FleetDetailText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: font.clone(),
                        font_size: 11.0,
                        color: Color::srgb(0.5, 0.7, 0.7),
                    },
                ),
            ));
        });

    // Wrapper container for Intel + Contacts panels at bottom-right
    commands
        .spawn((
            WorldUi,
            NodeBundle {
                node: UiNode {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    bottom: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0), // Gap between Intel and Contacts
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            // Intel panel (above Contacts)
            parent
                .spawn((
                    IntelPanelText,
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(140.0),
                            max_height: Val::Px(120.0),
                            overflow: Overflow {
                                y: OverflowAxis::Scroll,
                                ..default()
                            },
                            ..default()
                        },
                        background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                        border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                        ..default()
                    },
                ))
                .with_children(|intel| {
                    // Title
                    intel.spawn(TextBundle::from_section(
                        "Intel",
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.0, 1.0, 1.0),
                        },
                    ));

                    // Real divider line
                    intel.spawn(NodeBundle {
                        node: UiNode {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.0, 0.5, 0.5).into(),
                        ..default()
                    });

                    // Content (will be updated dynamically)
                    intel.spawn((
                        IntelContentText,
                        TextBundle::from_section(
                            "No target selected",
                            TextStyle {
                                font: font.clone(),
                                font_size: 13.0,
                                color: Color::srgb(0.6, 0.8, 0.8),
                            },
                        ),
                    ));
                });

            // Contacts panel (below Intel)
            parent
                .spawn((
                    TacticalPanelText,
                    Interaction::default(),
                    ScrollPosition::default(),
                    NodeBundle {
                        node: UiNode {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            min_width: Val::Px(140.0),
                            max_height: Val::Px(150.0),
                            overflow: Overflow {
                                y: OverflowAxis::Scroll,
                                ..default()
                            },
                            ..default()
                        },
                        background_color: Color::srgba(0.02, 0.05, 0.08, 0.85).into(),
                        border_color: Color::srgb(0.0, 0.8, 0.8).into(),
                        ..default()
                    },
                ))
                .with_children(|contacts| {
                    // Title
                    contacts.spawn(TextBundle::from_section(
                        "Contacts",
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color: Color::srgb(0.0, 1.0, 1.0),
                        },
                    ));

                    // Real divider line
                    contacts.spawn(NodeBundle {
                        node: UiNode {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.0, 0.5, 0.5).into(),
                        ..default()
                    });

                    // Container for contact items (will be populated dynamically)
                    contacts.spawn((
                        ContactsListContainer,
                        NodeBundle {
                            node: UiNode {
                                flex_direction: FlexDirection::Column,
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });
        });
}

fn setup_debug_panel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);

    if !font_on_disk.exists() {
        return;
    }

    let font = asset_server.load(font_path);

    // Debug panel config with background and padding
    let debug_config = PanelConfig::at(PanelPosition::TopLeft)
        .with_margin(14.0)
        .with_background(Color::srgb(0.08, 0.1, 0.12))
        .with_padding(10.0);

    let mut debug_node = UiNode {
        width: Val::Auto,
        height: Val::Auto,
        ..default()
    };
    debug_config.apply_to_node(&mut debug_node);
    // Adjust top position for debug panel (below HUD)
    debug_node.top = Val::Px(80.0);

    commands
        .spawn((
            DebugPanelText,
            NodeBundle {
                node: debug_node,
                background_color: debug_config.background_color.unwrap_or(Color::NONE).into(),
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Debug Panel",
                TextStyle {
                    font,
                    font_size: 12.0,
                    color: Color::srgb(0.85, 0.9, 0.95),
                },
            ));
        });
}

fn update_hud(
    view: Res<ViewMode>,
    ticks: Res<SimTickCount>,
    mut hud_text: Query<&mut Text, With<HudText>>,
) {
    if let Some(mut text) = hud_text.iter_mut().next() {
        text.0 = format!("View: {:?} | t{} | F3: Debug", *view, ticks.tick);
    }
}

fn update_log_panel(log: Res<EventLog>, mut log_text: Query<&mut Text, With<LogContentText>>) {
    if let Some(mut text) = log_text.iter_mut().next() {
        let entries = log.entries();
        if entries.is_empty() {
            text.0 = "Awaiting signal...".to_string();
        } else {
            let mut body = String::new();
            for entry in entries {
                body.push_str("› ");
                body.push_str(entry);
                body.push('\n');
            }
            text.0 = body.trim_end().to_string();
        }
    }
}

fn update_node_panel(
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut panel: Query<&mut Text, With<NodeListText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        let mut entries = nodes
            .iter()
            .filter(|(_, intel)| intel.revealed)
            .map(|(node, intel)| (node.id, intel.layer, intel.confidence, node.modifier))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.0);

        if entries.is_empty() {
            text.0 = "Nodes: --".to_string();
        } else {
            let mut body = String::from("Nodes:\n");
            for (id, layer, confidence, modifier) in entries {
                body.push_str(&format!(
                    "- {} L{} {:.0}% {} ({})\n",
                    id,
                    layer_to_short(layer),
                    confidence * 100.0,
                    modifier_to_short(modifier),
                    modifier_to_long(modifier)
                ));
            }
            text.0 = body.trim_end().to_string();
        }
    }
}

fn layer_to_short(layer: crate::world::KnowledgeLayer) -> &'static str {
    match layer {
        KnowledgeLayer::Existence => "0",
        KnowledgeLayer::Geography => "1",
        KnowledgeLayer::Resources => "2",
        KnowledgeLayer::Threats => "3",
        KnowledgeLayer::Stability => "4",
    }
}

fn modifier_to_short(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "RAD",
        Some(ZoneModifier::NebulaInterference) => "NEB",
        Some(ZoneModifier::RichOreVeins) => "ORE",
        Some(ZoneModifier::ClearSignals) => "CLR",
        None => "--",
    }
}

fn modifier_to_long(modifier: Option<ZoneModifier>) -> &'static str {
    match modifier {
        Some(ZoneModifier::HighRadiation) => "High Radiation",
        Some(ZoneModifier::NebulaInterference) => "Nebula",
        Some(ZoneModifier::RichOreVeins) => "Rich Ore",
        Some(ZoneModifier::ClearSignals) => "Clear Signals",
        None => "",
    }
}

fn update_hover_panel(
    hovered: Res<HoveredNode>,
    mut panel: Query<(&mut Text, &mut UiNode), With<HoverText>>,
) {
    if let Some((mut text, mut node)) = panel.iter_mut().next() {
        match (hovered.id, hovered.screen_pos) {
            (Some(id), Some(pos)) => {
                let layer = hovered.layer.unwrap_or(KnowledgeLayer::Existence);
                let modifier = modifier_to_short(hovered.modifier);
                let modifier_long = modifier_to_long(hovered.modifier);
                text.0 = format!(
                    "{} | L{} {:.0}% {} {}",
                    id,
                    layer_to_short(layer),
                    hovered.confidence * 100.0,
                    modifier,
                    modifier_long,
                );
                node.display = Display::Flex;
                // Reset right to auto so left takes effect
                node.right = Val::Auto;
                node.left = Val::Px(pos.x + 20.0);
                // Position slightly below cursor (screen Y is top-down for UI)
                node.top = Val::Px(pos.y + 20.0);
            }
            _ => {
                node.display = Display::None;
                // Move offscreen and clear text to prevent any flash
                node.left = Val::Px(-1000.0);
                node.top = Val::Px(-1000.0);
                text.0.clear();
            }
        }
    }
}

fn update_risk_panel(sector: Res<Sector>, mut panel: Query<&mut Text, With<RiskText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        let (route_risk, modifier_risk) = risk_breakdown(&sector);
        text.0 = format!("Risk: route {:.2} | mod {:.2}", route_risk, modifier_risk);
    }
}

fn risk_breakdown(sector: &Sector) -> (f32, f32) {
    let route_risk = if sector.routes.is_empty() {
        0.0
    } else {
        let total = sector.routes.iter().map(|route| route.risk).sum::<f32>();
        total / (sector.routes.len() as f32)
    };

    let modifier_risk = if sector.nodes.is_empty() {
        0.0
    } else {
        let total = sector
            .nodes
            .iter()
            .map(|node| {
                let effect = zone_modifier_effect(node.modifier);
                effect.fuel_risk + effect.confidence_risk + effect.pirate_risk
            })
            .sum::<f32>();
        total / (sector.nodes.len() as f32)
    };

    (route_risk, modifier_risk)
}

fn update_modifier_panel(
    sector: Res<Sector>,
    mut panel: Query<&mut Text, With<ModifierPanelText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
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

        let summary = counts
            .iter()
            .map(|(key, count)| format!("{}:{}", key, count))
            .collect::<Vec<_>>()
            .join(" ");

        text.0 = format!("Modifiers: {}", summary);
    }
}

fn update_cooldown_panel(
    ticks: Res<SimTickCount>,
    cooldown: Res<IntelRefreshCooldown>,
    mut panel: Query<&mut Text, With<CooldownText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        let remaining = cooldown.remaining_ticks(ticks.tick);
        if remaining == 0 {
            text.0 = "Intel refresh: ready".to_string();
        } else {
            text.0 = format!("Intel refresh: {}t", remaining);
        }
    }
}

fn update_player_panel(
    player: Query<(&Ship, &Cargo, &ZoneId), With<PlayerControl>>,
    mut panel: Query<&mut Text, With<PlayerPanelText>>,
) {
    if let Some(mut text) = panel.iter_mut().next() {
        match player.single() {
            Ok((ship, cargo, zone_id)) => {
                let fuel_pct = if ship.fuel_capacity > 0.0 {
                    (ship.fuel / ship.fuel_capacity) * 100.0
                } else {
                    0.0
                };
                let ore_pct = if cargo.capacity > 0.0 {
                    (cargo.common_ore / cargo.capacity) * 100.0
                } else {
                    0.0
                };
                text.0 = format!(
                    "Player: Zone {} | Fuel {:.0}% | Ore {:.0}% ({:.0}/{:.0})",
                    zone_id.0, fuel_pct, ore_pct, cargo.common_ore, cargo.capacity
                );
            }
            Err(_) => {
                text.0 = "Player: --".to_string();
            }
        }
    }
}

fn update_fleet_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    scouts: Query<&ScoutBehavior>,
    selected: Res<SelectedFleetUnit>,
    container_query: Query<Entity, With<FleetListContainer>>,
    existing_items: Query<Entity, With<FleetItem>>,
    empty_text: Query<Entity, With<FleetEmptyText>>,
) {
    use crate::fleets::ScoutPhase;
    use std::path::Path;

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);
    if !font_on_disk.exists() {
        return;
    }
    let font = asset_server.load(font_path);

    // Get the container
    let Ok(container_entity) = container_query.single() else {
        return;
    };

    // Despawn existing items and empty text
    for item_entity in existing_items.iter() {
        commands.entity(item_entity).despawn();
    }
    for empty_entity in empty_text.iter() {
        commands.entity(empty_entity).despawn();
    }

    // Collect scout data
    let scout_data: Vec<_> = scouts.iter().collect();

    // Spawn new items
    commands.entity(container_entity).with_children(|parent| {
        if scout_data.is_empty() {
            parent.spawn((
                FleetEmptyText,
                TextBundle::from_section(
                    "(no units)",
                    TextStyle {
                        font: font.clone(),
                        font_size: 12.0,
                        color: Color::srgb(0.4, 0.6, 0.6),
                    },
                ),
            ));
        } else {
            for (index, scout) in scout_data.iter().enumerate() {
                let phase_short = match scout.phase {
                    ScoutPhase::Scanning => "Scan",
                    ScoutPhase::Investigating => "Invest",
                    ScoutPhase::ZoneComplete => "Ready",
                    ScoutPhase::TravelingToGate => "Travel",
                    ScoutPhase::Jumping => "Jump",
                    ScoutPhase::Complete => "Done",
                };

                // Compact format: "Scout-1  Z12348  Scan"
                let text_content = format!(
                    "Scout-{}  Z{}  {}",
                    index + 1,
                    scout.current_zone,
                    phase_short
                );

                let is_selected = selected.index == Some(index);
                let color = contact_item_color(is_selected, false);

                parent.spawn((
                    FleetItem { index },
                    Interaction::None,
                    TextBundle::from_section(
                        text_content,
                        TextStyle {
                            font: font.clone(),
                            font_size: 12.0,
                            color,
                        },
                    ),
                ));
            }
        }
    });
}

fn handle_fleet_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    mut selected: ResMut<SelectedFleetUnit>,
    items: Query<(&Interaction, &FleetItem)>,
    panel: Query<&Interaction, With<FleetPanelMarker>>,
) {
    // Check if any fleet item was clicked
    for (interaction, fleet_item) in items.iter() {
        if matches!(interaction, Interaction::Pressed) {
            // Toggle selection: if already selected, deselect; otherwise select
            if selected.index == Some(fleet_item.index) {
                selected.index = None;
            } else {
                selected.index = Some(fleet_item.index);
            }
            return;
        }
    }

    // If mouse was just pressed and we didn't click a fleet item, check if we're outside panel
    if mouse.just_pressed(MouseButton::Left) {
        let panel_hovered = panel
            .iter()
            .any(|i| matches!(i, Interaction::Hovered | Interaction::Pressed));

        if !panel_hovered {
            selected.index = None;
        }
    }
}

fn update_fleet_item_styles(
    selected: Res<SelectedFleetUnit>,
    mut items: Query<(&Interaction, &FleetItem, &mut TextColor)>,
) {
    for (interaction, fleet_item, mut text_color) in items.iter_mut() {
        let is_selected = selected.index == Some(fleet_item.index);
        let is_hovered = matches!(interaction, Interaction::Hovered);
        text_color.0 = contact_item_color(is_selected, is_hovered);
    }
}

fn update_fleet_detail(
    scouts: Query<&ScoutBehavior>,
    selected: Res<SelectedFleetUnit>,
    mut detail_text: Query<&mut Text, With<FleetDetailText>>,
    mut divider: Query<&mut Visibility, With<FleetDetailDivider>>,
) {
    use crate::fleets::ScoutPhase;

    let mut text = match detail_text.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut divider_vis = match divider.single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };

    let Some(selected_index) = selected.index else {
        text.0 = String::new();
        *divider_vis = Visibility::Hidden;
        return;
    };

    // Find the scout at the selected index
    let scout_data: Vec<_> = scouts.iter().collect();
    let Some(scout) = scout_data.get(selected_index) else {
        text.0 = String::new();
        *divider_vis = Visibility::Hidden;
        return;
    };

    // Show the divider when we have detail to show
    *divider_vis = Visibility::Inherited;

    let risk_label = match scout.risk {
        RiskTolerance::Cautious => "Cautious",
        RiskTolerance::Balanced => "Balanced",
        RiskTolerance::Bold => "Bold",
    };

    let phase_label = match scout.phase {
        ScoutPhase::Scanning => "Scanning area",
        ScoutPhase::Investigating => "Investigating contacts",
        ScoutPhase::ZoneComplete => "Zone complete",
        ScoutPhase::TravelingToGate => "En route to gate",
        ScoutPhase::Jumping => "Jumping...",
        ScoutPhase::Complete => "Exploration complete",
    };

    let mut lines = Vec::new();
    lines.push(format!("Risk: {}", risk_label));
    lines.push(format!("Status: {}", phase_label));
    lines.push(format!("Gates queued: {}", scout.gates_to_explore.len()));
    lines.push(format!("Zones visited: {}", scout.visited_zones.len()));

    text.0 = lines.join("\n");
}

#[allow(deprecated)]
fn handle_panel_scroll(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut scrollable: Query<(&Interaction, &mut ScrollPosition)>,
) {
    for event in scroll_events.read() {
        for (interaction, mut scroll_pos) in scrollable.iter_mut() {
            if matches!(interaction, Interaction::Hovered) {
                scroll_pos.y -= event.y * 20.0;
                scroll_pos.y = scroll_pos.y.max(0.0);
            }
        }
    }
}

fn update_focus_panel(marker: Res<FocusMarker>, mut panel: Query<&mut Text, With<FocusText>>) {
    if let Some(mut text) = panel.iter_mut().next() {
        match marker.node_id() {
            Some(node_id) => {
                text.0 = format!("Focus: node {}", node_id);
            }
            None => {
                text.0 = "Focus: --".to_string();
            }
        };
    }
}

/// Formats the Intel panel content for the selected target.
pub fn format_intel_panel(info: Option<&IntelInfo>) -> String {
    let Some(info) = info else {
        return "No target selected".to_string();
    };

    let mut lines = Vec::new();
    lines.push(format!("Target: {}", info.label));
    lines.push(format!(
        "Position: ({:.0}, {:.0})",
        info.position.x, info.position.y
    ));
    lines.push(format!("Distance: {:.0}", info.distance));

    lines.join("\n")
}

/// Formats the contacts panel content from a list of targets.
/// Returns the formatted string with title, divider, and target list.
#[allow(dead_code)]
fn format_contacts_panel(
    entities: &[(bevy::prelude::Entity, Vec2, String)],
    selected_index: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push("Contacts".to_string());
    lines.push("--------".to_string());

    if entities.is_empty() {
        lines.push("(unidentified)".to_string());
    } else {
        for (index, (_, _, label)) in entities.iter().enumerate() {
            let indicator = if index == selected_index { ">" } else { " " };
            lines.push(format!("{} {}", indicator, label));
        }
    }

    lines.join("\n")
}

fn update_tactical_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    targets: Res<NearbyTargets>,
    container_query: Query<(Entity, Option<&Children>), With<ContactsListContainer>>,
    existing_items: Query<Entity, With<ContactItem>>,
) {
    // Only rebuild when targets change
    if !targets.is_changed() {
        return;
    }

    let font_path = "fonts/SpaceMono-Regular.ttf";
    let font_on_disk = Path::new("assets").join(font_path);
    if !font_on_disk.exists() {
        return;
    }
    let font = asset_server.load(font_path);

    // Get the container
    let Ok((container_entity, _)) = container_query.single() else {
        return;
    };

    // Despawn existing items
    for item_entity in existing_items.iter() {
        commands.entity(item_entity).despawn();
    }

    // Spawn new items
    commands.entity(container_entity).with_children(|parent| {
        if targets.entities.is_empty() {
            parent.spawn(TextBundle::from_section(
                "(unidentified)",
                TextStyle {
                    font: font.clone(),
                    font_size: 13.0,
                    color: Color::srgb(0.4, 0.6, 0.6),
                },
            ));
        } else {
            for (index, (_, _, label)) in targets.entities.iter().enumerate() {
                let is_selected = index == targets.selected_index;
                let indicator = if is_selected { ">" } else { " " };
                let text_content = format!("{} {}", indicator, label);
                let color = contact_item_color(is_selected, false);

                parent.spawn((
                    ContactItem { index },
                    Interaction::None,
                    TextBundle::from_section(
                        text_content,
                        TextStyle {
                            font: font.clone(),
                            font_size: 13.0,
                            color,
                        },
                    ),
                ));
            }
        }
    });
}

fn handle_contact_clicks(
    mut targets: ResMut<NearbyTargets>,
    items: Query<(&Interaction, &ContactItem), Changed<Interaction>>,
) {
    for (interaction, contact_item) in items.iter() {
        if matches!(interaction, Interaction::Pressed) {
            targets.selected_index = contact_item.index;
            targets.manually_selected = true;
        }
    }
}

fn update_contact_item_styles(
    targets: Res<NearbyTargets>,
    mut items: Query<(&Interaction, &ContactItem, &mut TextColor)>,
) {
    for (interaction, contact_item, mut text_color) in items.iter_mut() {
        let is_selected = contact_item.index == targets.selected_index;
        let is_hovered = matches!(interaction, Interaction::Hovered);
        text_color.0 = contact_item_color(is_selected, is_hovered);
    }
}

fn update_intel_panel(
    targets: Res<NearbyTargets>,
    player_query: Query<&Transform, With<PlayerControl>>,
    mut intel_text: Query<&mut Text, With<IntelContentText>>,
) {
    let mut text = match intel_text.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    // Get player position for distance calculation
    let player_pos = match player_query.single() {
        Ok(transform) => Vec2::new(transform.translation.x, transform.translation.y),
        Err(_) => {
            text.0 = "No target selected".to_string();
            return;
        }
    };

    // Get selected entity info
    let Some((_, pos, label)) = targets.entities.get(targets.selected_index) else {
        text.0 = "No target selected".to_string();
        return;
    };

    if !targets.manually_selected {
        text.0 = "No target selected".to_string();
        return;
    }

    let distance = player_pos.distance(*pos);
    let info = IntelInfo {
        entity: Entity::PLACEHOLDER,
        label: label.clone(),
        position: *pos,
        distance,
    };

    text.0 = format_intel_panel(Some(&info));
}

#[allow(clippy::too_many_arguments)]
fn update_debug_panel(
    debug_window: Res<DebugWindow>,
    seed: Res<WorldSeed>,
    ticks: Res<SimTickCount>,
    config: Res<SimConfig>,
    toggles: Res<RenderToggles>,
    zoom: Res<MapZoomOverride>,
    cooldown: Res<IntelRefreshCooldown>,
    marker: Res<FocusMarker>,
    state: Res<State<GameState>>,
    stations: Query<&Station>,
    ships: Query<&Ship>,
    scouts: Query<&ScoutBehavior>,
    nodes: Query<(&SystemNode, &SystemIntel)>,
    mut panel_container: Query<(&mut Visibility, &Children), With<DebugPanelText>>,
    mut text_query: Query<&mut Text>,
) {
    if let Ok((mut visibility, children)) = panel_container.single_mut() {
        if debug_window.open {
            *visibility = Visibility::Visible;

            // Update the text in the child
            for child in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child) {
                    let mut body = String::from("=== DEBUG PANEL (F3 to close) ===\n\n");

                    body.push_str(&format!("Seed: {} | Tick: {}\n", seed.value, ticks.tick));
                    body.push_str(&format!(
                        "Rate: {:.0} Hz | Paused: {}\n",
                        config.tick_hz, config.paused
                    ));
                    body.push_str(&format!("State: {:?}\n\n", state.get()));

                    body.push_str("Render Toggles:\n");
                    body.push_str(&format!(
                        "  Rings: {} | Grid: {}\n",
                        if toggles.rings_enabled() { "On" } else { "Off" },
                        if toggles.grid_enabled() { "On" } else { "Off" }
                    ));
                    body.push_str(&format!(
                        "  Route Labels: {} | Node Labels: {}\n",
                        if toggles.route_labels_enabled() {
                            "On"
                        } else {
                            "Off"
                        },
                        if toggles.node_labels_enabled() {
                            "On"
                        } else {
                            "Off"
                        }
                    ));
                    body.push_str(&format!("  Zoom: {}\n\n", zoom.label()));

                    body.push_str(&format!("Stations: {}\n", stations.iter().count()));
                    body.push_str(&format!("Ships: {}\n", ships.iter().count()));
                    body.push_str(&format!("Scouts: {}\n\n", scouts.iter().count()));

                    body.push_str(&format!(
                        "Intel Refresh CD: {} ticks\n",
                        cooldown.remaining_ticks(ticks.tick)
                    ));

                    match marker.node_id() {
                        Some(node_id) => {
                            body.push_str(&format!("Focus: node {}\n", node_id));
                        }
                        None => {
                            body.push_str("Focus: --\n");
                        }
                    }

                    let revealed_count = nodes.iter().filter(|(_, intel)| intel.revealed).count();
                    body.push_str(&format!(
                        "\nNodes: {} revealed / {} total\n",
                        revealed_count,
                        nodes.iter().count()
                    ));

                    body.push_str("\nDebug Keybinds:\n");
                    body.push_str("  -/= : change seed\n");
                    body.push_str("  V   : reveal adjacent\n");
                    body.push_str("  U   : reveal all\n");
                    body.push_str("  Z   : clear reveals\n");
                    body.push_str("  B   : spawn station\n");
                    body.push_str("  S   : spawn scout\n");
                    body.push_str("  P   : spawn pirate\n");
                    body.push_str("  I   : refresh intel\n");
                    body.push_str("  O   : advance intel\n");
                    body.push_str("  K   : randomize modifiers\n");

                    text.0 = body;
                    break;
                }
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn view_is_map(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::Map)
}

fn view_is_world(view: Res<ViewMode>) -> bool {
    matches!(*view, ViewMode::World)
}

fn sync_map_ui_visibility(
    view: Res<ViewMode>,
    debug_window: Res<DebugWindow>,
    mut elements: Query<(&mut UiNode, Option<&MapUi>, Option<&WorldUi>)>,
) {
    let display = if matches!(*view, ViewMode::Map) && !debug_window.open {
        Display::Flex
    } else {
        Display::None
    };

    let world_display = if matches!(*view, ViewMode::World) {
        Display::Flex
    } else {
        Display::None
    };

    for (mut node, map_ui, world_ui) in elements.iter_mut() {
        if map_ui.is_some() {
            node.display = display;
        }
        if world_ui.is_some() {
            node.display = world_display;
        }
    }
}

fn sync_map_grid_visibility(
    view: Res<ViewMode>,
    toggles: Res<RenderToggles>,
    debug_window: Res<DebugWindow>,
    mut roots: Query<&mut UiNode, With<MapGridRoot>>,
) {
    let show = matches!(*view, ViewMode::Map) && toggles.grid_enabled() && !debug_window.open;
    let display = if show { Display::Flex } else { Display::None };

    for mut node in roots.iter_mut() {
        node.display = display;
    }
}

#[cfg(test)]
mod tests {
    use super::{contact_item_color, format_contacts_panel, PanelConfig, PanelPosition};
    use bevy::ecs::entity::Entity;
    use bevy::prelude::{Color, Vec2};
    use bevy::ui::{Node as UiNode, PositionType, Val};

    #[test]
    fn panel_config_default_values() {
        let config = PanelConfig::default();
        assert_eq!(config.position, PanelPosition::TopLeft);
        assert_eq!(config.margin, 14.0);
        assert!(config.background_color.is_none());
        assert!(config.border_color.is_none());
        assert_eq!(config.border_width, 0.0);
        assert_eq!(config.padding, 0.0);
        assert!(config.title.is_none());
        assert!(config.width.is_none());
        assert!(config.height.is_none());
        assert!(!config.overflow_scroll);
    }

    #[test]
    fn panel_config_with_size() {
        let config = PanelConfig::at(PanelPosition::TopLeft).with_size(150.0, 200.0);
        assert_eq!(config.width, Some(150.0));
        assert_eq!(config.height, Some(200.0));
    }

    #[test]
    fn panel_config_with_scroll() {
        let config = PanelConfig::at(PanelPosition::TopLeft).with_scroll();
        assert!(config.overflow_scroll);
    }

    #[test]
    fn panel_config_apply_size_to_node() {
        let config = PanelConfig::at(PanelPosition::TopLeft).with_size(150.0, 200.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.width, Val::Px(150.0));
        assert_eq!(node.height, Val::Px(200.0));
    }

    #[test]
    fn panel_config_apply_scroll_to_node() {
        use bevy::ui::OverflowAxis;
        let config = PanelConfig::at(PanelPosition::TopLeft)
            .with_size(150.0, 200.0)
            .with_scroll();
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.overflow.y, OverflowAxis::Scroll);
    }

    #[test]
    fn panel_config_with_border() {
        let border_color = Color::srgb(0.5, 0.6, 0.7);
        let config = PanelConfig::at(PanelPosition::TopLeft).with_border(border_color, 2.0);

        assert!(config.border_color.is_some());
        assert_eq!(config.border_width, 2.0);
    }

    #[test]
    fn panel_config_with_title() {
        let config = PanelConfig::at(PanelPosition::TopLeft).with_title("My Panel");

        assert_eq!(config.title, Some("My Panel".to_string()));
    }

    #[test]
    fn panel_config_apply_border_to_node() {
        let border_color = Color::srgb(0.5, 0.6, 0.7);
        let config = PanelConfig::at(PanelPosition::TopLeft).with_border(border_color, 3.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.border.left, Val::Px(3.0));
        assert_eq!(node.border.right, Val::Px(3.0));
        assert_eq!(node.border.top, Val::Px(3.0));
        assert_eq!(node.border.bottom, Val::Px(3.0));
    }

    #[test]
    fn panel_config_at_creates_with_position() {
        let config = PanelConfig::at(PanelPosition::BottomRight);
        assert_eq!(config.position, PanelPosition::BottomRight);
        assert_eq!(config.margin, 14.0); // Default margin preserved
    }

    #[test]
    fn panel_config_builder_chain() {
        let config = PanelConfig::at(PanelPosition::TopRight)
            .with_margin(20.0)
            .with_padding(10.0);

        assert_eq!(config.position, PanelPosition::TopRight);
        assert_eq!(config.margin, 20.0);
        assert_eq!(config.padding, 10.0);
    }

    #[test]
    fn panel_config_apply_top_left() {
        let config = PanelConfig::at(PanelPosition::TopLeft).with_margin(14.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.left, Val::Px(14.0));
        assert_eq!(node.top, Val::Px(14.0));
    }

    #[test]
    fn panel_config_apply_top_right() {
        let config = PanelConfig::at(PanelPosition::TopRight).with_margin(14.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.right, Val::Px(14.0));
        assert_eq!(node.top, Val::Px(14.0));
    }

    #[test]
    fn panel_config_apply_bottom_left() {
        let config = PanelConfig::at(PanelPosition::BottomLeft).with_margin(14.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.left, Val::Px(14.0));
        assert_eq!(node.bottom, Val::Px(14.0));
    }

    #[test]
    fn panel_config_apply_bottom_right() {
        let config = PanelConfig::at(PanelPosition::BottomRight).with_margin(14.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.right, Val::Px(14.0));
        assert_eq!(node.bottom, Val::Px(14.0));
    }

    #[test]
    fn panel_config_apply_with_padding() {
        let config = PanelConfig::at(PanelPosition::TopLeft).with_padding(10.0);
        let mut node = UiNode::default();
        config.apply_to_node(&mut node);

        assert_eq!(node.padding.left, Val::Px(10.0));
        assert_eq!(node.padding.right, Val::Px(10.0));
        assert_eq!(node.padding.top, Val::Px(10.0));
        assert_eq!(node.padding.bottom, Val::Px(10.0));
    }

    #[test]
    fn contacts_panel_empty_shows_unidentified() {
        let entities: Vec<(Entity, Vec2, String)> = vec![];
        let result = format_contacts_panel(&entities, 0);

        assert!(result.contains("Contacts"));
        assert!(result.contains("--------"));
        assert!(result.contains("(unidentified)"));
    }

    #[test]
    fn contacts_panel_single_target_shows_selection() {
        let entity = Entity::from_bits(42);
        let entities = vec![(entity, Vec2::new(10.0, 20.0), "Ore Node".to_string())];
        let result = format_contacts_panel(&entities, 0);

        assert!(result.contains("Contacts"));
        assert!(result.contains("> Ore Node"));
    }

    #[test]
    fn contacts_panel_multiple_targets_shows_all() {
        let entities = vec![
            (
                Entity::from_bits(1),
                Vec2::new(10.0, 10.0),
                "Ore Node".to_string(),
            ),
            (
                Entity::from_bits(2),
                Vec2::new(20.0, 20.0),
                "Station-1".to_string(),
            ),
            (
                Entity::from_bits(3),
                Vec2::new(30.0, 30.0),
                "Pirate".to_string(),
            ),
        ];
        let result = format_contacts_panel(&entities, 1);

        assert!(result.contains("Contacts"));
        assert!(result.contains("  Ore Node")); // Not selected (space prefix)
        assert!(result.contains("> Station-1")); // Selected (> prefix)
        assert!(result.contains("  Pirate")); // Not selected
    }

    #[test]
    fn contacts_panel_selection_indicator_on_first() {
        let entities = vec![
            (
                Entity::from_bits(1),
                Vec2::new(10.0, 10.0),
                "Alpha".to_string(),
            ),
            (
                Entity::from_bits(2),
                Vec2::new(20.0, 20.0),
                "Beta".to_string(),
            ),
        ];
        let result = format_contacts_panel(&entities, 0);

        assert!(result.contains("> Alpha"));
        assert!(result.contains("  Beta"));
    }

    #[test]
    fn contacts_panel_selection_indicator_on_last() {
        let entities = vec![
            (
                Entity::from_bits(1),
                Vec2::new(10.0, 10.0),
                "Alpha".to_string(),
            ),
            (
                Entity::from_bits(2),
                Vec2::new(20.0, 20.0),
                "Beta".to_string(),
            ),
        ];
        let result = format_contacts_panel(&entities, 1);

        assert!(result.contains("  Alpha"));
        assert!(result.contains("> Beta"));
    }

    #[test]
    fn contact_item_color_default_when_not_selected_or_hovered() {
        let is_selected = false;
        let is_hovered = false;
        let color = contact_item_color(is_selected, is_hovered);
        // Default cyan color
        assert!(color.to_srgba().red < 0.1);
        assert!(color.to_srgba().green > 0.9);
        assert!(color.to_srgba().blue > 0.9);
    }

    #[test]
    fn contact_item_color_highlight_when_selected() {
        let is_selected = true;
        let is_hovered = false;
        let color = contact_item_color(is_selected, is_hovered);
        // Selected = brighter/white
        assert!(color.to_srgba().green > 0.9);
    }

    #[test]
    fn contact_item_color_hover_when_hovered_not_selected() {
        let is_selected = false;
        let is_hovered = true;
        let color = contact_item_color(is_selected, is_hovered);
        // Hovered = slightly brighter
        assert!(color.to_srgba().green > 0.7);
    }

    #[test]
    fn contact_item_contains_index() {
        let item = super::ContactItem { index: 5 };
        assert_eq!(item.index, 5);
    }

    #[test]
    fn format_intel_empty_when_no_selection() {
        let result = super::format_intel_panel(None);
        assert!(result.contains("No target"));
    }

    #[test]
    fn format_intel_shows_entity_details() {
        let entity = Entity::from_bits(42);
        let info = super::IntelInfo {
            entity,
            label: "Ore Node".to_string(),
            position: Vec2::new(100.0, 200.0),
            distance: 150.0,
        };
        let result = super::format_intel_panel(Some(&info));

        assert!(result.contains("Ore Node"));
        assert!(result.contains("100")); // X position
        assert!(result.contains("200")); // Y position
        assert!(result.contains("150")); // Distance
    }
}
