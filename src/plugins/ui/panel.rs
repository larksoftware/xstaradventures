//! Panel configuration and builder for UI panels.

use bevy::prelude::*;
use bevy::ui::{Node as UiNode, OverflowAxis, PositionType, UiRect, Val};

// =============================================================================
// Panel Position
// =============================================================================

/// Position anchor for panels on the screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PanelPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

// =============================================================================
// Panel Configuration
// =============================================================================

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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
