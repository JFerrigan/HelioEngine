use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use font8x8::{UnicodeFonts, BASIC_FONTS};
use heliobound_audio::{GameAudio, SoundEffect};
use heliobound_core::{
    Camera, CityConfig, CityGenerator, DoomMapConfig, DoomMapGenerator, PlanetConfig,
    ProceduralPlanet, Ray, Vec3, VoxelCell, VoxelCoord, VoxelMaterial, VoxelWorld,
};
use heliobound_gfx::{
    raycast, GraphicsConfig, Layer, MaterialGlyphMap, Overlay, Scene, SceneBuilder, SceneCell,
    TextStyle, Viewport,
};
use pixels::{PixelsBuilder, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

const CHAR_WIDTH: usize = 8;
const CHAR_HEIGHT: usize = 8;
const VIEWPORT: Viewport = Viewport {
    width: 160,
    height: 90,
};
const FRAME_WIDTH: u32 = (VIEWPORT.width * CHAR_WIDTH) as u32;
const FRAME_HEIGHT: u32 = (VIEWPORT.height * CHAR_HEIGHT) as u32;
const PLANET_RADIUS: i32 = 42_000_000;
const PLANET_TERRAIN_AMPLITUDE: f32 = 5_000_000.0;
const PLANET_START_ALTITUDE: f32 = 125_000.0;
const PLANET_START_Y_OFFSET: f32 = 18_000.0;
const PLANET_VIEW_DISTANCE: f32 = 8_000_000.0;
const FLIGHT_SPEED: f32 = 12_000.0;
const WALK_SPEED: f32 = 15.0;
const BOOST_MULTIPLIER: f32 = 8.0;
const WALK_BOOST_MULTIPLIER: f32 = 2.25;
const WALK_EYE_HEIGHT: f32 = 3.2;
const WALK_COLLISION_RADIUS: f32 = 0.34;
const CITY_FIGURE_EYE_HEIGHT: f32 = WALK_EYE_HEIGHT;
const CITY_FIGURE_SPEED: f32 = 4.0;
const CITY_FIGURE_GAZE_DISTANCE: f32 = 70.0;
const CITY_FIGURE_GAZE_DOT: f32 = 0.93;
const ENEMY_EYE_HEIGHT: f32 = WALK_EYE_HEIGHT;
const ENEMY_SPEED: f32 = 5.5;
const ENEMY_ATTACK_RANGE: f32 = 2.4;
const ENEMY_ATTACK_DAMAGE: i32 = 8;
const WEAPON_RANGE: f32 = 95.0;
const WEAPON_DAMAGE: i32 = 55;
const SHOT_FLASH_TIME: f32 = 0.12;
const BULLET_TRACE_TIME: f32 = 0.18;
const MOUSE_SENSITIVITY: f32 = 0.0025;
const PITCH_LIMIT: f32 = 1.52;
const ROLL_SPEED: f32 = 1.8;
const CORN_MAZE_TILES: usize = 25;
const CORN_MAZE_TILE_SIZE: i32 = 15;
const CORN_WALK_EYE_HEIGHT: f32 = 9.6;
const CORN_WALK_SPEED: f32 = 34.0;
const CORN_COLLISION_RADIUS: f32 = 1.15;
const CORN_STALK_BASE_HEIGHT: i32 = 15;
const CORN_MINIMAP_RADIUS: i32 = 6;
const BAR_EYE_HEIGHT: f32 = 10.5;
const BAR_WALK_SPEED: f32 = 18.0;
const BAR_COLLISION_RADIUS: f32 = 0.85;
const STANDARD_WALK_PROFILE: WalkProfile = WalkProfile {
    eye_height: WALK_EYE_HEIGHT,
    speed: WALK_SPEED,
    collision_radius: WALK_COLLISION_RADIUS,
};
const CORN_WALK_PROFILE: WalkProfile = WalkProfile {
    eye_height: CORN_WALK_EYE_HEIGHT,
    speed: CORN_WALK_SPEED,
    collision_radius: CORN_COLLISION_RADIUS,
};
const BAR_WALK_PROFILE: WalkProfile = WalkProfile {
    eye_height: BAR_EYE_HEIGHT,
    speed: BAR_WALK_SPEED,
    collision_radius: BAR_COLLISION_RADIUS,
};
const NPC_BODY_OFFSETS: [(i32, i32, i32, bool); 19] = [
    (-1, 1, 0, false),
    (1, 1, 0, false),
    (-1, 2, 0, false),
    (0, 2, 0, false),
    (1, 2, 0, false),
    (-2, 2, 0, true),
    (2, 2, 0, true),
    (-1, 3, -1, true),
    (0, 3, -1, false),
    (1, 3, -1, true),
    (-1, 3, 0, true),
    (0, 3, 0, false),
    (1, 3, 0, true),
    (-1, 3, 1, true),
    (0, 3, 1, false),
    (1, 3, 1, true),
    (-1, 4, 0, true),
    (0, 4, 0, true),
    (1, 4, 0, true),
];

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = AppState::new();
    let mut audio = GameAudio::open();
    let mut mouse_captured = false;
    let mut last_frame = Instant::now();

    let window = {
        let size = LogicalSize::new(FRAME_WIDTH as f64, FRAME_HEIGHT as f64);
        #[allow(deprecated)]
        Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Heliobound")
                        .with_inner_size(size)
                        .with_min_inner_size(size),
                )
                .expect("failed to create window"),
        )
    };
    let mut pixels = {
        let surface = SurfaceTexture::new(FRAME_WIDTH, FRAME_HEIGHT, &window);
        PixelsBuilder::new(FRAME_WIDTH, FRAME_HEIGHT, surface)
            .build()
            .expect("failed to create pixels surface")
    };

    #[allow(deprecated)]
    event_loop.run(|event, elwt| match event {
        Event::WindowEvent { window_id, event } => {
            if window_id != window.id() {
                return;
            }

            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Focused(false) => {
                    app.input = PlayerInput::default();
                    mouse_captured = set_mouse_captured(&window, false);
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => {
                    if app.mode == AppMode::CityShooter && mouse_captured {
                        app.fire_weapon();
                        play_audio_events(&mut audio, app.drain_audio_events());
                    } else if app.mode != AppMode::Menu {
                        mouse_captured = set_mouse_captured(&window, true);
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let action = app.handle_keyboard(&event.physical_key, event.state);
                    match action {
                        KeyboardAction::None => {}
                        KeyboardAction::Exit => elwt.exit(),
                        KeyboardAction::ReleaseMouse => {
                            if mouse_captured {
                                mouse_captured = set_mouse_captured(&window, false);
                            } else {
                                app.enter_menu();
                                audio.leave_ambience();
                            }
                        }
                        KeyboardAction::EnterMenu => {
                            app.enter_menu();
                            audio.leave_ambience();
                            mouse_captured = set_mouse_captured(&window, false);
                        }
                        KeyboardAction::StartScene => {
                            update_mode_audio(&mut audio, app.mode);
                            mouse_captured = set_mouse_captured(&window, false);
                        }
                        KeyboardAction::Fire => {
                            app.fire_weapon();
                            play_audio_events(&mut audio, app.drain_audio_events());
                        }
                    }
                }
                WindowEvent::Resized(size) => {
                    pixels
                        .resize_surface(size.width, size.height)
                        .expect("failed to resize pixels surface");
                }
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let dt = (now - last_frame).as_secs_f32().min(0.1);
                    last_frame = now;

                    let scene = app.frame(dt, mouse_captured);
                    play_audio_events(&mut audio, app.drain_audio_events());
                    render_scene(
                        &scene,
                        pixels.frame_mut(),
                        FRAME_WIDTH as usize,
                        FRAME_HEIGHT as usize,
                    );
                    pixels.render().expect("failed to render pixels");
                }
                _ => {}
            }
        }
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => {
            if mouse_captured {
                app.apply_mouse_motion(delta.0 as f32, delta.1 as f32);
            }
        }
        Event::AboutToWait => {
            window.request_redraw();
        }
        _ => {}
    })?;

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppMode {
    Menu,
    PlanetFlight,
    CityWalk,
    CityShooter,
    CornMaze,
    BarScene,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardAction {
    None,
    Exit,
    ReleaseMouse,
    EnterMenu,
    StartScene,
    Fire,
}

struct AppState {
    mode: AppMode,
    planet: ProceduralPlanet,
    city: VoxelWorld,
    doom_map: VoxelWorld,
    bar_scene: VoxelWorld,
    corn_maze: CornMazeState,
    planet_builder: SceneBuilder,
    city_builder: SceneBuilder,
    camera: Camera,
    input: PlayerInput,
    city_figures: CityFigureState,
    shooter: ShooterState,
    audio_events: Vec<SoundEffect>,
    tick: u64,
}

impl AppState {
    fn new() -> Self {
        Self {
            mode: AppMode::Menu,
            planet: build_demo_planet(),
            city: build_demo_city(),
            doom_map: build_doom_map(),
            bar_scene: build_bar_scene(),
            corn_maze: CornMazeState::new(),
            planet_builder: SceneBuilder::new(
                GraphicsConfig {
                    viewport: VIEWPORT,
                    max_distance: PLANET_VIEW_DISTANCE,
                },
                MaterialGlyphMap,
            ),
            city_builder: SceneBuilder::new(
                GraphicsConfig {
                    viewport: VIEWPORT,
                    max_distance: 140.0,
                },
                MaterialGlyphMap,
            ),
            camera: planet_start_camera(),
            input: PlayerInput::default(),
            city_figures: CityFigureState::new(),
            shooter: ShooterState::new(),
            audio_events: Vec::new(),
            tick: 0,
        }
    }

    fn handle_keyboard(&mut self, key: &PhysicalKey, state: ElementState) -> KeyboardAction {
        let pressed = state == ElementState::Pressed;

        if pressed {
            match (self.mode, key) {
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit1)) => {
                    self.start_planet();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit2)) => {
                    self.start_city();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit3)) => {
                    self.start_shooter();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit4)) => {
                    self.start_corn_maze();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit5)) => {
                    self.start_bar();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::Exit;
                }
                (AppMode::CityShooter, PhysicalKey::Code(KeyCode::Space)) => {
                    return KeyboardAction::Fire;
                }
                (_, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::ReleaseMouse;
                }
                (_, PhysicalKey::Code(KeyCode::KeyM)) => {
                    return KeyboardAction::EnterMenu;
                }
                _ => {}
            }
        }

        if self.mode != AppMode::Menu {
            handle_movement_input(&mut self.input, key, state);
        }

        KeyboardAction::None
    }

    fn enter_menu(&mut self) {
        self.mode = AppMode::Menu;
        self.input = PlayerInput::default();
    }

    fn start_planet(&mut self) {
        self.mode = AppMode::PlanetFlight;
        self.camera = planet_start_camera();
        self.input = PlayerInput::default();
    }

    fn start_city(&mut self) {
        self.mode = AppMode::CityWalk;
        self.camera = city_start_camera();
        self.input = PlayerInput::default();
        self.city_figures = CityFigureState::new();
    }

    fn start_shooter(&mut self) {
        self.mode = AppMode::CityShooter;
        self.camera = doom_start_camera();
        self.input = PlayerInput::default();
        self.shooter = ShooterState::new();
    }

    fn start_corn_maze(&mut self) {
        self.mode = AppMode::CornMaze;
        self.corn_maze = CornMazeState::new();
        self.camera = corn_maze_start_camera(&self.corn_maze);
        self.input = PlayerInput::default();
    }

    fn start_bar(&mut self) {
        self.mode = AppMode::BarScene;
        self.camera = bar_start_camera();
        self.input = PlayerInput::default();
    }

    fn frame(&mut self, dt: f32, mouse_captured: bool) -> Scene {
        self.tick = self.tick.wrapping_add(1);

        match self.mode {
            AppMode::Menu => build_menu_scene(self.tick),
            AppMode::PlanetFlight => {
                update_flight_camera(&mut self.camera, &self.input, dt);
                self.planet_builder
                    .build_planet(&self.planet, &self.camera, self.tick)
            }
            AppMode::CityWalk => {
                update_walking_camera(&mut self.camera, &self.input, &self.city, dt);
                self.city_figures.update(&self.city, &self.camera, dt);
                let render_world = city_world_with_figures(&self.city, &self.city_figures);
                let mut scene = self
                    .city_builder
                    .build(&render_world, &self.camera, self.tick);
                render_city_walk_scene(&mut scene, &self.city_figures, mouse_captured);
                scene
            }
            AppMode::CityShooter => {
                update_walking_camera(&mut self.camera, &self.input, &self.doom_map, dt);
                if self
                    .shooter
                    .update(&self.doom_map, self.camera.position, dt)
                {
                    self.audio_events.push(SoundEffect::PlayerHurt);
                }
                let render_world = shooter_world_with_enemies(&self.doom_map, &self.shooter);
                let mut scene = self
                    .city_builder
                    .build(&render_world, &self.camera, self.tick);
                render_shooter_scene(&mut scene, &self.camera, &self.shooter, mouse_captured);
                scene
            }
            AppMode::CornMaze => {
                update_walking_camera_with_profile(
                    &mut self.camera,
                    &self.input,
                    &self.corn_maze.world,
                    CORN_WALK_PROFILE,
                    dt,
                );
                self.corn_maze.update(self.camera.position);
                let mut scene =
                    self.city_builder
                        .build(&self.corn_maze.world, &self.camera, self.tick);
                render_corn_maze_scene(&mut scene, &self.corn_maze, &self.camera, mouse_captured);
                scene
            }
            AppMode::BarScene => {
                update_walking_camera_with_profile(
                    &mut self.camera,
                    &self.input,
                    &self.bar_scene,
                    BAR_WALK_PROFILE,
                    dt,
                );
                let mut scene = self
                    .city_builder
                    .build(&self.bar_scene, &self.camera, self.tick);
                render_bar_scene(&mut scene, mouse_captured);
                scene
            }
        }
    }

    fn apply_mouse_motion(&mut self, delta_x: f32, delta_y: f32) {
        match self.mode {
            AppMode::Menu => {}
            AppMode::PlanetFlight => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Unrestricted)
            }
            AppMode::CityWalk | AppMode::CityShooter | AppMode::CornMaze | AppMode::BarScene => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Clamped)
            }
        }
    }

    fn fire_weapon(&mut self) {
        if self.mode == AppMode::CityShooter {
            self.audio_events
                .extend(self.shooter.fire(&self.doom_map, &self.camera));
        }
    }

    fn drain_audio_events(&mut self) -> Vec<SoundEffect> {
        self.audio_events.drain(..).collect()
    }
}

