use bevy::prelude::*;

mod compat;
mod factions;
mod fleets;
mod ore;
mod pirates;
mod plugins;
mod ships;
mod stations;
mod world;

const DEFAULT_SEED: u64 = 12345;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed = match parse_seed_from_args(&args) {
        Some(value) => value,
        None => DEFAULT_SEED,
    };

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.07, 0.1)))
        .insert_resource(plugins::worldgen::WorldSeed { value: seed })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "X Star Adventures".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            plugins::core::CorePlugin,
            plugins::worldgen::WorldGenPlugin,
            plugins::player::PlayerPlugin,
            plugins::sim::SimPlugin,
            plugins::orders::OrdersPlugin,
            plugins::ui::UIPlugin,
            plugins::render2d::Render2DPlugin,
            plugins::saveload::SaveLoadPlugin,
        ))
        .run();
}

fn parse_seed_from_args(args: &[String]) -> Option<u64> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--seed" {
            if let Some(value) = args.get(index + 1) {
                match value.parse::<u64>() {
                    Ok(parsed) => {
                        return Some(parsed);
                    }
                    Err(_) => {
                        eprintln!("Invalid seed value: {}", value);
                        return None;
                    }
                }
            } else {
                eprintln!("Missing seed value after --seed");
                return None;
            }
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{parse_seed_from_args, DEFAULT_SEED};

    #[test]
    fn parse_seed_from_args_reads_value() {
        let args = vec!["game".to_string(), "--seed".to_string(), "4242".to_string()];
        let seed = parse_seed_from_args(&args);
        assert_eq!(seed, Some(4242));
    }

    #[test]
    fn parse_seed_from_args_returns_none_for_missing_value() {
        let args = vec!["game".to_string(), "--seed".to_string()];
        let seed = parse_seed_from_args(&args);
        assert_eq!(seed, None);
    }

    #[test]
    fn parse_seed_from_args_returns_none_for_invalid_value() {
        let args = vec!["game".to_string(), "--seed".to_string(), "nope".to_string()];
        let seed = parse_seed_from_args(&args);
        assert_eq!(seed, None);
    }

    #[test]
    fn default_seed_matches_slice_requirement() {
        assert_eq!(DEFAULT_SEED, 12345);
    }
}
