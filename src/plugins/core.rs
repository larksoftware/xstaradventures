use bevy::prelude::*;

use crate::compat::SpriteBundle;
use std::time::Duration;

pub struct CorePlugin;

#[derive(Resource, Debug, Clone)]
pub struct SimConfig {
    pub tick_hz: f32,
    pub paused: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            tick_hz: 10.0,
            paused: false,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct FogConfig {
    pub decay_existence: f32,
    pub decay_geography: f32,
    pub decay_resources: f32,
    pub decay_threats: f32,
    pub decay_stability: f32,
    pub floor_existence: f32,
    pub floor_geography: f32,
    pub floor_resources: f32,
    pub floor_threats: f32,
    pub floor_stability: f32,
}

impl Default for FogConfig {
    fn default() -> Self {
        Self {
            decay_existence: 0.0005,
            decay_geography: 0.001,
            decay_resources: 0.0015,
            decay_threats: 0.002,
            decay_stability: 0.0025,
            floor_existence: 0.25,
            floor_geography: 0.2,
            floor_resources: 0.15,
            floor_threats: 0.12,
            floor_stability: 0.1,
        }
    }
}

#[derive(Resource, Debug)]
pub struct EventLog {
    entries: Vec<String>,
    max_entries: usize,
}

impl Default for EventLog {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 8,
        }
    }
}

impl EventLog {
    pub fn push(&mut self, entry: String) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            let overflow = self.entries.len() - self.max_entries;
            self.entries.drain(0..overflow);
        }
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    World,
    Map,
}

#[derive(Resource, Debug, Default)]
pub struct DebugWindow {
    pub open: bool,
}

#[derive(Resource, Debug, Default)]
pub struct RunTimer {
    pub elapsed_seconds: f32,
}

impl RunTimer {
    #[allow(dead_code)]
    pub fn minutes(&self) -> u32 {
        (self.elapsed_seconds / 60.0) as u32
    }

    #[allow(dead_code)]
    pub fn seconds(&self) -> u32 {
        (self.elapsed_seconds % 60.0) as u32
    }
}

#[derive(Resource, Debug, Clone)]
pub struct InputBindings {
    pub move_up: KeyCode,
    pub move_down: KeyCode,
    pub rotate_left: KeyCode,
    pub rotate_right: KeyCode,
    pub brake: KeyCode,
    pub interact: KeyCode,
    pub toggle_debug: KeyCode,
    pub scout_risk_down: KeyCode,
    pub scout_risk_up: KeyCode,
    pub pause: KeyCode,
    pub rate_up: KeyCode,
    pub rate_down: KeyCode,
    pub save: KeyCode,
    pub load: KeyCode,
    pub seed_up: KeyCode,
    pub seed_down: KeyCode,
    pub toggle_nodes: KeyCode,
    pub toggle_routes: KeyCode,
    pub toggle_rings: KeyCode,
    pub toggle_grid: KeyCode,
    pub toggle_route_labels: KeyCode,
    pub toggle_node_labels: KeyCode,
    pub refresh_intel: KeyCode,
    pub advance_intel: KeyCode,
    pub randomize_modifiers: KeyCode,
    pub toggle_map: KeyCode,
    pub reveal_adjacent: KeyCode,
    pub spawn_station: KeyCode,
    pub spawn_ship: KeyCode,
    pub spawn_pirate: KeyCode,
    pub reveal_all: KeyCode,
    pub clear_reveal: KeyCode,
    pub center_camera: KeyCode,
    pub cycle_target: KeyCode,
    pub navigate: KeyCode,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            move_up: KeyCode::KeyW,
            move_down: KeyCode::KeyS,
            rotate_left: KeyCode::KeyA,
            rotate_right: KeyCode::KeyD,
            brake: KeyCode::Space,
            interact: KeyCode::KeyJ,
            toggle_debug: KeyCode::F3,
            scout_risk_down: KeyCode::Comma,
            scout_risk_up: KeyCode::Period,
            pause: KeyCode::Escape,
            rate_up: KeyCode::BracketRight,
            rate_down: KeyCode::BracketLeft,
            save: KeyCode::F5,
            load: KeyCode::F9,
            seed_up: KeyCode::Equal,
            seed_down: KeyCode::Minus,
            toggle_nodes: KeyCode::KeyN,
            toggle_routes: KeyCode::KeyR,
            toggle_rings: KeyCode::KeyF,
            toggle_grid: KeyCode::KeyG,
            toggle_route_labels: KeyCode::KeyT,
            toggle_node_labels: KeyCode::KeyY,
            refresh_intel: KeyCode::KeyI,
            advance_intel: KeyCode::KeyO,
            randomize_modifiers: KeyCode::KeyK,
            toggle_map: KeyCode::KeyM,
            reveal_adjacent: KeyCode::KeyV,
            spawn_station: KeyCode::KeyB,
            spawn_ship: KeyCode::KeyS,
            spawn_pirate: KeyCode::KeyP,
            reveal_all: KeyCode::KeyU,
            clear_reveal: KeyCode::KeyZ,
            center_camera: KeyCode::KeyH,
            cycle_target: KeyCode::Tab,
            navigate: KeyCode::KeyN,
        }
    }
}