fn update_mode_audio(audio: &mut GameAudio, mode: AppMode) {
    match mode {
        AppMode::CityWalk => audio.enter_city_mode(),
        AppMode::CornMaze => audio.enter_corn_maze_mode(),
        AppMode::BarScene => audio.enter_bar_mode(),
        AppMode::CityShooter => audio.enter_doom_mode(),
        AppMode::Menu | AppMode::PlanetFlight => audio.leave_ambience(),
    }
}

fn play_audio_events(audio: &mut GameAudio, events: Vec<SoundEffect>) {
    for event in events {
        audio.play_effect(event);
    }
}

fn look_at(position: Vec3, target: Vec3) -> Camera {
    let direction = (target - position).normalized();
    let yaw = direction.x.atan2(direction.z);
    let pitch = direction.y.asin();
    Camera::new(position).looking_at(yaw, pitch)
}

fn planet_start_camera() -> Camera {
    let envelope_radius = PLANET_RADIUS as f32 + PLANET_TERRAIN_AMPLITUDE;
    look_at(
        Vec3::new(
            0.0,
            PLANET_START_Y_OFFSET,
            -(envelope_radius + PLANET_START_ALTITUDE),
        ),
        Vec3::ZERO,
    )
    .with_fov_y(55.0_f32.to_radians())
    .with_max_distance(PLANET_VIEW_DISTANCE)
}

fn city_start_camera() -> Camera {
    Camera::new(Vec3::new(0.5, WALK_EYE_HEIGHT, -55.5))
        .looking_at(0.0, 0.0)
        .with_fov_y(62.0_f32.to_radians())
        .with_max_distance(140.0)
}

fn doom_start_camera() -> Camera {
    Camera::new(Vec3::new(0.5, WALK_EYE_HEIGHT, -55.5))
        .looking_at(0.0, 0.0)
        .with_fov_y(68.0_f32.to_radians())
        .with_max_distance(140.0)
}

fn corn_maze_start_camera(maze: &CornMazeState) -> Camera {
    look_at(
        Vec3::new(
            maze.start_position.x,
            CORN_WALK_EYE_HEIGHT,
            maze.start_position.z,
        ),
        Vec3::new(
            maze.start_position.x + 24.0,
            CORN_WALK_EYE_HEIGHT,
            maze.start_position.z,
        ),
    )
    .with_fov_y(64.0_f32.to_radians())
    .with_max_distance(180.0)
}

fn bar_start_camera() -> Camera {
    look_at(
        Vec3::new(0.5, BAR_EYE_HEIGHT, -34.5),
        Vec3::new(-34.0, BAR_EYE_HEIGHT, 28.0),
    )
    .with_fov_y(66.0_f32.to_radians())
    .with_max_distance(120.0)
}

#[derive(Clone, Copy, Debug, Default)]
struct PlayerInput {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    roll_left: bool,
    roll_right: bool,
    boost: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WalkProfile {
    eye_height: f32,
    speed: f32,
    collision_radius: f32,
}

#[derive(Clone, Debug)]
struct CityFigureState {
    figures: Vec<CityFigure>,
}

impl CityFigureState {
    fn new() -> Self {
        Self {
            figures: spawn_city_figures(),
        }
    }

    fn update(&mut self, city: &VoxelWorld, camera: &Camera, dt: f32) {
        for figure in &mut self.figures {
            figure.watching_player = figure.is_looking_at_player(city, camera);
            if figure.watching_player {
                continue;
            }

            figure.advance(city, dt);
        }
    }

    fn watching_count(&self) -> usize {
        self.figures
            .iter()
            .filter(|figure| figure.watching_player)
            .count()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CityFigure {
    position: Vec3,
    route: Vec<Vec3>,
    waypoint: usize,
    last_direction: Vec3,
    watching_player: bool,
}

impl CityFigure {
    fn new(route: Vec<Vec3>) -> Self {
        let position = route.first().copied().unwrap_or(Vec3::ZERO);
        let waypoint = if route.len() > 1 { 1 } else { 0 };
        let last_direction = route
            .get(waypoint)
            .copied()
            .map(|target| horizontal(target - position))
            .unwrap_or(Vec3::new(0.0, 0.0, 1.0));

        Self {
            position,
            route,
            waypoint,
            last_direction,
            watching_player: false,
        }
    }

    fn advance(&mut self, city: &VoxelWorld, dt: f32) {
        if self.route.is_empty() {
            return;
        }

        let mut to_waypoint = horizontal(self.route[self.waypoint] - self.position);
        if horizontal_distance(self.position, self.route[self.waypoint]) < 0.35 {
            self.waypoint = (self.waypoint + 1) % self.route.len();
            to_waypoint = horizontal(self.route[self.waypoint] - self.position);
        }

        if to_waypoint.length() <= f32::EPSILON {
            return;
        }

        self.last_direction = to_waypoint;
        let candidate = self.position + to_waypoint * CITY_FIGURE_SPEED * dt;
        let candidate = Vec3::new(candidate.x, 0.0, candidate.z);
        if can_walk_to(city, Vec3::new(candidate.x, WALK_EYE_HEIGHT, candidate.z)) {
            self.position = candidate;
        }
    }

    fn is_looking_at_player(&self, city: &VoxelWorld, camera: &Camera) -> bool {
        let distance = horizontal_distance(self.position, camera.position);
        if distance > CITY_FIGURE_GAZE_DISTANCE {
            return false;
        }

        let to_player = horizontal(camera.position - self.target_position());
        if to_player.length() <= f32::EPSILON {
            return true;
        }

        self.last_direction.dot(to_player) >= CITY_FIGURE_GAZE_DOT
            && has_line_of_sight(city, self.target_position(), camera.position)
    }

    fn target_position(&self) -> Vec3 {
        Vec3::new(self.position.x, CITY_FIGURE_EYE_HEIGHT, self.position.z)
    }
}

fn spawn_city_figures() -> Vec<CityFigure> {
    vec![
        CityFigure::new(vec![
            Vec3::new(0.5, 0.0, -48.5),
            Vec3::new(0.5, 0.0, 48.5),
            Vec3::new(48.5, 0.0, 48.5),
            Vec3::new(48.5, 0.0, -48.5),
        ]),
        CityFigure::new(vec![
            Vec3::new(-32.5, 0.0, -64.5),
            Vec3::new(-32.5, 0.0, 0.5),
            Vec3::new(32.5, 0.0, 0.5),
            Vec3::new(32.5, 0.0, -64.5),
        ]),
        CityFigure::new(vec![
            Vec3::new(-64.5, 0.0, 16.5),
            Vec3::new(16.5, 0.0, 16.5),
            Vec3::new(16.5, 0.0, 64.5),
            Vec3::new(-64.5, 0.0, 64.5),
        ]),
        CityFigure::new(vec![
            Vec3::new(64.5, 0.0, -16.5),
            Vec3::new(-16.5, 0.0, -16.5),
            Vec3::new(-16.5, 0.0, 32.5),
            Vec3::new(64.5, 0.0, 32.5),
        ]),
        CityFigure::new(vec![
            Vec3::new(-48.5, 0.0, 48.5),
            Vec3::new(-48.5, 0.0, -32.5),
            Vec3::new(16.5, 0.0, -32.5),
            Vec3::new(16.5, 0.0, 48.5),
        ]),
    ]
}

#[derive(Clone, Debug)]
struct CornMazeState {
    world: VoxelWorld,
    open_tiles: Vec<bool>,
    start_position: Vec3,
    exit_position: Vec3,
    escaped: bool,
}

impl CornMazeState {
    fn new() -> Self {
        let (world, open_tiles, start_position, exit_position) = build_corn_maze();
        Self {
            world,
            open_tiles,
            start_position,
            exit_position,
            escaped: false,
        }
    }

    fn update(&mut self, player_position: Vec3) {
        if horizontal_distance(player_position, self.exit_position) < 3.2 {
            self.escaped = true;
        }
    }
}

#[derive(Clone, Debug)]
struct ShooterState {
    enemies: Vec<Enemy>,
    bullet_traces: Vec<BulletTrace>,
    health: i32,
    kills: u32,
    shots_fired: u32,
    shot_flash_timer: f32,
}

impl ShooterState {
    fn new() -> Self {
        Self {
            enemies: spawn_enemies(),
            bullet_traces: Vec::new(),
            health: 100,
            kills: 0,
            shots_fired: 0,
            shot_flash_timer: 0.0,
        }
    }

    fn update(&mut self, city: &VoxelWorld, player_position: Vec3, dt: f32) -> bool {
        self.shot_flash_timer = (self.shot_flash_timer - dt).max(0.0);
        for trace in &mut self.bullet_traces {
            trace.time_left = (trace.time_left - dt).max(0.0);
        }
        self.bullet_traces
            .retain(|trace| trace.time_left > f32::EPSILON);

        let mut player_hurt = false;
        for enemy in &mut self.enemies {
            if !enemy.is_alive() {
                continue;
            }

            enemy.attack_cooldown = (enemy.attack_cooldown - dt).max(0.0);
            let to_player = horizontal(player_position - enemy.position);
            let distance = horizontal_distance(enemy.position, player_position);

            if distance <= ENEMY_ATTACK_RANGE {
                if enemy.attack_cooldown <= 0.0 {
                    self.health = (self.health - ENEMY_ATTACK_DAMAGE).max(0);
                    enemy.attack_cooldown = 1.1;
                    player_hurt = true;
                }
            } else if distance < 80.0 && to_player.length() > f32::EPSILON {
                let candidate = enemy.position + to_player * ENEMY_SPEED * dt;
                let candidate = Vec3::new(candidate.x, WALK_EYE_HEIGHT, candidate.z);
                if can_walk_to(city, candidate) {
                    enemy.position.x = candidate.x;
                    enemy.position.z = candidate.z;
                }
            }
        }

        player_hurt
    }

    fn fire(&mut self, city: &VoxelWorld, camera: &Camera) -> Vec<SoundEffect> {
        self.shots_fired += 1;
        self.shot_flash_timer = SHOT_FLASH_TIME;
        self.bullet_traces.push(BulletTrace::from_camera(camera));
        let mut effects = vec![SoundEffect::Gunshot];

        let render_world = shooter_world_with_enemies(city, self);
        let Some(hit) = raycast(
            &render_world,
            Ray::new(camera.position, camera.forward()),
            WEAPON_RANGE,
        ) else {
            return effects;
        };
        let Some(index) = self.enemy_index_for_voxel(hit.coord) else {
            return effects;
        };

        let enemy = &mut self.enemies[index];
        enemy.health = (enemy.health - WEAPON_DAMAGE).max(0);
        if enemy.health == 0 {
            self.kills += 1;
            effects.push(SoundEffect::EnemyDeath);
        } else {
            effects.push(SoundEffect::EnemyHit);
        }
        effects
    }

    fn enemy_index_for_voxel(&self, coord: VoxelCoord) -> Option<usize> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| enemy.is_alive())
            .find_map(|(index, enemy)| enemy.contains_voxel(coord).then_some(index))
    }

