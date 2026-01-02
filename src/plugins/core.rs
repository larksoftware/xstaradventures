use bevy::prelude::*;
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

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ViewMode {
    World,
    Map,
}

impl Default for ViewMode {
    fn default() -> Self {
        Self::World
    }
}

#[derive(Resource, Debug, Clone)]
pub struct InputBindings {
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
    pub toggle_backdrop: KeyCode,
    pub toggle_route_labels: KeyCode,
    pub refresh_intel: KeyCode,
    pub advance_intel: KeyCode,
    pub randomize_modifiers: KeyCode,
    pub toggle_map: KeyCode,
    pub reveal_adjacent: KeyCode,
    pub spawn_station: KeyCode,
    pub spawn_ship: KeyCode,
    pub reveal_all: KeyCode,
    pub clear_reveal: KeyCode,
    pub map_zoom: KeyCode,
    pub center_camera: KeyCode,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            pause: KeyCode::Space,
            rate_up: KeyCode::BracketRight,
            rate_down: KeyCode::BracketLeft,
            save: KeyCode::KeyS,
            load: KeyCode::KeyL,
            seed_up: KeyCode::Equal,
            seed_down: KeyCode::Minus,
            toggle_nodes: KeyCode::KeyN,
            toggle_routes: KeyCode::KeyR,
            toggle_rings: KeyCode::KeyF,
            toggle_grid: KeyCode::KeyG,
            toggle_backdrop: KeyCode::KeyP,
            toggle_route_labels: KeyCode::KeyT,
            refresh_intel: KeyCode::KeyI,
            advance_intel: KeyCode::KeyO,
            randomize_modifiers: KeyCode::KeyK,
            toggle_map: KeyCode::KeyM,
            reveal_adjacent: KeyCode::KeyV,
            spawn_station: KeyCode::KeyB,
            spawn_ship: KeyCode::KeyJ,
            reveal_all: KeyCode::KeyA,
            clear_reveal: KeyCode::KeyZ,
            map_zoom: KeyCode::KeyC,
            center_camera: KeyCode::KeyH,
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
            .add_systems(OnEnter(GameState::Boot), log_enter_boot)
            .add_systems(OnEnter(GameState::Boot), transition_to_loading)
            .add_systems(OnEnter(GameState::Loading), setup_loading_screen)
            .add_systems(OnExit(GameState::Loading), teardown_loading_screen)
            .add_systems(OnEnter(GameState::InGame), log_enter_ingame)
            .add_systems(OnExit(GameState::InGame), log_exit_ingame)
            .add_systems(
                Update,
                (handle_pause_toggle, handle_tick_rate_input, handle_view_toggle),
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
                color: Color::rgb(0.02, 0.02, 0.04),
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

    if timer.timer.finished() {
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
    mut log: ResMut<EventLog>,
) {
    if input.just_pressed(bindings.toggle_map) {
        *view = match *view {
            ViewMode::World => ViewMode::Map,
            ViewMode::Map => ViewMode::World,
        };
        log.push(format!("View: {:?}", view));
    }
}

fn fixed_time_from_config(config: &SimConfig) -> Time<Fixed> {
    let tick_hz = if config.tick_hz <= 0.0 { 10.0 } else { config.tick_hz };
    let seconds = 1.0 / tick_hz;
    Time::<Fixed>::from_duration(Duration::from_secs_f32(seconds))
}
