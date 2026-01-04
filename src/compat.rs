use bevy::camera::{Camera, Camera2d, OrthographicProjection, Projection};
use bevy::color::Color;
use bevy::prelude::{
    Bundle, GlobalTransform, Handle, InheritedVisibility, Text as UiText, Transform,
    ViewVisibility, Visibility, ZIndex,
};
use bevy::render::sync_world::SyncToRenderWorld;
use bevy::sprite::{Anchor, Sprite, Text2d};
use bevy::text::{Font, TextBounds, TextColor, TextFont, TextLayout};
use bevy::ui::widget::TextNodeFlags;
use bevy::ui::{BackgroundColor, BorderColor, ContentSize, FocusPolicy, Node as UiNode};

#[derive(Clone)]
pub struct TextStyle {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 20.0,
            color: Color::WHITE,
        }
    }
}

#[derive(Bundle, Default)]
pub struct TextBundle {
    pub node: UiNode,
    pub text: UiText,
    pub text_layout: TextLayout,
    pub text_font: TextFont,
    pub text_color: TextColor,
    pub text_node_flags: TextNodeFlags,
    pub content_size: ContentSize,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
    pub background_color: BackgroundColor,
}

impl TextBundle {
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: UiText::from(value.into()),
            text_font: TextFont {
                font: style.font.clone(),
                font_size: style.font_size,
                ..Default::default()
            },
            text_color: TextColor(style.color),
            ..Default::default()
        }
    }

    pub fn with_node(mut self, node: UiNode) -> Self {
        self.node = node;
        self
    }

    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = BackgroundColor(color);
        self
    }

    pub fn with_z_index(mut self, z_index: ZIndex) -> Self {
        self.z_index = z_index;
        self
    }
}

#[derive(Bundle, Default)]
pub struct NodeBundle {
    pub node: UiNode,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
}

#[derive(Bundle, Default)]
pub struct Text2dBundle {
    pub text: Text2d,
    pub text_font: TextFont,
    pub text_color: TextColor,
    pub text_layout: TextLayout,
    pub text_bounds: TextBounds,
    pub anchor: Anchor,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub sync: SyncToRenderWorld,
}

impl Text2dBundle {
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: Text2d::new(value.into()),
            text_font: TextFont {
                font: style.font.clone(),
                font_size: style.font_size,
                ..Default::default()
            },
            text_color: TextColor(style.color),
            ..Default::default()
        }
    }
}

#[derive(Bundle, Default)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub sync: SyncToRenderWorld,
}

#[derive(Bundle, Default)]
pub struct SpatialBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub sync: SyncToRenderWorld,
}

impl SpatialBundle {
    pub fn from_transform(transform: Transform) -> Self {
        Self {
            transform,
            global_transform: GlobalTransform::from(transform),
            ..Default::default()
        }
    }
}

#[derive(Bundle)]
pub struct Camera2dBundle {
    pub camera: Camera,
    pub projection: Projection,
    pub camera_2d: Camera2d,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub sync: SyncToRenderWorld,
}

impl Default for Camera2dBundle {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            projection: Projection::Orthographic(OrthographicProjection::default_2d()),
            camera_2d: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            sync: SyncToRenderWorld,
        }
    }
}