    fn alive_count(&self) -> usize {
        self.enemies.iter().filter(|enemy| enemy.is_alive()).count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BulletTrace {
    origin: Vec3,
    direction: Vec3,
    time_left: f32,
}

impl BulletTrace {
    fn from_camera(camera: &Camera) -> Self {
        Self {
            origin: camera.position + camera.right() * 0.22 - camera.up() * 0.34,
            direction: camera.forward(),
            time_left: BULLET_TRACE_TIME,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Enemy {
    position: Vec3,
    health: i32,
    attack_cooldown: f32,
}

impl Enemy {
    fn new(x: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, 0.0, z),
            health: 100,
            attack_cooldown: 0.0,
        }
    }

    fn is_alive(self) -> bool {
        self.health > 0
    }

    fn contains_voxel(self, coord: VoxelCoord) -> bool {
        npc_body_contains_voxel(self.position, coord)
    }
}

fn spawn_enemies() -> Vec<Enemy> {
    vec![
        Enemy::new(0.5, -34.5),
        Enemy::new(-42.5, -22.5),
        Enemy::new(39.5, -5.5),
        Enemy::new(-18.5, 39.5),
        Enemy::new(22.5, 42.5),
        Enemy::new(0.5, 53.5),
    ]
}

fn handle_movement_input(input: &mut PlayerInput, key: &PhysicalKey, state: ElementState) {
    let pressed = state == ElementState::Pressed;
    match key {
        PhysicalKey::Code(KeyCode::KeyW) => input.forward = pressed,
        PhysicalKey::Code(KeyCode::KeyS) => input.backward = pressed,
        PhysicalKey::Code(KeyCode::KeyA) => input.left = pressed,
        PhysicalKey::Code(KeyCode::KeyD) => input.right = pressed,
        PhysicalKey::Code(KeyCode::Space) => input.up = pressed,
        PhysicalKey::Code(KeyCode::ControlLeft) | PhysicalKey::Code(KeyCode::ControlRight) => {
            input.down = pressed
        }
        PhysicalKey::Code(KeyCode::KeyQ) => input.roll_left = pressed,
        PhysicalKey::Code(KeyCode::KeyE) => input.roll_right = pressed,
        PhysicalKey::Code(KeyCode::ShiftLeft) | PhysicalKey::Code(KeyCode::ShiftRight) => {
            input.boost = pressed
        }
        _ => {}
    }
}

fn update_flight_camera(camera: &mut Camera, input: &PlayerInput, dt: f32) {
    let mut movement = Vec3::ZERO;

    if input.forward {
        movement = movement + camera.forward();
    }
    if input.backward {
        movement = movement - camera.forward();
    }
    if input.right {
        movement = movement + camera.right();
    }
    if input.left {
        movement = movement - camera.right();
    }
    if input.up {
        movement = movement + camera.up();
    }
    if input.down {
        movement = movement - camera.up();
    }
    if input.roll_left {
        camera.roll_by(ROLL_SPEED * dt);
    }
    if input.roll_right {
        camera.roll_by(-ROLL_SPEED * dt);
    }

    if movement.length() > f32::EPSILON {
        let speed = if input.boost {
            FLIGHT_SPEED * BOOST_MULTIPLIER
        } else {
            FLIGHT_SPEED
        };
        camera.position = camera.position + movement.normalized() * speed * dt;
    }
}

fn update_walking_camera(camera: &mut Camera, input: &PlayerInput, city: &VoxelWorld, dt: f32) {
    update_walking_camera_with_profile(camera, input, city, STANDARD_WALK_PROFILE, dt);
}

fn update_walking_camera_with_profile(
    camera: &mut Camera,
    input: &PlayerInput,
    city: &VoxelWorld,
    profile: WalkProfile,
    dt: f32,
) {
    let mut movement = Vec3::ZERO;
    let forward = horizontal(camera.forward());
    let right = horizontal(camera.right());

    if input.forward {
        movement = movement + forward;
    }
    if input.backward {
        movement = movement - forward;
    }
    if input.right {
        movement = movement + right;
    }
    if input.left {
        movement = movement - right;
    }

    if movement.length() > f32::EPSILON {
        let speed = if input.boost {
            profile.speed * WALK_BOOST_MULTIPLIER
        } else {
            profile.speed
        };
        let step = movement.normalized() * speed * dt;
        camera.position = move_walking_with_collision(camera.position, step, city, profile);
    }
}

fn move_walking_with_collision(
    position: Vec3,
    step: Vec3,
    city: &VoxelWorld,
    profile: WalkProfile,
) -> Vec3 {
    let full = Vec3::new(position.x + step.x, profile.eye_height, position.z + step.z);
    if can_walk_to_with_profile(city, full, profile) {
        return full;
    }

    let x_then_z = resolve_axis_slide(position, step.x, step.z, true, city, profile);
    let z_then_x = resolve_axis_slide(position, step.z, step.x, false, city, profile);
    if horizontal_distance(position, x_then_z) >= horizontal_distance(position, z_then_x) {
        x_then_z
    } else {
        z_then_x
    }
}

fn resolve_axis_slide(
    position: Vec3,
    primary: f32,
    secondary: f32,
    primary_is_x: bool,
    city: &VoxelWorld,
    profile: WalkProfile,
) -> Vec3 {
    let mut resolved = position;
    let primary_candidate = if primary_is_x {
        Vec3::new(resolved.x + primary, profile.eye_height, resolved.z)
    } else {
        Vec3::new(resolved.x, profile.eye_height, resolved.z + primary)
    };
    if can_walk_to_with_profile(city, primary_candidate, profile) {
        resolved = primary_candidate;
    }

    let secondary_candidate = if primary_is_x {
        Vec3::new(resolved.x, profile.eye_height, resolved.z + secondary)
    } else {
        Vec3::new(resolved.x + secondary, profile.eye_height, resolved.z)
    };
    if can_walk_to_with_profile(city, secondary_candidate, profile) {
        resolved = secondary_candidate;
    }

    Vec3::new(resolved.x, profile.eye_height, resolved.z)
}

fn horizontal(direction: Vec3) -> Vec3 {
    Vec3::new(direction.x, 0.0, direction.z).normalized()
}

fn can_walk_to(city: &VoxelWorld, position: Vec3) -> bool {
    can_walk_to_with_profile(city, position, STANDARD_WALK_PROFILE)
}

fn can_walk_to_with_profile(city: &VoxelWorld, position: Vec3, profile: WalkProfile) -> bool {
    let samples = [
        (0.0, 0.0),
        (-profile.collision_radius, -profile.collision_radius),
        (-profile.collision_radius, profile.collision_radius),
        (profile.collision_radius, -profile.collision_radius),
        (profile.collision_radius, profile.collision_radius),
    ];
    samples.iter().all(|(offset_x, offset_z)| {
        has_standing_clearance(
            city,
            Vec3::new(position.x + offset_x, position.y, position.z + offset_z),
            profile,
        )
    })
}

fn has_standing_clearance(city: &VoxelWorld, position: Vec3, profile: WalkProfile) -> bool {
    let x = position.x.floor() as i32;
    let z = position.z.floor() as i32;
    for y in 1..=profile.eye_height.ceil() as i32 {
        if city.get(VoxelCoord::new(x, y, z)).is_some() {
            return false;
        }
    }
    true
}

fn horizontal_distance(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, 0.0, a.z - b.z).length()
}

fn has_line_of_sight(city: &VoxelWorld, origin: Vec3, target: Vec3) -> bool {
    let delta = target - origin;
    let distance = delta.length();
    if distance <= 0.5 {
        return true;
    }

    raycast(city, Ray::new(origin, delta), distance - 0.35).is_none()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PitchMode {
    Clamped,
    Unrestricted,
}

fn apply_mouse_look(camera: &mut Camera, delta_x: f32, delta_y: f32, pitch_mode: PitchMode) {
    let horizontal = delta_x * MOUSE_SENSITIVITY;
    let vertical = -delta_y * MOUSE_SENSITIVITY;

    match pitch_mode {
        PitchMode::Clamped => {
            let roll_sin = camera.roll_radians.sin();
            let roll_cos = camera.roll_radians.cos();
            let yaw = wrap_angle(camera.yaw_radians + horizontal * roll_cos - vertical * roll_sin);
            let pitch = (camera.pitch_radians + horizontal * roll_sin + vertical * roll_cos)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT);
            camera.set_look_angles(yaw, pitch, 0.0);
        }
        PitchMode::Unrestricted => {
            camera.rotate_local_yaw_pitch(horizontal, vertical);
        }
    }
}

fn wrap_angle(radians: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (radians + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

fn set_mouse_captured(window: &Window, captured: bool) -> bool {
    if captured {
        let grabbed = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
            .is_ok();
        window.set_cursor_visible(!grabbed);
        grabbed
    } else {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        false
    }
}

fn build_demo_planet() -> ProceduralPlanet {
    ProceduralPlanet::new(PlanetConfig {
        seed: 0x48E1_10B0_0D,
        radius: PLANET_RADIUS,
        crust_depth: 3,
        terrain_amplitude: PLANET_TERRAIN_AMPLITUDE,
    })
}

fn build_demo_city() -> VoxelWorld {
    CityGenerator::new(CityConfig {
        seed: 0x51D5_C17A,
        half_extent: 72,
        block_size: 16,
        road_width: 5,
        max_height: 32,
    })
    .generate()
}

fn build_doom_map() -> VoxelWorld {
    DoomMapGenerator::new(DoomMapConfig::default()).generate()
}

#[derive(Clone, Copy, Debug)]
enum BarFacing {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy, Debug)]
enum BarPose {
    Standing,
    Sitting,
}

fn build_bar_scene() -> VoxelWorld {
    let mut world = VoxelWorld::new();

    stamp_bar_room(&mut world);
    stamp_bar_counter(&mut world);
    stamp_stage(&mut world);
    stamp_tables_and_chairs(&mut world);
    stamp_jukebox(&mut world);
    stamp_dart_boards(&mut world);
    stamp_bar_people(&mut world);

    world
}

fn stamp_bar_room(world: &mut VoxelWorld) {
    fill_cuboid(
        world,
        VoxelCoord::new(-72, 0, -48),
        VoxelCoord::new(72, 0, 48),
        VoxelMaterial::Regolith,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-72, 1, -48),
        VoxelCoord::new(-71, 18, 48),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(71, 1, -48),
        VoxelCoord::new(72, 18, 48),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-72, 1, -48),
        VoxelCoord::new(72, 18, -47),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-72, 1, 47),
        VoxelCoord::new(72, 18, 48),
        VoxelMaterial::Basalt,
    );