#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Boot,
    Loading,
    InGame,
}

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        let config = SimConfig::default();
        let fixed_time = fixed_time_from_config(&config);
        let bindings = InputBindings::default();
        let fog_config = FogConfig::default();

        app.init_state::<GameState>()
            .insert_resource(config)
            .insert_resource(fixed_time)
            .insert_resource(bindings)
            .insert_resource(fog_config)
            .init_resource::<ViewMode>()
            .init_resource::<EventLog>()
            .init_resource::<RunTimer>()
            .init_resource::<DebugWindow>()
            .add_systems(OnEnter(GameState::Boot), log_enter_boot)
            .add_systems(OnEnter(GameState::Boot), transition_to_loading)
            .add_systems(OnEnter(GameState::Loading), setup_loading_screen)
            .add_systems(OnExit(GameState::Loading), teardown_loading_screen)
            .add_systems(OnEnter(GameState::InGame), log_enter_ingame)
            .add_systems(OnExit(GameState::InGame), log_exit_ingame)
            .add_systems(
                Update,
                (
                    handle_pause_toggle,
                    handle_tick_rate_input,
                    handle_view_toggle,
                    handle_debug_toggle,
                    update_run_timer.run_if(in_state(GameState::InGame)),
                ),
            )
            .add_systems(Update, tick_loading.run_if(in_state(GameState::Loading)));
    }
}

fn log_enter_boot(mut log: ResMut<EventLog>) {
    log.push("State: Boot".to_string());
    info!("State: Boot");
}

fn transition_to_loading(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Loading);
}

fn log_enter_ingame(mut log: ResMut<EventLog>) {
    log.push("State: InGame".to_string());
    info!("State: InGame");
}

fn log_exit_ingame(mut log: ResMut<EventLog>) {
    log.push("State: leaving InGame".to_string());
    info!("State: leaving InGame");
}

#[derive(Component)]
struct LoadingScreen;

#[derive(Resource)]
struct LoadingTimer {
    timer: Timer,
}

fn setup_loading_screen(mut commands: Commands) {
    let size = Vec2::new(4000.0, 2250.0);

    commands.spawn((
        LoadingScreen,
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.02, 0.02, 0.04),
                custom_size: Some(size),
                ..default()
            },
            ..default()
        },
    ));

    commands.insert_resource(LoadingTimer {
        timer: Timer::from_seconds(0.35, TimerMode::Once),
    });
}

fn tick_loading(
    time: Res<Time>,
    mut timer: ResMut<LoadingTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    timer.timer.tick(time.delta());

    if timer.timer.is_finished() {
        next_state.set(GameState::InGame);
    }
}

fn teardown_loading_screen(mut commands: Commands, screens: Query<Entity, With<LoadingScreen>>) {
    for entity in screens.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LoadingTimer>();
}

fn handle_pause_toggle(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut config: ResMut<SimConfig>,
) {
    if input.just_pressed(bindings.pause) {
        config.paused = !config.paused;
        info!("Sim paused: {}", config.paused);
    }
}

fn handle_tick_rate_input(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut config: ResMut<SimConfig>,
    mut fixed_time: ResMut<Time<Fixed>>,
) {
    let mut updated = false;

    if input.just_pressed(bindings.rate_up) {
        config.tick_hz = (config.tick_hz + 1.0).min(60.0);
        updated = true;
    }

    if input.just_pressed(bindings.rate_down) {
        config.tick_hz = (config.tick_hz - 1.0).max(1.0);
        updated = true;
    }

    if updated {
        *fixed_time = fixed_time_from_config(&config);
        info!("Sim tick rate: {} Hz", config.tick_hz);
    }
}

fn handle_view_toggle(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut view: ResMut<ViewMode>,
) {
    if input.just_pressed(bindings.toggle_map) {
        *view = match *view {
            ViewMode::World => ViewMode::Map,
            ViewMode::Map => ViewMode::World,
        };
    }
}

fn handle_debug_toggle(
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut debug_window: ResMut<DebugWindow>,
) {
    if input.just_pressed(bindings.toggle_debug) {
        debug_window.open = !debug_window.open;
        info!(
            "Debug window: {}",
            if debug_window.open { "open" } else { "closed" }
        );
    }
}

