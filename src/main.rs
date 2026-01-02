use bevy::prelude::*;

mod plugins;
mod ships;
mod stations;
mod world;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.05, 0.07, 0.1)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "XStar Adventures".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            plugins::core::CorePlugin,
            plugins::worldgen::WorldGenPlugin,
            plugins::sim::SimPlugin,
            plugins::orders::OrdersPlugin,
            plugins::ui::UIPlugin,
            plugins::render2d::Render2DPlugin,
            plugins::saveload::SaveLoadPlugin,
        ))
        .run();
}