    for z in (-42..=42).step_by(12) {
        fill_cuboid(
            world,
            VoxelCoord::new(-68, 18, z),
            VoxelCoord::new(68, 18, z + 1),
            VoxelMaterial::ShipHull,
        );
    }
    for x in (-60..=60).step_by(20) {
        fill_cuboid(
            world,
            VoxelCoord::new(x, 1, -44),
            VoxelCoord::new(x + 1, 17, -43),
            VoxelMaterial::ShipHull,
        );
        fill_cuboid(
            world,
            VoxelCoord::new(x, 1, 43),
            VoxelCoord::new(x + 1, 17, 44),
            VoxelMaterial::ShipHull,
        );
    }
}

fn stamp_bar_counter(world: &mut VoxelWorld) {
    fill_cuboid(
        world,
        VoxelCoord::new(43, 1, -34),
        VoxelCoord::new(58, 5, 32),
        VoxelMaterial::Habitat,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(41, 6, -34),
        VoxelCoord::new(59, 6, 32),
        VoxelMaterial::ShipHull,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(39, 2, -33),
        VoxelCoord::new(39, 2, 31),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(62, 1, -38),
        VoxelCoord::new(68, 14, 36),
        VoxelMaterial::Habitat,
    );

    for y in [4, 8, 12] {
        fill_cuboid(
            world,
            VoxelCoord::new(59, y, -36),
            VoxelCoord::new(62, y, 34),
            VoxelMaterial::ShipHull,
        );
    }
    for z in (-31..=29).step_by(5) {
        stamp_bottle(world, 40, 7, z);
        stamp_bottle(world, 60, 5, z - 1);
        stamp_bottle(world, 60, 9, z + 1);
        stamp_bottle(world, 60, 13, z);
    }
}

fn stamp_stage(world: &mut VoxelWorld) {
    fill_cuboid(
        world,
        VoxelCoord::new(-62, 1, 20),
        VoxelCoord::new(-16, 3, 43),
        VoxelMaterial::ShipHull,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-62, 4, 43),
        VoxelCoord::new(-16, 16, 45),
        VoxelMaterial::CarbonLife,
    );
    for x in (-58..=-20).step_by(8) {
        fill_cuboid(
            world,
            VoxelCoord::new(x, 4, 42),
            VoxelCoord::new(x + 1, 15, 42),
            VoxelMaterial::Basalt,
        );
    }

    stamp_microphone(world, -38, 4, 30);
    stamp_drum_kit(world, -52, 4, 35);
    stamp_bottle(world, -22, 4, 25);
}

fn stamp_tables_and_chairs(world: &mut VoxelWorld) {
    for (x, z) in [(-30, -22), (-4, -20), (22, -20), (-24, 6), (18, 7)] {
        stamp_table(world, x, z);
        stamp_chair(world, x - 7, z, BarFacing::East);
        stamp_chair(world, x + 7, z, BarFacing::West);
        stamp_chair(world, x, z - 7, BarFacing::South);
    }

    for z in (-25..=25).step_by(10) {
        stamp_chair(world, 35, z, BarFacing::East);
    }
}

fn stamp_table(world: &mut VoxelWorld, x: i32, z: i32) {
    fill_cuboid(
        world,
        VoxelCoord::new(x - 4, 5, z - 3),
        VoxelCoord::new(x + 4, 5, z + 3),
        VoxelMaterial::Habitat,
    );
    for (dx, dz) in [(-3, -2), (3, -2), (-3, 2), (3, 2)] {
        fill_cuboid(
            world,
            VoxelCoord::new(x + dx, 1, z + dz),
            VoxelCoord::new(x + dx, 4, z + dz),
            VoxelMaterial::Basalt,
        );
    }
    stamp_ash_tray(world, x, 6, z);
    stamp_bottle(world, x + 3, 6, z - 1);
}

fn stamp_chair(world: &mut VoxelWorld, x: i32, z: i32, facing: BarFacing) {
    fill_oriented_cuboid(
        world,
        x,
        z,
        facing,
        VoxelCoord::new(-2, 2, -2),
        VoxelCoord::new(2, 3, 2),
        VoxelMaterial::Habitat,
    );
    fill_oriented_cuboid(
        world,
        x,
        z,
        facing,
        VoxelCoord::new(-2, 4, 2),
        VoxelCoord::new(2, 9, 2),
        VoxelMaterial::ShipHull,
    );
    for (lx, lz) in [(-2, -2), (2, -2), (-2, 2), (2, 2)] {
        fill_oriented_cuboid(
            world,
            x,
            z,
            facing,
            VoxelCoord::new(lx, 1, lz),
            VoxelCoord::new(lx, 2, lz),
            VoxelMaterial::Basalt,
        );
    }
}

fn stamp_jukebox(world: &mut VoxelWorld) {
    fill_cuboid(
        world,
        VoxelCoord::new(-68, 1, -36),
        VoxelCoord::new(-59, 10, -25),
        VoxelMaterial::Habitat,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-67, 5, -35),
        VoxelCoord::new(-58, 10, -26),
        VoxelMaterial::Glass,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-66, 7, -34),
        VoxelCoord::new(-59, 8, -27),
        VoxelMaterial::Beacon,
    );
    for z in [-34, -30, -26] {
        fill_cuboid(
            world,
            VoxelCoord::new(-58, 2, z),
            VoxelCoord::new(-58, 4, z),
            VoxelMaterial::ShipHull,
        );
    }
}

fn stamp_dart_boards(world: &mut VoxelWorld) {
    for z in [8_i32, 24] {
        for y in 7_i32..=13 {
            for dz in -3_i32..=3 {
                let distance = (y - 10).abs() + dz.abs();
                if distance <= 4 {
                    let material = if distance <= 1 {
                        VoxelMaterial::Beacon
                    } else if distance == 2 {
                        VoxelMaterial::Glass
                    } else {
                        VoxelMaterial::Habitat
                    };
                    world.set(VoxelCoord::new(-70, y, z + dz), VoxelCell::new(material));
                }
            }
        }
    }
}

fn stamp_bar_people(world: &mut VoxelWorld) {
    let people = [
        (-37, 29, BarFacing::South, BarPose::Standing),
        (-52, 31, BarFacing::East, BarPose::Standing),
        (-37, -22, BarFacing::East, BarPose::Sitting),
        (-23, -22, BarFacing::West, BarPose::Sitting),
        (-4, -27, BarFacing::North, BarPose::Sitting),
        (29, -20, BarFacing::West, BarPose::Sitting),
        (35, -5, BarFacing::East, BarPose::Sitting),
        (35, 15, BarFacing::East, BarPose::Sitting),
        (6, 6, BarFacing::West, BarPose::Standing),
        (48, 26, BarFacing::West, BarPose::Standing),
        (-60, -12, BarFacing::East, BarPose::Standing),
    ];

    for (index, (x, z, facing, pose)) in people.into_iter().enumerate() {
        stamp_bar_person(world, x, z, facing, pose, index);
    }
}

fn stamp_bar_person(
    world: &mut VoxelWorld,
    x: i32,
    z: i32,
    facing: BarFacing,
    pose: BarPose,
    index: usize,
) {
    let clothing = if index % 3 == 0 {
        VoxelMaterial::SiliconLife
    } else if index % 3 == 1 {
        VoxelMaterial::Glass
    } else {
        VoxelMaterial::Habitat
    };
    let accent = if index % 2 == 0 {
        VoxelMaterial::Beacon
    } else {
        VoxelMaterial::ShipHull
    };

    match pose {
        BarPose::Standing => {
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(-2, 1, -1),
                VoxelCoord::new(-1, 5, 1),
                VoxelMaterial::CarbonLife,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(1, 1, -1),
                VoxelCoord::new(2, 5, 1),
                VoxelMaterial::CarbonLife,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(-2, 6, -1),
                VoxelCoord::new(2, 10, 1),
                clothing,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(-4, 7, 0),
                VoxelCoord::new(-3, 10, 0),
                VoxelMaterial::CarbonLife,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(3, 7, 0),
                VoxelCoord::new(4, 10, 0),
                VoxelMaterial::CarbonLife,
            );
            stamp_bar_head(world, x, z, facing, 11, accent);
        }
        BarPose::Sitting => {
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(-2, 1, -2),
                VoxelCoord::new(-1, 3, 1),
                VoxelMaterial::CarbonLife,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(1, 1, -2),
                VoxelCoord::new(2, 3, 1),
                VoxelMaterial::CarbonLife,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(-2, 4, -1),
                VoxelCoord::new(2, 8, 1),
                clothing,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(-4, 5, 0),
                VoxelCoord::new(-3, 8, 0),
                VoxelMaterial::CarbonLife,
            );
            fill_oriented_cuboid(
                world,
                x,
                z,
                facing,
                VoxelCoord::new(3, 5, 0),
                VoxelCoord::new(4, 8, 0),
                VoxelMaterial::CarbonLife,
            );
            stamp_bar_head(world, x, z, facing, 9, accent);
        }
    }
}