fn update_run_timer(time: Res<Time>, mut timer: ResMut<RunTimer>) {
    timer.elapsed_seconds += time.delta_secs();
}

fn fixed_time_from_config(config: &SimConfig) -> Time<Fixed> {
    let tick_hz = if config.tick_hz <= 0.0 {
        10.0
    } else {
        config.tick_hz
    };
    let seconds = 1.0 / tick_hz;
    Time::<Fixed>::from_duration(Duration::from_secs_f32(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[test]
    fn sim_config_default_values() {
        let config = SimConfig::default();
        assert_eq!(config.tick_hz, 10.0);
        assert!(!config.paused);
    }

    #[test]
    fn event_log_push_trims_oldest_entries() {
        let mut log = EventLog::default();
        for index in 0..12 {
            log.push(format!("entry-{}", index));
        }

        let entries = log.entries();
        assert_eq!(entries.len(), 8);
        assert_eq!(entries.first().map(String::as_str), Some("entry-4"));
        assert_eq!(entries.last().map(String::as_str), Some("entry-11"));
    }

    #[test]
    fn fixed_time_from_config_clamps_non_positive_tick_rate() {
        let config = SimConfig {
            tick_hz: 0.0,
            paused: false,
        };
        let fixed = fixed_time_from_config(&config);
        assert_eq!(fixed.timestep().as_secs_f32(), 0.1);
    }

    #[test]
    fn handle_view_toggle_updates_view() {
        let mut world = World::default();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(InputBindings::default());
        world.insert_resource(ViewMode::World);

        {
            let mut input = world.resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::KeyM);
        }

        let mut system_state: SystemState<(
            Res<ButtonInput<KeyCode>>,
            Res<InputBindings>,
            ResMut<ViewMode>,
        )> = SystemState::new(&mut world);
        let (input, bindings, view) = system_state.get_mut(&mut world);
        handle_view_toggle(input, bindings, view);
        system_state.apply(&mut world);

        let view = world.resource::<ViewMode>();
        assert_eq!(*view, ViewMode::Map);
    }

    #[test]
    fn handle_pause_toggle_flips_config() {
        let mut world = World::default();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(InputBindings::default());
        world.insert_resource(SimConfig::default());

        {
            let mut input = world.resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::Escape);
        }

        let mut system_state: SystemState<(
            Res<ButtonInput<KeyCode>>,
            Res<InputBindings>,
            ResMut<SimConfig>,
        )> = SystemState::new(&mut world);
        let (input, bindings, config) = system_state.get_mut(&mut world);
        handle_pause_toggle(input, bindings, config);
        system_state.apply(&mut world);

        let config = world.resource::<SimConfig>();
        assert!(config.paused);
    }

    #[test]
    fn handle_tick_rate_input_clamps_and_updates_fixed_time() {
        let mut world = World::default();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(InputBindings::default());
        world.insert_resource(SimConfig {
            tick_hz: 1.0,
            paused: false,
        });
        world.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(0.1)));

        {
            let mut input = world.resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::BracketLeft);
        }

        let mut system_state: SystemState<(
            Res<ButtonInput<KeyCode>>,
            Res<InputBindings>,
            ResMut<SimConfig>,
            ResMut<Time<Fixed>>,
        )> = SystemState::new(&mut world);
        let (input, bindings, config, fixed_time) = system_state.get_mut(&mut world);
        handle_tick_rate_input(input, bindings, config, fixed_time);
        system_state.apply(&mut world);

        let config = world.resource::<SimConfig>();
        assert_eq!(config.tick_hz, 1.0);
        let fixed_time = world.resource::<Time<Fixed>>();
        assert_eq!(fixed_time.timestep().as_secs_f32(), 1.0);
    }

    #[test]
    fn spawn_pirate_binding_is_key_p() {
        let bindings = InputBindings::default();
        assert_eq!(bindings.spawn_pirate, KeyCode::KeyP);
    }

    #[test]
    fn spawn_ship_binding_is_key_s() {
        let bindings = InputBindings::default();
        assert_eq!(bindings.spawn_ship, KeyCode::KeyS);
    }
}

#[test]
fn run_timer_tracks_elapsed_time() {
    let timer = RunTimer {
        elapsed_seconds: 125.0,
    };
    assert_eq!(timer.minutes(), 2);
    assert_eq!(timer.seconds(), 5);
}

#[test]
fn run_timer_default_is_zero() {
    let timer = RunTimer::default();
    assert_eq!(timer.elapsed_seconds, 0.0);
    assert_eq!(timer.minutes(), 0);
    assert_eq!(timer.seconds(), 0);
}