fn stamp_bar_head(
    world: &mut VoxelWorld,
    x: i32,
    z: i32,
    facing: BarFacing,
    base_y: i32,
    accent: VoxelMaterial,
) {
    fill_oriented_cuboid(
        world,
        x,
        z,
        facing,
        VoxelCoord::new(-1, base_y, -1),
        VoxelCoord::new(1, base_y + 2, 1),
        VoxelMaterial::CarbonLife,
    );
    set_oriented(world, x, z, facing, -1, base_y + 1, -2, accent);
    set_oriented(world, x, z, facing, 1, base_y + 1, -2, accent);
    fill_oriented_cuboid(
        world,
        x,
        z,
        facing,
        VoxelCoord::new(-2, base_y + 3, -1),
        VoxelCoord::new(2, base_y + 3, 1),
        accent,
    );
}

fn stamp_bottle(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    world.set(
        VoxelCoord::new(x, y, z),
        VoxelCell::new(VoxelMaterial::Glass),
    );
    world.set(
        VoxelCoord::new(x, y + 1, z),
        VoxelCell::new(VoxelMaterial::Glass),
    );
    world.set(
        VoxelCoord::new(x, y + 2, z),
        VoxelCell::new(VoxelMaterial::Beacon),
    );
}

fn stamp_ash_tray(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    for (dx, dz) in [(-1, 0), (0, -1), (0, 0), (0, 1), (1, 0)] {
        world.set(
            VoxelCoord::new(x + dx, y, z + dz),
            VoxelCell::new(VoxelMaterial::Basalt),
        );
    }
    world.set(
        VoxelCoord::new(x, y + 1, z),
        VoxelCell::new(VoxelMaterial::Glass),
    );
}

fn stamp_microphone(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    fill_cuboid(
        world,
        VoxelCoord::new(x, y, z),
        VoxelCoord::new(x, y + 8, z),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x - 1, y + 8, z - 1),
        VoxelCoord::new(x + 1, y + 9, z + 1),
        VoxelMaterial::Glass,
    );
}

fn stamp_drum_kit(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    fill_cuboid(
        world,
        VoxelCoord::new(x - 3, y, z - 1),
        VoxelCoord::new(x + 3, y + 3, z + 1),
        VoxelMaterial::Habitat,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x - 5, y + 4, z - 3),
        VoxelCoord::new(x - 2, y + 4, z),
        VoxelMaterial::Glass,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x + 2, y + 5, z - 3),
        VoxelCoord::new(x + 5, y + 5, z),
        VoxelMaterial::Glass,
    );
}

fn fill_cuboid(world: &mut VoxelWorld, a: VoxelCoord, b: VoxelCoord, material: VoxelMaterial) {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    let min_z = a.z.min(b.z);
    let max_z = a.z.max(b.z);

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                world.set(VoxelCoord::new(x, y, z), VoxelCell::new(material));
            }
        }
    }
}

fn fill_oriented_cuboid(
    world: &mut VoxelWorld,
    origin_x: i32,
    origin_z: i32,
    facing: BarFacing,
    a: VoxelCoord,
    b: VoxelCoord,
    material: VoxelMaterial,
) {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    let min_z = a.z.min(b.z);
    let max_z = a.z.max(b.z);

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                set_oriented(world, origin_x, origin_z, facing, x, y, z, material);
            }
        }
    }
}

fn set_oriented(
    world: &mut VoxelWorld,
    origin_x: i32,
    origin_z: i32,
    facing: BarFacing,
    local_x: i32,
    y: i32,
    local_z: i32,
    material: VoxelMaterial,
) {
    let (right_x, right_z, forward_x, forward_z) = match facing {
        BarFacing::North => (1, 0, 0, -1),
        BarFacing::South => (-1, 0, 0, 1),
        BarFacing::East => (0, 1, 1, 0),
        BarFacing::West => (0, -1, -1, 0),
    };
    world.set(
        VoxelCoord::new(
            origin_x + local_x * right_x + local_z * forward_x,
            y,
            origin_z + local_x * right_z + local_z * forward_z,
        ),
        VoxelCell::new(material),
    );
}

fn build_corn_maze() -> (VoxelWorld, Vec<bool>, Vec3, Vec3) {
    let open = carve_corn_maze_tiles();
    let mut world = VoxelWorld::new();
    let start = corn_tile_center(1, 1);
    let exit = corn_tile_center(CORN_MAZE_TILES - 2, CORN_MAZE_TILES - 2);
    let half = corn_maze_half_extent();

    for tile_z in 0..CORN_MAZE_TILES {
        for tile_x in 0..CORN_MAZE_TILES {
            let tile_open = open[tile_z * CORN_MAZE_TILES + tile_x];
            let base_x = tile_x as i32 * CORN_MAZE_TILE_SIZE - half;
            let base_z = tile_z as i32 * CORN_MAZE_TILE_SIZE - half;

            for local_z in 0..CORN_MAZE_TILE_SIZE {
                for local_x in 0..CORN_MAZE_TILE_SIZE {
                    let x = base_x + local_x;
                    let z = base_z + local_z;
                    world.set(
                        VoxelCoord::new(x, 0, z),
                        VoxelCell::new(VoxelMaterial::Regolith),
                    );

                    if tile_open {
                        continue;
                    }

                    if should_place_corn_stalk(tile_x, tile_z, local_x, local_z) {
                        stamp_corn_stalk(&mut world, x, z, corn_stalk_height(tile_x, tile_z, x, z));
                    }
                }
            }
        }
    }

    stamp_exit_marker(&mut world, exit);
    (world, open, start, exit)
}

fn carve_corn_maze_tiles() -> Vec<bool> {
    let mut open = vec![false; CORN_MAZE_TILES * CORN_MAZE_TILES];
    let mut stack = vec![(1usize, 1usize)];
    set_corn_tile_open(&mut open, 1, 1);

    while let Some((x, z)) = stack.pop() {
        let mut moved = false;
        for (dx, dz) in shuffled_maze_dirs(x, z) {
            let Some(next_x) = x.checked_add_signed(dx * 2) else {
                continue;
            };
            let Some(next_z) = z.checked_add_signed(dz * 2) else {
                continue;
            };
            if next_x == 0
                || next_z == 0
                || next_x >= CORN_MAZE_TILES - 1
                || next_z >= CORN_MAZE_TILES - 1
                || corn_tile_open(&open, next_x, next_z)
            {
                continue;
            }

            set_corn_tile_open(
                &mut open,
                x.checked_add_signed(dx).unwrap(),
                z.checked_add_signed(dz).unwrap(),
            );
            set_corn_tile_open(&mut open, next_x, next_z);
            stack.push((x, z));
            stack.push((next_x, next_z));
            moved = true;
            break;
        }

        if !moved {
            continue;
        }
    }

    set_corn_tile_open(&mut open, CORN_MAZE_TILES - 2, CORN_MAZE_TILES - 2);
    open
}

fn shuffled_maze_dirs(x: usize, z: usize) -> [(isize, isize); 4] {
    let mut dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let mut seed = (x as u64 + 1).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (z as u64 + 1).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ 0xC0A1_FE1D_5EED;
    for index in 0..dirs.len() {
        seed ^= seed.wrapping_shl(13);
        seed ^= seed.wrapping_shr(7);
        seed ^= seed.wrapping_shl(17);
        let swap = index + (seed as usize % (dirs.len() - index));
        dirs.swap(index, swap);
    }
    dirs
}

fn corn_tile_open(open: &[bool], x: usize, z: usize) -> bool {
    open[z * CORN_MAZE_TILES + x]
}

fn should_place_corn_stalk(tile_x: usize, tile_z: usize, local_x: i32, local_z: i32) -> bool {
    if local_x == 0
        || local_z == 0
        || local_x == CORN_MAZE_TILE_SIZE - 1
        || local_z == CORN_MAZE_TILE_SIZE - 1
    {
        return (local_x + local_z).rem_euclid(2) == 0;
    }

    local_x.rem_euclid(3) == (tile_z as i32).rem_euclid(3)
        && local_z.rem_euclid(3) == (tile_x as i32).rem_euclid(3)
}

fn corn_stalk_height(tile_x: usize, tile_z: usize, x: i32, z: i32) -> i32 {
    let hash = ((tile_x as u64 + 3).wrapping_mul(0x517C_C1B7))
        ^ ((tile_z as u64 + 5).wrapping_mul(0xA24B_AED5))
        ^ ((x as i64 as u64).wrapping_mul(0x9E37_79B1))
        ^ ((z as i64 as u64).wrapping_mul(0x85EB_CA77));
    CORN_STALK_BASE_HEIGHT + (hash % 3) as i32
}

fn stamp_corn_stalk(world: &mut VoxelWorld, x: i32, z: i32, height: i32) {
    for y in 1..=height {
        world.set(
            VoxelCoord::new(x, y, z),
            VoxelCell::new(VoxelMaterial::CornStalk),
        );
    }

    for (index, y) in [4, 7, 10, 13].into_iter().enumerate() {
        if y >= height {
            continue;
        }

        let [(dx_a, dz_a), (dx_b, dz_b)] = corn_leaf_offsets(x + index as i32, z);
        for length in 1..=2 {
            for (dx, dz) in [(dx_a, dz_a), (dx_b, dz_b)] {
                world.set(
                    VoxelCoord::new(x + dx * length, y, z + dz * length),
                    VoxelCell::new(VoxelMaterial::CarbonLife),
                );
            }
        }
    }

    world.set(
        VoxelCoord::new(x, height + 1, z),
        VoxelCell::new(VoxelMaterial::CarbonLife),
    );
    world.set(
        VoxelCoord::new(x, height + 2, z),
        VoxelCell::new(VoxelMaterial::CornStalk),
    );
}

fn corn_leaf_offsets(x: i32, z: i32) -> [(i32, i32); 2] {
    if (x + z).rem_euclid(2) == 0 {
        [(-1, 0), (1, 0)]
    } else {
        [(0, -1), (0, 1)]
    }
}

fn set_corn_tile_open(open: &mut [bool], x: usize, z: usize) {
    open[z * CORN_MAZE_TILES + x] = true;
}

fn corn_maze_half_extent() -> i32 {
    (CORN_MAZE_TILES as i32 * CORN_MAZE_TILE_SIZE) / 2
}

fn corn_tile_center(tile_x: usize, tile_z: usize) -> Vec3 {
    let half = corn_maze_half_extent();
    let center_offset = CORN_MAZE_TILE_SIZE as f32 * 0.5;
    Vec3::new(
        tile_x as f32 * CORN_MAZE_TILE_SIZE as f32 - half as f32 + center_offset,
        0.0,
        tile_z as f32 * CORN_MAZE_TILE_SIZE as f32 - half as f32 + center_offset,
    )
}

fn corn_tile_from_world(position: Vec3) -> Option<(usize, usize)> {
    let half = corn_maze_half_extent();
    let tile_x = ((position.x.floor() as i32 + half).div_euclid(CORN_MAZE_TILE_SIZE)) as isize;
    let tile_z = ((position.z.floor() as i32 + half).div_euclid(CORN_MAZE_TILE_SIZE)) as isize;
    if tile_x < 0
        || tile_z < 0
        || tile_x >= CORN_MAZE_TILES as isize
        || tile_z >= CORN_MAZE_TILES as isize
    {
        return None;
    }

    Some((tile_x as usize, tile_z as usize))
}

fn stamp_exit_marker(world: &mut VoxelWorld, position: Vec3) {
    let origin = VoxelCoord::new(position.x.floor() as i32, 0, position.z.floor() as i32);
    for y in 1..=9 {
        world.set(
            VoxelCoord::new(origin.x, y, origin.z),
            VoxelCell::new(VoxelMaterial::Beacon),
        );
    }
    for (x, z) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        world.set(
            VoxelCoord::new(origin.x + x, 1, origin.z + z),
            VoxelCell::new(VoxelMaterial::Glass),
        );
    }
}

fn city_world_with_figures(base: &VoxelWorld, figures: &CityFigureState) -> VoxelWorld {
    let mut world = base.clone();
    for figure in &figures.figures {
        stamp_city_figure(&mut world, figure);
    }
    world
}

fn shooter_world_with_enemies(base: &VoxelWorld, shooter: &ShooterState) -> VoxelWorld {
    let mut world = base.clone();
    for enemy in &shooter.enemies {
        if enemy.is_alive() {
            stamp_shooter_enemy(&mut world, enemy);
        }
    }
    world
}

fn stamp_city_figure(world: &mut VoxelWorld, figure: &CityFigure) {
    let accent = if figure.watching_player {
        VoxelMaterial::Beacon
    } else {
        VoxelMaterial::Glass
    };
    stamp_npc_body(world, figure.position, VoxelMaterial::CarbonLife, accent);
}

fn stamp_shooter_enemy(world: &mut VoxelWorld, enemy: &Enemy) {
    stamp_npc_body(
        world,
        enemy.position,
        VoxelMaterial::SiliconLife,
        VoxelMaterial::Beacon,
    );
}

fn stamp_npc_body(
    world: &mut VoxelWorld,
    position: Vec3,
    body: VoxelMaterial,
    accent: VoxelMaterial,
) {
    let origin = VoxelCoord::new(position.x.floor() as i32, 0, position.z.floor() as i32);
    for (x, y, z, is_accent) in NPC_BODY_OFFSETS {
        let material = if is_accent { accent } else { body };
        world.set(
            VoxelCoord::new(origin.x + x, origin.y + y, origin.z + z),
            VoxelCell::new(material),
        );
    }
}

fn npc_body_contains_voxel(position: Vec3, coord: VoxelCoord) -> bool {
    let origin = VoxelCoord::new(position.x.floor() as i32, 0, position.z.floor() as i32);
    NPC_BODY_OFFSETS
        .iter()
        .any(|(x, y, z, _)| VoxelCoord::new(origin.x + *x, origin.y + *y, origin.z + *z) == coord)
}

fn build_menu_scene(tick: u64) -> Scene {
    let mut scene = Scene::new(VIEWPORT);
    let mut background = Layer {
        name: "menu".to_string(),
        z: 0,
        cells: Vec::with_capacity(VIEWPORT.width * VIEWPORT.height),
    };

    for y in 0..VIEWPORT.height {
        for x in 0..VIEWPORT.width {
            let glyph = if x == 0 || y == 0 || x == VIEWPORT.width - 1 || y == VIEWPORT.height - 1 {
                '#'
            } else if (x + y + tick as usize / 18) % 37 == 0 {
                '.'
            } else {
                ' '
            };
            background.cells.push(SceneCell {
                x: x as i32,
                y: y as i32,
                glyph,
                style: TextStyle::default(),
            });
        }
    }

    scene.layers.push(background);
    scene.overlays.push(Overlay {
        x: 57,
        y: 30,
        z: 10,
        text: "HELIOBOUND".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 37,
        z: 10,
        text: "1  PLANET FLIGHT".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 41,
        z: 10,
        text: "2  CITY WALK".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 45,
        z: 10,
        text: "3  DOOMLIKE ARENA".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 49,
        z: 10,
        text: "4  CORN MAZE".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 53,
        z: 10,
        text: "5  STARHUSK BAR".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 60,
        z: 10,
        text: "WASD MOVE   CLICK/SPACE FIRE   M MENU".to_string(),
        style: TextStyle::default(),
    });
    scene
}

fn render_city_walk_scene(scene: &mut Scene, figures: &CityFigureState, mouse_captured: bool) {
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "CITY WALK  figures {} watching {}  mouse {}  M menu",
            figures.figures.len(),
            figures.watching_count(),
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
}

fn render_bar_scene(scene: &mut Scene, mouse_captured: bool) {
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "STARHUSK BAR  granular voxel crowd  mouse {}  M menu",
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
}

fn render_corn_maze_scene(
    scene: &mut Scene,
    maze: &CornMazeState,
    camera: &Camera,
    mouse_captured: bool,
) {
    scene.layers.push(Layer {
        name: "minimap".to_string(),
        z: 80,
        cells: corn_minimap_cells(scene.viewport, maze, camera),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "CORN MAZE  {}  mouse {}  M menu",
            if maze.escaped {
                "exit found"
            } else {
                "find the beacon"
            },
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
}

fn corn_minimap_cells(viewport: Viewport, maze: &CornMazeState, camera: &Camera) -> Vec<SceneCell> {
    let Some((player_x, player_z)) = corn_tile_from_world(camera.position) else {
        return Vec::new();
    };
    let Some((exit_x, exit_z)) = corn_tile_from_world(maze.exit_position) else {
        return Vec::new();
    };

    let diameter = CORN_MINIMAP_RADIUS * 2 + 1;
    let origin_x = viewport.width as i32 - diameter - 3;
    let origin_y = 3;
    let mut cells = Vec::new();

    for dz in -CORN_MINIMAP_RADIUS..=CORN_MINIMAP_RADIUS {
        for dx in -CORN_MINIMAP_RADIUS..=CORN_MINIMAP_RADIUS {
            let distance_sq = dx * dx + dz * dz;
            if distance_sq > CORN_MINIMAP_RADIUS * CORN_MINIMAP_RADIUS {
                continue;
            }

            let map_x = player_x as isize + dx as isize;
            let map_z = player_z as isize + dz as isize;
            let glyph = if dx == 0 && dz == 0 {
                player_minimap_glyph(camera)
            } else if map_x == exit_x as isize && map_z == exit_z as isize {
                'X'
            } else if distance_sq >= (CORN_MINIMAP_RADIUS - 1) * (CORN_MINIMAP_RADIUS - 1) {
                'O'
            } else if map_x < 0
                || map_z < 0
                || map_x >= CORN_MAZE_TILES as isize
                || map_z >= CORN_MAZE_TILES as isize
            {
                ' '
            } else if corn_tile_open(&maze.open_tiles, map_x as usize, map_z as usize) {
                '.'
            } else {
                '#'
            };

            if glyph != ' ' {
                cells.push(SceneCell {
                    x: origin_x + dx + CORN_MINIMAP_RADIUS,
                    y: origin_y + dz + CORN_MINIMAP_RADIUS,
                    glyph,
                    style: TextStyle::default(),
                });
            }
        }
    }

    cells
}

fn player_minimap_glyph(camera: &Camera) -> char {
    let forward = horizontal(camera.forward());
    if forward.x.abs() > forward.z.abs() {
        if forward.x >= 0.0 {
            '>'
        } else {
            '<'
        }
    } else if forward.z >= 0.0 {
        'v'
    } else {
        '^'
    }
}

fn render_shooter_scene(
    scene: &mut Scene,
    camera: &Camera,
    shooter: &ShooterState,
    mouse_captured: bool,
) {
    scene.layers.push(Layer {
        name: "bullets".to_string(),
        z: 35,
        cells: bullet_cells(scene.viewport, camera, &shooter.bullet_traces),
    });
    scene.layers.push(Layer {
        name: "weapon".to_string(),
        z: 40,
        cells: weapon_cells(scene.viewport, shooter.shot_flash_timer > 0.0),
    });
    scene.layers.push(Layer {
        name: "reticle".to_string(),
        z: 50,
        cells: reticle_cells(scene.viewport),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "DOOMLIKE ARENA  health {}  enemies {}  kills {}  mouse {}",
            shooter.health,
            shooter.alive_count(),
            shooter.kills,
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Projection {
    x: i32,
    y: i32,
    distance: f32,
}

fn project_world_point(camera: &Camera, point: Vec3, viewport: Viewport) -> Option<Projection> {
    let delta = point - camera.position;
    let forward = delta.dot(camera.forward());
    if forward <= 0.1 {
        return None;
    }

    let aspect = viewport.width.max(1) as f32 / viewport.height.max(1) as f32;
    let tan_half_fov = (camera.fov_y_radians * 0.5).tan();
    let sensor_x = delta.dot(camera.right()) / forward;
    let sensor_y = delta.dot(camera.up()) / forward;
    let normalized_x = sensor_x / (aspect * tan_half_fov);
    let normalized_y = sensor_y / tan_half_fov;

    if !(-1.1..=1.1).contains(&normalized_x) || !(-1.1..=1.1).contains(&normalized_y) {
        return None;
    }

    Some(Projection {
        x: (((normalized_x + 1.0) * 0.5) * viewport.width as f32).round() as i32,
        y: (((1.0 - normalized_y) * 0.5) * viewport.height as f32).round() as i32,
        distance: delta.length(),
    })
}

fn bullet_cells(viewport: Viewport, camera: &Camera, traces: &[BulletTrace]) -> Vec<SceneCell> {
    let mut cells = Vec::new();

    for trace in traces {
        let progress = 1.0 - (trace.time_left / BULLET_TRACE_TIME).clamp(0.0, 1.0);
        let near = 4.0 + progress * 14.0;
        let far = 34.0 + progress * 46.0;

        for (step, distance) in evenly_spaced(near, far, 8).into_iter().enumerate() {
            let point = trace.origin + trace.direction * distance;
            if let Some(projection) = project_world_point(camera, point, viewport) {
                cells.push(SceneCell {
                    x: projection.x,
                    y: projection.y,
                    glyph: bullet_glyph(step, projection.distance),
                    style: TextStyle::default(),
                });
            }
        }
    }

    cells
}

fn evenly_spaced(start: f32, end: f32, count: usize) -> Vec<f32> {
    if count <= 1 {
        return vec![start];
    }

    (0..count)
        .map(|index| {
            let t = index as f32 / (count - 1) as f32;
            start + (end - start) * t
        })
        .collect()
}

fn bullet_glyph(step: usize, distance: f32) -> char {
    if step == 0 || distance < 12.0 {
        '*'
    } else if step % 2 == 0 {
        '+'
    } else {
        '.'
    }
}

fn reticle_cells(viewport: Viewport) -> Vec<SceneCell> {
    let center_x = viewport.width as i32 / 2;
    let center_y = viewport.height as i32 / 2;
    [
        (-2, 0, '-'),
        (-1, 0, '-'),
        (0, 0, '+'),
        (1, 0, '-'),
        (2, 0, '-'),
        (0, -1, '|'),
        (0, 1, '|'),
    ]
    .into_iter()
    .map(|(x, y, glyph)| SceneCell {
        x: center_x + x,
        y: center_y + y,
        glyph,
        style: TextStyle::default(),
    })
    .collect()
}

fn weapon_cells(viewport: Viewport, flash: bool) -> Vec<SceneCell> {
    let art = [
        "      ||      ",
        "     /##\\     ",
        " ___/####\\___ ",
        "|___  ##  ___|",
        "    |####|    ",
    ];
    let start_x = (viewport.width as i32 - art[0].len() as i32) / 2;
    let start_y = viewport.height as i32 - art.len() as i32 - 2;
    let mut cells = Vec::new();

    if flash {
        for (x, y, glyph) in [(0, -3, '*'), (-1, -2, '+'), (1, -2, '+'), (0, -1, '*')] {
            cells.push(SceneCell {
                x: start_x + art[0].len() as i32 / 2 + x,
                y: start_y + y,
                glyph,
                style: TextStyle::default(),
            });
        }
    }

    for (row, line) in art.iter().enumerate() {
        for (col, glyph) in line.chars().enumerate() {
            if glyph == ' ' {
                continue;
            }
            cells.push(SceneCell {
                x: start_x + col as i32,
                y: start_y + row as i32,
                glyph,
                style: TextStyle::default(),
            });
        }
    }

    cells
}

fn render_scene(scene: &Scene, frame: &mut [u8], width: usize, height: usize) {
    clear(frame, [0x08, 0x0b, 0x10, 0xff]);

    let mut scene = scene.clone();
    scene.sort_layers();

    for layer in &scene.layers {
        let color = match layer.name.as_str() {
            "background" => [0x50, 0x58, 0x66, 0xff],
            "menu" => [0x78, 0xc6, 0xa3, 0xff],
            "voxels" => [0xdf, 0xe8, 0xdb, 0xff],
            "planet" => [0xdf, 0xe8, 0xdb, 0xff],
            "enemies" => [0xff, 0x65, 0x5a, 0xff],
            "bullets" => [0xff, 0xea, 0x8a, 0xff],
            "weapon" => [0xf0, 0xc6, 0x5b, 0xff],
            "reticle" => [0x9f, 0xf5, 0xff, 0xff],
            "minimap" => [0x9f, 0xf5, 0xff, 0xff],
            _ => [0xe6, 0xee, 0xf3, 0xff],
        };

        for cell in &layer.cells {
            draw_glyph(frame, width, height, cell.x, cell.y, cell.glyph, color);
        }
    }

    for overlay in &scene.overlays {
        draw_text(
            frame,
            width,
            height,
            overlay.x,
            overlay.y,
            &overlay.text,
            [0xf0, 0xc6, 0x5b, 0xff],
        );
    }
}

fn clear(frame: &mut [u8], color: [u8; 4]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
}

fn draw_text(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
) {
    for (offset, ch) in text.chars().enumerate() {
        draw_glyph(frame, width, height, x + offset as i32, y, ch, color);
    }
}

fn draw_glyph(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    ch: char,
    color: [u8; 4],
) {
    let glyph = BASIC_FONTS.get(ch).or_else(|| BASIC_FONTS.get('?'));
    let Some(glyph) = glyph else {
        return;
    };

    let base_x = x * CHAR_WIDTH as i32;
    let base_y = y * CHAR_HEIGHT as i32;

    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..8 {
            let mask = 1u8 << col;
            if bits & mask != 0 {
                let px = base_x + col as i32;
                let py = base_y + row as i32;
                set_pixel(frame, width, height, px, py, color);
            }
        }
    }
}

fn set_pixel(frame: &mut [u8], width: usize, height: usize, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= width || y >= height {
        return;
    }
    let idx = (y * width + x) * 4;
    frame[idx..idx + 4].copy_from_slice(&color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_look_is_roll_relative() {
        let mut camera = Camera::new(Vec3::ZERO).with_roll(std::f32::consts::FRAC_PI_2);

        apply_mouse_look(&mut camera, 10.0, 0.0, PitchMode::Unrestricted);

        assert!(camera.forward().y > 0.0);
        assert!(camera.forward().x.abs() < 0.001);
    }

    #[test]
    fn space_mouse_look_allows_full_pitch_rotation() {
        let mut camera = Camera::new(Vec3::ZERO);

        apply_mouse_look(&mut camera, 0.0, -1_000.0, PitchMode::Unrestricted);

        assert!(camera.pitch_radians.abs() > PITCH_LIMIT);
        assert!(camera.pitch_radians <= std::f32::consts::PI);
        assert!(camera.pitch_radians >= -std::f32::consts::PI);
    }

    #[test]
    fn space_mouse_look_keeps_horizontal_direction_after_overhead_flip() {
        let mut camera = Camera::new(Vec3::ZERO);

        apply_mouse_look(&mut camera, 0.0, -800.0, PitchMode::Unrestricted);
        apply_mouse_look(&mut camera, 20.0, 0.0, PitchMode::Unrestricted);

        assert!(camera.forward().x > 0.0);
        assert!(camera.right().dot(Vec3::new(1.0, 0.0, 0.0)) > 0.99);
    }

    #[test]
    fn planet_start_altitude_is_decoupled_from_planet_radius() {
        let camera = planet_start_camera();
        let envelope_radius = PLANET_RADIUS as f32 + PLANET_TERRAIN_AMPLITUDE;
        let altitude = camera.position.length() - envelope_radius;

        assert!(altitude > PLANET_START_ALTITUDE);
        assert!(altitude < PLANET_START_ALTITUDE + 10.0);
    }

    #[test]
    fn flight_speed_stays_local_scale_when_planet_scales() {
        assert_eq!(FLIGHT_SPEED, 12_000.0);
        assert!(PLANET_RADIUS as f32 / FLIGHT_SPEED > 3_000.0);
    }

    #[test]
    fn walking_mouse_look_keeps_pitch_clamped() {
        let mut camera = Camera::new(Vec3::ZERO);

        apply_mouse_look(&mut camera, 0.0, -10_000.0, PitchMode::Clamped);

        assert_eq!(camera.pitch_radians, PITCH_LIMIT);
    }

    #[test]
    fn walking_movement_stays_on_eye_height() {
        let city = build_demo_city();
        let mut camera = city_start_camera();
        let input = PlayerInput {
            forward: true,
            ..PlayerInput::default()
        };

        update_walking_camera(&mut camera, &input, &city, 1.0);

        assert_eq!(camera.position.y, WALK_EYE_HEIGHT);
    }

    #[test]
    fn walking_collision_slides_along_walls() {
        let mut world = VoxelWorld::new();
        for y in 1..=WALK_EYE_HEIGHT.ceil() as i32 {
            world.set(
                VoxelCoord::new(1, y, 0),
                VoxelCell::new(VoxelMaterial::Basalt),
            );
            world.set(
                VoxelCoord::new(1, y, 1),
                VoxelCell::new(VoxelMaterial::Basalt),
            );
        }
        let start = Vec3::new(0.5, WALK_EYE_HEIGHT, 0.5);

        let moved = move_walking_with_collision(
            start,
            Vec3::new(1.0, 0.0, 0.5),
            &world,
            STANDARD_WALK_PROFILE,
        );

        assert_eq!(moved.x, start.x);
        assert!(moved.z > start.z);
    }

    #[test]
    fn walking_collision_blocks_when_both_slide_axes_are_blocked() {
        let mut world = VoxelWorld::new();
        for y in 1..=WALK_EYE_HEIGHT.ceil() as i32 {
            world.set(
                VoxelCoord::new(1, y, 0),
                VoxelCell::new(VoxelMaterial::Basalt),
            );
            world.set(
                VoxelCoord::new(0, y, 1),
                VoxelCell::new(VoxelMaterial::Basalt),
            );
            world.set(
                VoxelCoord::new(1, y, 1),
                VoxelCell::new(VoxelMaterial::Basalt),
            );
        }
        let start = Vec3::new(0.5, WALK_EYE_HEIGHT, 0.5);

        let moved = move_walking_with_collision(
            start,
            Vec3::new(1.0, 0.0, 1.0),
            &world,
            STANDARD_WALK_PROFILE,
        );

        assert_eq!(moved, start);
    }

    #[test]
    fn menu_can_start_city_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit2), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::CityWalk);
    }

    #[test]
    fn city_figure_moves_when_not_watching_player() {
        let city = build_demo_city();
        let camera = Camera::new(Vec3::new(60.5, WALK_EYE_HEIGHT, 60.5)).looking_at(0.0, 0.0);
        let mut figure =
            CityFigure::new(vec![Vec3::new(0.5, 0.0, -48.5), Vec3::new(0.5, 0.0, -32.5)]);
        let before = figure.position;

        figure.watching_player = figure.is_looking_at_player(&city, &camera);
        figure.advance(&city, 1.0);

        assert!(!figure.watching_player);
        assert_ne!(figure.position, before);
    }

    #[test]
    fn city_figure_stops_when_it_is_watching_player() {
        let city = build_demo_city();
        let camera = city_start_camera();
        let mut figures = CityFigureState {
            figures: vec![CityFigure::new(vec![
                Vec3::new(0.5, 0.0, -48.5),
                Vec3::new(0.5, 0.0, -64.5),
            ])],
        };
        let before = figures.figures[0].position;

        figures.update(&city, &camera, 1.0);

        assert!(figures.figures[0].watching_player);
        assert_eq!(figures.figures[0].position, before);
    }

    #[test]
    fn city_figures_are_inserted_as_voxel_objects() {
        let mut app = AppState::new();
        app.start_city();
        app.city_figures = CityFigureState {
            figures: vec![CityFigure::new(vec![
                Vec3::new(0.5, 0.0, -44.5),
                Vec3::new(0.5, 0.0, -32.5),
            ])],
        };

        let world = city_world_with_figures(&app.city, &app.city_figures);

        assert_eq!(
            world.get(VoxelCoord::new(0, 3, -45)),
            Some(VoxelCell::new(VoxelMaterial::CarbonLife))
        );
        assert_eq!(
            world.get(VoxelCoord::new(-1, 3, -46)),
            Some(VoxelCell::new(VoxelMaterial::Glass))
        );
    }

    #[test]
    fn npc_eye_heights_match_player_eye_height() {
        assert_eq!(CITY_FIGURE_EYE_HEIGHT, WALK_EYE_HEIGHT);
        assert_eq!(ENEMY_EYE_HEIGHT, WALK_EYE_HEIGHT);
    }

    #[test]
    fn city_figure_is_hit_by_voxel_raycast() {
        let mut app = AppState::new();
        app.start_city();
        app.city_figures = CityFigureState {
            figures: vec![CityFigure::new(vec![
                Vec3::new(0.5, 0.0, -44.5),
                Vec3::new(0.5, 0.0, -32.5),
            ])],
        };
        let world = city_world_with_figures(&app.city, &app.city_figures);

        let hit = raycast(
            &world,
            Ray::new(app.camera.position, app.camera.forward()),
            20.0,
        )
        .expect("center ray should hit city figure object");

        assert!(matches!(
            hit.cell.material,
            VoxelMaterial::CarbonLife | VoxelMaterial::Glass | VoxelMaterial::Beacon
        ));
    }

    #[test]
    fn menu_can_start_city_shooter_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit3), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::CityShooter);
    }

    #[test]
    fn menu_can_start_corn_maze_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit4), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::CornMaze);
    }

    #[test]
    fn menu_can_start_bar_scene_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit5), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::BarScene);
        assert_eq!(app.camera.position.y, BAR_EYE_HEIGHT);
    }

    #[test]
    fn bar_scene_contains_granular_people_and_props() {
        let bar = build_bar_scene();

        assert!(bar.voxel_count() > 20_000);
        assert!(BAR_EYE_HEIGHT > WALK_EYE_HEIGHT * 3.0);
        assert_eq!(
            bar.get(VoxelCoord::new(-36, 12, 27)),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
        assert_eq!(
            bar.get(VoxelCoord::new(40, 9, -31)),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
        assert_eq!(
            bar.get(VoxelCoord::new(-30, 7, -22)),
            Some(VoxelCell::new(VoxelMaterial::Glass))
        );
        assert_eq!(
            bar.get(VoxelCoord::new(-62, 8, -34)),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
        assert_eq!(
            bar.get(VoxelCoord::new(-70, 10, 8)),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
    }

    #[test]
    fn corn_maze_contains_tall_corn_and_exit_marker() {
        let maze = CornMazeState::new();
        let wall_base = -corn_maze_half_extent();

        assert!(maze.world.voxel_count() > 4_000);
        assert!(CORN_MAZE_TILE_SIZE > 10);
        assert!(CORN_WALK_EYE_HEIGHT > WALK_EYE_HEIGHT * 2.0);
        assert!(maze
            .world
            .get(VoxelCoord::new(wall_base, 8, wall_base))
            .is_some_and(|cell| cell.material == VoxelMaterial::CornStalk));
        assert_eq!(
            maze.world.get(VoxelCoord::new(wall_base + 1, 8, wall_base)),
            None
        );
        assert!(maze
            .world
            .get(VoxelCoord::new(wall_base + 2, 10, wall_base))
            .is_some_and(|cell| cell.material == VoxelMaterial::CarbonLife));
        assert!(maze
            .world
            .get(VoxelCoord::new(
                maze.exit_position.x.floor() as i32,
                8,
                maze.exit_position.z.floor() as i32,
            ))
            .is_some_and(|cell| cell.material == VoxelMaterial::Beacon));
    }

    #[test]
    fn corn_maze_start_is_walkable() {
        let maze = CornMazeState::new();

        assert!(can_walk_to_with_profile(
            &maze.world,
            Vec3::new(
                maze.start_position.x,
                CORN_WALK_EYE_HEIGHT,
                maze.start_position.z
            ),
            CORN_WALK_PROFILE
        ));
    }

    #[test]
    fn corn_maze_corn_is_hit_by_voxel_raycast() {
        let maze = CornMazeState::new();
        let half = corn_maze_half_extent();
        let camera = look_at(
            Vec3::new(
                maze.start_position.x,
                CORN_WALK_EYE_HEIGHT,
                maze.start_position.z,
            ),
            Vec3::new(
                -half as f32 + 0.5,
                CORN_WALK_EYE_HEIGHT,
                maze.start_position.z + 0.5,
            ),
        );

        let hit = raycast(
            &maze.world,
            Ray::new(camera.position, camera.forward()),
            40.0,
        )
        .expect("ray should hit nearby corn wall");

        assert_eq!(hit.cell.material, VoxelMaterial::CornStalk);
    }

    #[test]
    fn corn_maze_marks_exit_when_player_reaches_beacon() {
        let mut maze = CornMazeState::new();

        maze.update(Vec3::new(
            maze.exit_position.x + 1.0,
            CORN_WALK_EYE_HEIGHT,
            maze.exit_position.z,
        ));

        assert!(maze.escaped);
    }

    #[test]
    fn corn_maze_maps_world_position_to_tile() {
        let maze = CornMazeState::new();

        assert_eq!(corn_tile_from_world(maze.start_position), Some((1, 1)));
        assert_eq!(
            corn_tile_from_world(maze.exit_position),
            Some((CORN_MAZE_TILES - 2, CORN_MAZE_TILES - 2))
        );
    }

    #[test]
    fn corn_maze_minimap_draws_player_walls_and_exit_when_visible() {
        let maze = CornMazeState::new();
        let camera = look_at(
            Vec3::new(
                maze.exit_position.x - CORN_MAZE_TILE_SIZE as f32,
                CORN_WALK_EYE_HEIGHT,
                maze.exit_position.z,
            ),
            maze.exit_position,
        )
        .with_fov_y(64.0_f32.to_radians());

        let cells = corn_minimap_cells(VIEWPORT, &maze, &camera);

        assert!(cells
            .iter()
            .any(|cell| ['<', '>', '^', 'v'].contains(&cell.glyph)));
        assert!(cells.iter().any(|cell| cell.glyph == '#'));
        assert!(cells.iter().any(|cell| cell.glyph == '.'));
        assert!(cells.iter().any(|cell| cell.glyph == 'X'));
    }

    #[test]
    fn corn_maze_scene_draws_minimap_layer() {
        let mut app = AppState::new();
        app.start_corn_maze();

        let scene = app.frame(0.0, true);

        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "minimap" && !layer.cells.is_empty()));
    }

    #[test]
    fn shooter_fire_damages_centered_enemy() {
        let mut app = AppState::new();
        app.start_shooter();

        let health_before = app.shooter.enemies[0].health;
        app.fire_weapon();

        assert!(app.shooter.enemies[0].health < health_before);
        assert_eq!(app.shooter.shots_fired, 1);
    }

    #[test]
    fn shooter_fire_uses_enemy_voxel_hitbox() {
        let mut app = AppState::new();
        app.start_shooter();
        let target = Vec3::new(-0.5, ENEMY_EYE_HEIGHT, -35.5);
        app.camera = look_at(app.camera.position, target)
            .with_fov_y(68.0_f32.to_radians())
            .with_max_distance(140.0);

        let health_before = app.shooter.enemies[0].health;
        app.fire_weapon();

        assert!(app.shooter.enemies[0].health < health_before);
    }

    #[test]
    fn shooter_fire_ignores_non_enemy_voxel_hits() {
        let mut app = AppState::new();
        app.start_shooter();
        app.camera = look_at(app.camera.position, Vec3::new(12.0, 3.0, -42.0))
            .with_fov_y(68.0_f32.to_radians())
            .with_max_distance(140.0);

        let health_before = app.shooter.enemies[0].health;
        app.fire_weapon();

        assert_eq!(app.shooter.enemies[0].health, health_before);
        assert_eq!(app.drain_audio_events(), vec![SoundEffect::Gunshot]);
    }

    #[test]
    fn shooter_fire_queues_gunshot_and_hit_audio() {
        let mut app = AppState::new();
        app.start_shooter();

        app.fire_weapon();
        let events = app.drain_audio_events();

        assert_eq!(events, vec![SoundEffect::Gunshot, SoundEffect::EnemyHit]);
    }

    #[test]
    fn shooter_kill_queues_death_audio() {
        let mut app = AppState::new();
        app.start_shooter();

        app.fire_weapon();
        app.drain_audio_events();
        app.fire_weapon();
        let events = app.drain_audio_events();

        assert_eq!(events, vec![SoundEffect::Gunshot, SoundEffect::EnemyDeath]);
        assert_eq!(app.shooter.kills, 1);
    }

    #[test]
    fn shooter_enemy_attack_queues_player_hurt_audio() {
        let mut app = AppState::new();
        app.start_shooter();
        app.shooter.enemies[0].position = app.camera.position + Vec3::new(0.5, 0.0, 0.0);

        app.frame(0.016, true);
        let events = app.drain_audio_events();

        assert_eq!(events, vec![SoundEffect::PlayerHurt]);
        assert!(app.shooter.health < 100);
    }

    #[test]
    fn shooter_uses_doom_map_not_city_grid() {
        let mut app = AppState::new();
        app.start_shooter();

        assert!(app.doom_map.get(VoxelCoord::new(0, 7, 0)).is_some());
        assert!(app.city.get(VoxelCoord::new(0, 7, 0)).is_none());
        assert!(app.doom_map.get(VoxelCoord::new(0, 1, -42)).is_none());
    }

    #[test]
    fn walking_collision_blocks_doom_arena_walls() {
        let map = build_doom_map();

        assert!(!can_walk_to(&map, Vec3::new(-64.0, WALK_EYE_HEIGHT, 0.0)));
        assert!(can_walk_to(&map, Vec3::new(0.5, WALK_EYE_HEIGHT, -55.5)));
    }

    #[test]
    fn shooter_scene_draws_weapon_with_enemies_in_voxel_world() {
        let mut app = AppState::new();
        app.start_shooter();

        let scene = app.frame(0.0, true);

        assert!(!scene.layers.iter().any(|layer| layer.name == "enemies"));
        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "weapon" && !layer.cells.is_empty()));
    }

    #[test]
    fn shooter_enemies_are_inserted_as_voxel_objects() {
        let mut app = AppState::new();
        app.start_shooter();
        let world = shooter_world_with_enemies(&app.doom_map, &app.shooter);

        assert_eq!(
            world.get(VoxelCoord::new(0, 3, -35)),
            Some(VoxelCell::new(VoxelMaterial::SiliconLife))
        );
        assert_eq!(
            world.get(VoxelCoord::new(-1, 3, -36)),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
    }

    #[test]
    fn shooter_enemy_is_hit_by_voxel_raycast() {
        let mut app = AppState::new();
        app.start_shooter();
        let world = shooter_world_with_enemies(&app.doom_map, &app.shooter);

        let hit = raycast(
            &world,
            Ray::new(app.camera.position, app.camera.forward()),
            30.0,
        )
        .expect("center ray should hit shooter enemy object");

        assert!(matches!(
            hit.cell.material,
            VoxelMaterial::SiliconLife | VoxelMaterial::Beacon
        ));
    }

    #[test]
    fn shooter_scene_draws_center_reticle() {
        let mut app = AppState::new();
        app.start_shooter();

        let scene = app.frame(0.0, true);
        let reticle = scene
            .layers
            .iter()
            .find(|layer| layer.name == "reticle")
            .expect("shooter scene should include reticle layer");

        assert!(reticle.cells.iter().any(|cell| {
            cell.x == VIEWPORT.width as i32 / 2
                && cell.y == VIEWPORT.height as i32 / 2
                && cell.glyph == '+'
        }));
    }

    #[test]
    fn shooter_fire_draws_bullet_tracer() {
        let mut app = AppState::new();
        app.start_shooter();
        app.fire_weapon();

        let scene = app.frame(0.0, true);
        let bullets = scene
            .layers
            .iter()
            .find(|layer| layer.name == "bullets")
            .expect("shooter scene should include bullet layer");

        assert!(!bullets.cells.is_empty());
    }

    #[test]
    fn q_and_e_roll_are_reversed_for_flight() {
        let mut q_camera = planet_start_camera();
        let mut e_camera = planet_start_camera();

        update_flight_camera(
            &mut q_camera,
            &PlayerInput {
                roll_left: true,
                ..PlayerInput::default()
            },
            1.0,
        );
        update_flight_camera(
            &mut e_camera,
            &PlayerInput {
                roll_right: true,
                ..PlayerInput::default()
            },
            1.0,
        );

        assert!(q_camera.roll_radians > 0.0);
        assert!(e_camera.roll_radians < 0.0);
    }
}
