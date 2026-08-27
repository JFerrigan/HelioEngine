use std::collections::{HashMap, VecDeque};
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
const ZOMBIE_WALK_SPEED: f32 = 13.0;
const ZOMBIE_SPRINT_MULTIPLIER: f32 = 1.25;
const ZOMBIE_SPRINT_DRAIN: f32 = 0.34;
const ZOMBIE_SPRINT_RECHARGE: f32 = 0.20;
const ZOMBIE_EYE_HEIGHT: f32 = 2.5;
const ZOMBIE_COLLISION_RADIUS: f32 = 0.24;
const ZOMBIE_SPAWN_CLEARANCE_RADIUS: i32 = 2;
const ZOMBIE_ATTACK_RANGE: f32 = 2.1;
const ZOMBIE_ATTACK_COOLDOWN: f32 = 1.05;
const ZOMBIE_MAX_HITS: i32 = 3;
const ZOMBIE_HIT_FLASH_TIME: f32 = 0.32;
const ZOMBIE_ROUND_BREAK_TIME: f32 = 3.0;
const ZOMBIE_SPAWN_INTERVAL: f32 = 0.65;
const ZOMBIE_START_AMMO: i32 = 96;
const ZOMBIE_MAG_SIZE: i32 = 24;
const ZOMBIE_WALL_WEAPON_COST: i32 = 750;
const ZOMBIE_DOOR_COST: i32 = 900;
const ZOMBIE_HIT_POINTS: i32 = 10;
const ZOMBIE_KILL_POINTS: i32 = 100;
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
const ASSET_VIEWER_ROTATE_SPEED: f32 = 1.8;
const ASSET_VIEWER_ROLL_SPEED: f32 = 1.4;
const ASSET_VIEWER_ZOOM_SPEED: f32 = 22.0;
const ASSET_VIEWER_DEFAULT_DISTANCE: f32 = 28.0;
const ASSET_VIEWER_MIN_DISTANCE: f32 = 8.0;
const ASSET_VIEWER_MAX_DISTANCE: f32 = 80.0;
const ASSET_VIEWER_MOUSE_SENSITIVITY: f32 = 0.006;
const WEAPON_VIEW_RENDER_WIDTH: usize = 72;
const WEAPON_VIEW_RENDER_HEIGHT: usize = 40;
const WEAPON_VIEW_SCREEN_RIGHT_MARGIN: i32 = 1;
const WEAPON_VIEW_SCREEN_BOTTOM_MARGIN: i32 = -1;
const WEAPON_VIEW_SCREEN_SHIFT_X: i32 = 0;
const WEAPON_VIEW_SCREEN_SHIFT_Y: i32 = 0;
const WEAPON_VIEW_CAMERA_RIGHT: f32 = 1.25;
const WEAPON_VIEW_CAMERA_UP: f32 = 1.05;
const WEAPON_VIEW_CAMERA_BACK: f32 = 1.75;
const WEAPON_VIEW_TARGET_LEFT: f32 = 0.95;
const WEAPON_VIEW_TARGET_UP: f32 = 0.10;
const WEAPON_VIEW_TARGET_FORWARD: f32 = 0.45;
const WEAPON_VIEW_CAMERA_FOV: f32 = 24.0;
const WEAPON_VIEW_CAMERA_DISTANCE_SCALE: f32 = 4.0;
const SANDBOX_HALF_EXTENT: i32 = 72;
const SANDBOX_EYE_HEIGHT: f32 = 2.7;
const SANDBOX_SPEED: f32 = 9.0;
const SANDBOX_COLLISION_RADIUS: f32 = 0.32;
const SANDBOX_REACH: f32 = 8.0;
const LIMINAL_SEED: u64 = 0xA551_011C_E0FF_1CE5;
const LIMINAL_ROOM_HEIGHT: i32 = 8;
const LIMINAL_HALL_HALF_WIDTH: i32 = 5;
const LIMINAL_INTERACTION_RANGE: f32 = 5.0;
const LIMINAL_WALK_PROFILE: WalkProfile = WalkProfile {
    eye_height: WALK_EYE_HEIGHT,
    speed: 12.0,
    collision_radius: WALK_COLLISION_RADIUS,
};
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
const ZOMBIE_BODY_OFFSETS: [(i32, i32, i32, VoxelMaterial); 18] = [
    (-1, 1, 0, VoxelMaterial::Zombie),
    (0, 1, 0, VoxelMaterial::Zombie),
    (1, 1, 0, VoxelMaterial::Zombie),
    (-1, 2, 0, VoxelMaterial::Zombie),
    (0, 2, 0, VoxelMaterial::Zombie),
    (1, 2, 0, VoxelMaterial::Zombie),
    (-1, 3, 0, VoxelMaterial::Zombie),
    (0, 3, 0, VoxelMaterial::Zombie),
    (1, 3, 0, VoxelMaterial::Zombie),
    (-1, 3, -1, VoxelMaterial::Zombie),
    (1, 3, -1, VoxelMaterial::Zombie),
    (-1, 4, 0, VoxelMaterial::Zombie),
    (0, 4, 0, VoxelMaterial::Beacon),
    (1, 4, 0, VoxelMaterial::Zombie),
    (0, 4, -1, VoxelMaterial::Beacon),
    (-1, 2, -1, VoxelMaterial::Beacon),
    (1, 2, -1, VoxelMaterial::Beacon),
    (0, 5, 0, VoxelMaterial::Beacon),
];
const DRONE_GATE_SEED: u64 = 0xD20A_6A7E_2026_0826;
const DRONE_GATE_VIEW_DISTANCE: f32 = 900.0;
const DRONE_GATE_START_BACK: f32 = 42.0;
const DRONE_GATE_FRAME_RADIUS: i32 = 7;
const DRONE_GATE_INNER_RADIUS: f32 = 5.7;
const DRONE_GATE_TUBE_RADIUS: i32 = 1;
const DRONE_GATE_DEPTH: i32 = 2;
const DRONE_GATE_PASS_DISTANCE: f32 = 7.5;
const DRONE_GATE_RING_SEGMENTS: usize = 32;
const DRONE_GATE_COURSE_WIDTH: f32 = 80.0;
const DRONE_GATE_COURSE_HEIGHT: f32 = 28.0;
const DRONE_GATE_COURSE_SPACING: f32 = 58.0;

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
                    button,
                    ..
                } => {
                    if mouse_captured {
                        app.handle_mouse_button(button);
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
    AssetViewer,
    VoxelSandbox,
    Zombies,
    Liminal,
    DroneGateRunner,
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
    asset_viewer: AssetViewerState,
    sandbox: VoxelSandboxState,
    zombies_map: VoxelWorld,
    zombies: ZombiesState,
    liminal: LiminalState,
    drone_gate_runner: DroneGateRunnerState,
    weapon_asset: PreviewAsset,
    planet_builder: SceneBuilder,
    city_builder: SceneBuilder,
    camera: Camera,
    input: PlayerInput,
    city_figures: CityFigureState,
    shooter: ShooterState,
    viewmodel_bob: ViewmodelBob,
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
            asset_viewer: AssetViewerState::new(),
            sandbox: VoxelSandboxState::new(),
            zombies_map: build_zombies_map(&ZombiesState::new()),
            zombies: ZombiesState::new(),
            liminal: LiminalState::new_seeded(LIMINAL_SEED),
            drone_gate_runner: DroneGateRunnerState::new_seeded(DRONE_GATE_SEED),
            weapon_asset: PreviewAsset::new("gun", build_weapon_asset()),
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
            viewmodel_bob: ViewmodelBob::default(),
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
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit6)) => {
                    self.start_asset_viewer();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit7)) => {
                    self.start_voxel_sandbox();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit8)) => {
                    self.start_zombies();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit9)) => {
                    self.start_liminal();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Digit0)) => {
                    self.start_drone_gate_runner();
                    return KeyboardAction::StartScene;
                }
                (AppMode::AssetViewer, PhysicalKey::Code(KeyCode::KeyM)) => {
                    return KeyboardAction::EnterMenu;
                }
                (AppMode::AssetViewer, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::ReleaseMouse;
                }
                (AppMode::AssetViewer, key) => {
                    if let Some(index) = asset_digit_index(key) {
                        self.asset_viewer.select(index);
                        return KeyboardAction::None;
                    }
                    match key {
                        PhysicalKey::Code(KeyCode::KeyN) | PhysicalKey::Code(KeyCode::Period) => {
                            self.asset_viewer.select_next();
                            return KeyboardAction::None;
                        }
                        PhysicalKey::Code(KeyCode::KeyP) | PhysicalKey::Code(KeyCode::Comma) => {
                            self.asset_viewer.select_previous();
                            return KeyboardAction::None;
                        }
                        _ => {}
                    }
                }
                (AppMode::VoxelSandbox, key) => {
                    if let Some(index) = asset_digit_index(key) {
                        self.sandbox.select_block(index);
                        return KeyboardAction::None;
                    }
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::Exit;
                }
                (AppMode::CityShooter, PhysicalKey::Code(KeyCode::Space)) => {
                    return KeyboardAction::Fire;
                }
                (AppMode::Zombies, PhysicalKey::Code(KeyCode::Space)) => {
                    return KeyboardAction::Fire;
                }
                (AppMode::Zombies, PhysicalKey::Code(KeyCode::KeyR)) => {
                    self.zombies.reload();
                    return KeyboardAction::None;
                }
                (AppMode::Zombies, PhysicalKey::Code(KeyCode::KeyF)) => {
                    self.zombies
                        .interact(&mut self.zombies_map, self.camera.position);
                    return KeyboardAction::None;
                }
                (AppMode::Liminal, PhysicalKey::Code(KeyCode::KeyF)) => {
                    self.liminal.interact(self.camera.position);
                    return KeyboardAction::None;
                }
                (AppMode::Liminal, PhysicalKey::Code(KeyCode::KeyT)) => {
                    self.liminal.force_next_anomaly();
                    return KeyboardAction::None;
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
        self.viewmodel_bob = ViewmodelBob::default();
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

    fn start_asset_viewer(&mut self) {
        self.mode = AppMode::AssetViewer;
        self.asset_viewer = AssetViewerState::new();
        self.camera = self.asset_viewer.camera();
        self.input = PlayerInput::default();
    }

    fn start_voxel_sandbox(&mut self) {
        self.mode = AppMode::VoxelSandbox;
        self.sandbox = VoxelSandboxState::new();
        self.camera = sandbox_start_camera(&self.sandbox.world);
        self.input = PlayerInput::default();
    }

    fn start_zombies(&mut self) {
        self.mode = AppMode::Zombies;
        self.zombies = ZombiesState::new();
        self.zombies_map = build_zombies_map(&self.zombies);
        self.camera = zombies_start_camera();
        self.input = PlayerInput::default();
        self.viewmodel_bob = ViewmodelBob::default();
    }

    fn start_liminal(&mut self) {
        self.mode = AppMode::Liminal;
        self.liminal = LiminalState::new_seeded(LIMINAL_SEED);
        self.camera = liminal_start_camera(&self.liminal);
        self.input = PlayerInput::default();
    }

    fn start_drone_gate_runner(&mut self) {
        self.mode = AppMode::DroneGateRunner;
        self.drone_gate_runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        self.camera = drone_gate_runner_start_camera(&self.drone_gate_runner);
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
                let before_move = self.camera.position;
                update_walking_camera(&mut self.camera, &self.input, &self.doom_map, dt);
                self.viewmodel_bob.update(
                    horizontal_distance(before_move, self.camera.position),
                    moving_on_ground(&self.input),
                    dt,
                );
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
                render_shooter_scene(
                    &mut scene,
                    &self.camera,
                    &self.shooter,
                    &self.weapon_asset,
                    self.viewmodel_bob.offset(),
                    mouse_captured,
                );
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
            AppMode::AssetViewer => {
                self.asset_viewer.update(&self.input, dt);
                self.camera = self.asset_viewer.camera();
                let asset = self.asset_viewer.selected_asset();
                let mut scene = self
                    .city_builder
                    .build(&asset.world, &self.camera, self.tick);
                render_asset_viewer_scene(&mut scene, &self.asset_viewer, mouse_captured);
                scene
            }
            AppMode::VoxelSandbox => {
                update_sandbox_camera(&mut self.camera, &self.input, &self.sandbox.world, dt);
                let mut scene =
                    self.city_builder
                        .build(&self.sandbox.world, &self.camera, self.tick);
                render_voxel_sandbox_scene(&mut scene, &self.sandbox, mouse_captured);
                scene
            }
            AppMode::Zombies => {
                let before_move = self.camera.position;
                self.zombies
                    .update_player(&mut self.camera, &self.input, &self.zombies_map, dt);
                self.viewmodel_bob.update(
                    horizontal_distance(before_move, self.camera.position),
                    moving_on_ground(&self.input),
                    dt,
                );
                self.zombies_map = build_zombies_map(&self.zombies);
                let hurt = self.zombies.update_rounds_and_zombies(
                    &self.zombies_map,
                    self.camera.position,
                    dt,
                );
                if hurt {
                    self.audio_events.push(SoundEffect::PlayerHurt);
                }
                let render_world = zombies_world_with_zombies(&self.zombies_map, &self.zombies);
                let mut scene = self
                    .city_builder
                    .build(&render_world, &self.camera, self.tick);
                render_zombies_scene(
                    &mut scene,
                    &self.camera,
                    &self.zombies,
                    &self.weapon_asset,
                    self.viewmodel_bob.offset(),
                    mouse_captured,
                );
                scene
            }
            AppMode::Liminal => {
                update_walking_camera_with_profile(
                    &mut self.camera,
                    &self.input,
                    &self.liminal.world,
                    LIMINAL_WALK_PROFILE,
                    dt,
                );
                self.liminal.update_player_room(&mut self.camera);
                let mut scene =
                    self.city_builder
                        .build(&self.liminal.world, &self.camera, self.tick);
                render_liminal_scene(&mut scene, &self.liminal, mouse_captured);
                scene
            }
            AppMode::DroneGateRunner => {
                update_flight_camera(&mut self.camera, &self.input, dt);
                self.drone_gate_runner.update(self.camera.position, dt);
                let world = self.drone_gate_runner.render_world();
                let mut scene = self.city_builder.build(&world, &self.camera, self.tick);
                render_drone_gate_runner_scene(&mut scene, &self.drone_gate_runner, mouse_captured);
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
            AppMode::DroneGateRunner => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Unrestricted)
            }
            AppMode::CityWalk
            | AppMode::CityShooter
            | AppMode::CornMaze
            | AppMode::BarScene
            | AppMode::VoxelSandbox
            | AppMode::Zombies
            | AppMode::Liminal => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Clamped)
            }
            AppMode::AssetViewer => self.asset_viewer.rotate_with_mouse(delta_x, delta_y),
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton) {
        match (self.mode, button) {
            (AppMode::CityShooter, MouseButton::Left) => self.fire_weapon(),
            (AppMode::Zombies, MouseButton::Left) => self.fire_weapon(),
            (AppMode::VoxelSandbox, MouseButton::Left) => self.sandbox.remove_block(&self.camera),
            (AppMode::VoxelSandbox, MouseButton::Right) => self.sandbox.place_block(&self.camera),
            _ => {}
        }
    }

    fn fire_weapon(&mut self) {
        if self.mode == AppMode::CityShooter {
            self.audio_events
                .extend(self.shooter.fire(&self.doom_map, &self.camera));
        } else if self.mode == AppMode::Zombies {
            self.audio_events
                .extend(self.zombies.fire(&self.zombies_map, &self.camera));
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
        AppMode::CityShooter | AppMode::Zombies => audio.enter_doom_mode(),
        AppMode::Menu
        | AppMode::PlanetFlight
        | AppMode::AssetViewer
        | AppMode::VoxelSandbox
        | AppMode::Liminal
        | AppMode::DroneGateRunner => audio.leave_ambience(),
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

fn sandbox_start_camera(world: &VoxelWorld) -> Camera {
    let x = 0.5;
    let z = -10.5;
    let y = terrain_surface_y(world, x, z).unwrap_or(8) as f32 + SANDBOX_EYE_HEIGHT + 1.0;
    Camera::new(Vec3::new(x, y, z))
        .looking_at(0.0, -0.12)
        .with_fov_y(68.0_f32.to_radians())
        .with_max_distance(120.0)
}

fn zombies_start_camera() -> Camera {
    Camera::new(Vec3::new(0.5, WALK_EYE_HEIGHT, -66.5))
        .looking_at(0.0, 0.0)
        .with_fov_y(68.0_f32.to_radians())
        .with_max_distance(150.0)
}

fn liminal_start_camera(liminal: &LiminalState) -> Camera {
    Camera::new(liminal.start_position)
        .looking_at(0.0, 0.0)
        .with_fov_y(66.0_f32.to_radians())
        .with_max_distance(120.0)
}

fn drone_gate_runner_start_camera(runner: &DroneGateRunnerState) -> Camera {
    let first_gate = runner.course.gates.first().copied().unwrap_or(Vec3::ZERO);
    look_at(runner.start_position, first_gate)
        .with_fov_y(72.0_f32.to_radians())
        .with_max_distance(DRONE_GATE_VIEW_DISTANCE)
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

#[derive(Clone, Copy, Debug, Default)]
struct ViewmodelBob {
    phase: f32,
    intensity: f32,
}

impl ViewmodelBob {
    fn update(&mut self, movement_distance: f32, moving: bool, dt: f32) {
        if moving && movement_distance > f32::EPSILON {
            self.phase += movement_distance * 0.575;
            self.intensity = (self.intensity + dt * 6.0).min(1.0);
        } else {
            self.intensity = (self.intensity - dt * 10.0).max(0.0);
        }
    }

    fn offset(self) -> (i32, i32) {
        let sway = (self.phase * 2.1).sin() * 2.0 * self.intensity;
        let lift = (self.phase * 4.2).sin() * 1.1 * self.intensity;
        (sway.round() as i32, lift.round() as i32)
    }
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct DroneGateRunnerConfig {
    gate_radius: i32,
    inner_radius: f32,
    tube_radius: i32,
    pass_distance: f32,
    spacing: f32,
}

impl Default for DroneGateRunnerConfig {
    fn default() -> Self {
        Self {
            gate_radius: DRONE_GATE_FRAME_RADIUS,
            inner_radius: DRONE_GATE_INNER_RADIUS,
            tube_radius: DRONE_GATE_TUBE_RADIUS,
            pass_distance: DRONE_GATE_PASS_DISTANCE,
            spacing: DRONE_GATE_COURSE_SPACING,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DroneGateCourse {
    seed: u64,
    name: &'static str,
    gates: Vec<Vec3>,
}

#[derive(Clone, Debug)]
struct DroneGateRunnerState {
    config: DroneGateRunnerConfig,
    course: DroneGateCourse,
    active_gate: usize,
    passed_gates: u32,
    laps: u32,
    start_position: Vec3,
    previous_position: Vec3,
    best_streak: u32,
    elapsed: f32,
}

impl DroneGateRunnerState {
    fn new_seeded(seed: u64) -> Self {
        let config = DroneGateRunnerConfig::default();
        let course = generate_drone_gate_course(seed, config);
        let first_gate = course.gates.first().copied().unwrap_or(Vec3::ZERO);
        let second_gate = course
            .gates
            .get(1)
            .copied()
            .unwrap_or(first_gate + Vec3::new(0.0, 0.0, config.spacing));
        let direction = horizontal(second_gate - first_gate);
        let start_position = first_gate - direction * DRONE_GATE_START_BACK;
        let start_position = Vec3::new(start_position.x, first_gate.y, start_position.z);
        Self {
            config,
            course,
            active_gate: 0,
            passed_gates: 0,
            laps: 0,
            start_position,
            previous_position: start_position,
            best_streak: 0,
            elapsed: 0.0,
        }
    }

    fn update(&mut self, player_position: Vec3, dt: f32) {
        self.elapsed += dt;
        if self.course.gates.is_empty() {
            self.previous_position = player_position;
            return;
        }

        if self.crossed_active_gate(self.previous_position, player_position) {
            self.advance_gate();
        }
        self.previous_position = player_position;
    }

    fn crossed_active_gate(&self, from: Vec3, to: Vec3) -> bool {
        let gate = self.course.gates[self.active_gate];
        let center_delta = to - gate;
        let radial = Vec3::new(center_delta.x, center_delta.y, 0.0).length();
        if radial > self.config.inner_radius {
            return false;
        }

        let previous_plane = from.z - gate.z;
        let current_plane = to.z - gate.z;
        previous_plane.abs().min(current_plane.abs()) <= self.config.pass_distance
            || previous_plane.signum() != current_plane.signum()
    }

    fn advance_gate(&mut self) {
        self.passed_gates += 1;
        self.best_streak = self.best_streak.max(self.passed_gates);
        self.active_gate = (self.active_gate + 1) % self.course.gates.len();
        if self.active_gate == 0 {
            self.laps += 1;
        }
    }

    fn active_gate_position(&self) -> Option<Vec3> {
        self.course.gates.get(self.active_gate).copied()
    }

    fn render_world(&self) -> VoxelWorld {
        build_drone_gate_runner_world(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LiminalRoomType {
    Hallway,
    Office,
    ConferenceRoom,
    Bathroom,
    BreakRoom,
    UtilityRoom,
}

impl LiminalRoomType {
    fn label(self) -> &'static str {
        match self {
            Self::Hallway => "hallway",
            Self::Office => "office",
            Self::ConferenceRoom => "conference",
            Self::Bathroom => "bathroom",
            Self::BreakRoom => "break room",
            Self::UtilityRoom => "utility",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiminalConnectionType {
    Door,
    Hallway,
    Loop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LiminalBounds {
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
}

impl LiminalBounds {
    fn contains(self, position: Vec3) -> bool {
        let x = position.x.floor() as i32;
        let z = position.z.floor() as i32;
        x >= self.min_x && x <= self.max_x && z >= self.min_z && z <= self.max_z
    }

    fn center(self) -> Vec3 {
        Vec3::new(
            (self.min_x + self.max_x + 1) as f32 * 0.5,
            WALK_EYE_HEIGHT,
            (self.min_z + self.max_z + 1) as f32 * 0.5,
        )
    }
}

#[derive(Clone, Debug)]
struct LiminalChair {
    position: Vec3,
    facing: BarFacing,
    observed: bool,
    rotated: bool,
}

#[derive(Clone, Debug)]
struct LiminalLight {
    position: Vec3,
    repaired: bool,
}

#[derive(Clone, Debug)]
struct LiminalRoom {
    id: usize,
    room_type: LiminalRoomType,
    bounds: LiminalBounds,
    sign_text: String,
    original_sign_text: String,
    visited: bool,
    visit_count: u32,
    chair: Option<LiminalChair>,
    light: Option<LiminalLight>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LiminalConnection {
    a: usize,
    b: usize,
    connection_type: LiminalConnectionType,
}

#[derive(Clone, Debug)]
struct LiminalWorldGraph {
    rooms: Vec<LiminalRoom>,
    connections: Vec<LiminalConnection>,
}

impl LiminalWorldGraph {
    fn room_at(&self, position: Vec3) -> Option<usize> {
        self.rooms
            .iter()
            .find(|room| room.bounds.contains(position))
            .map(|room| room.id)
    }

    fn room(&self, id: usize) -> Option<&LiminalRoom> {
        self.rooms.iter().find(|room| room.id == id)
    }

    fn room_mut(&mut self, id: usize) -> Option<&mut LiminalRoom> {
        self.rooms.iter_mut().find(|room| room.id == id)
    }

    fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiminalAnomalyKind {
    RoomSignChange,
    ObservedChairRotation,
    HallwayLoop,
}

impl LiminalAnomalyKind {
    fn label(self) -> &'static str {
        match self {
            Self::RoomSignChange => "sign change",
            Self::ObservedChairRotation => "chair rotation",
            Self::HallwayLoop => "hallway loop",
        }
    }
}

#[derive(Clone, Debug)]
struct LiminalAnomalyManager {
    forced_cursor: usize,
    triggered: Vec<LiminalAnomalyKind>,
}

impl LiminalAnomalyManager {
    fn new() -> Self {
        Self {
            forced_cursor: 0,
            triggered: Vec::new(),
        }
    }

    fn trigger(
        &mut self,
        kind: LiminalAnomalyKind,
        graph: &mut LiminalWorldGraph,
        wrongness: &mut f32,
        debug_message: &mut String,
    ) -> bool {
        match kind {
            LiminalAnomalyKind::RoomSignChange => {
                if self.trigger_room_sign_change(graph, None).is_none() {
                    return false;
                }
            }
            LiminalAnomalyKind::ObservedChairRotation => {
                if self
                    .triggered
                    .contains(&LiminalAnomalyKind::ObservedChairRotation)
                {
                    return false;
                }
                let Some(chair) = graph
                    .rooms
                    .iter_mut()
                    .filter_map(|room| room.chair.as_mut())
                    .find(|chair| chair.observed && !chair.rotated)
                else {
                    return false;
                };
                chair.facing = rotate_facing_clockwise(chair.facing);
                chair.rotated = true;
            }
            LiminalAnomalyKind::HallwayLoop => {
                if self.triggered.contains(&LiminalAnomalyKind::HallwayLoop) {
                    return false;
                }
            }
        }

        if !self.triggered.contains(&kind) {
            self.triggered.push(kind);
        }
        *wrongness = (*wrongness + 6.0).min(100.0);
        *debug_message = format!("forced anomaly: {}", kind.label());
        true
    }

    fn trigger_room_sign_change(
        &self,
        graph: &mut LiminalWorldGraph,
        preferred_room: Option<usize>,
    ) -> Option<()> {
        if self.triggered.contains(&LiminalAnomalyKind::RoomSignChange) {
            return None;
        }
        let room = if let Some(room_id) = preferred_room {
            graph.room_mut(room_id)?
        } else {
            graph.rooms.iter_mut().find(|room| {
                room.room_type != LiminalRoomType::Hallway && !room.sign_text.ends_with('?')
            })?
        };
        if room.room_type == LiminalRoomType::Hallway || room.sign_text.ends_with('?') {
            return None;
        }
        room.sign_text = format!("{}?", room.original_sign_text.replace('-', " "));
        Some(())
    }

    fn force_next(
        &mut self,
        graph: &mut LiminalWorldGraph,
        wrongness: &mut f32,
        debug_message: &mut String,
    ) -> bool {
        let sequence = [
            LiminalAnomalyKind::RoomSignChange,
            LiminalAnomalyKind::ObservedChairRotation,
            LiminalAnomalyKind::HallwayLoop,
        ];
        for _ in 0..sequence.len() {
            let kind = sequence[self.forced_cursor % sequence.len()];
            self.forced_cursor += 1;
            if self.trigger(kind, graph, wrongness, debug_message) {
                return true;
            }
        }
        *debug_message = "no anomaly candidate ready".to_string();
        false
    }

    fn maybe_trigger_on_room_exit(
        &mut self,
        exited_room: usize,
        current_room: Option<usize>,
        graph: &mut LiminalWorldGraph,
        wrongness: &mut f32,
        debug_message: &mut String,
    ) -> bool {
        if !self.triggered.contains(&LiminalAnomalyKind::RoomSignChange)
            && graph
                .room(exited_room)
                .is_some_and(|room| room.room_type != LiminalRoomType::Hallway && room.visited)
        {
            if self
                .trigger_room_sign_change(graph, Some(exited_room))
                .is_some()
            {
                self.triggered.push(LiminalAnomalyKind::RoomSignChange);
                *wrongness = (*wrongness + 6.0).min(100.0);
                *debug_message = format!("anomaly: {}", LiminalAnomalyKind::RoomSignChange.label());
                return true;
            }
        }

        if !self
            .triggered
            .contains(&LiminalAnomalyKind::ObservedChairRotation)
            && current_room != Some(exited_room)
            && graph
                .room(exited_room)
                .and_then(|room| room.chair.as_ref())
                .is_some_and(|chair| chair.observed && !chair.rotated)
        {
            return self.trigger(
                LiminalAnomalyKind::ObservedChairRotation,
                graph,
                wrongness,
                debug_message,
            );
        }

        if *wrongness >= 12.0 && !self.triggered.contains(&LiminalAnomalyKind::HallwayLoop) {
            return self.trigger(
                LiminalAnomalyKind::HallwayLoop,
                graph,
                wrongness,
                debug_message,
            );
        }

        false
    }

    fn hallway_loop_active(&self) -> bool {
        self.triggered.contains(&LiminalAnomalyKind::HallwayLoop)
    }
}

#[derive(Clone, Debug)]
struct LiminalObjective {
    target_room: usize,
    completed: bool,
}

impl LiminalObjective {
    fn description(&self, graph: &LiminalWorldGraph) -> String {
        let room = graph
            .room(self.target_room)
            .map(|room| room.sign_text.as_str())
            .unwrap_or("unknown room");
        if self.completed {
            format!("complete: repaired light in {}", room)
        } else {
            format!("repair flickering light in {}", room)
        }
    }
}

#[derive(Clone, Debug)]
struct LiminalState {
    seed: u64,
    world: VoxelWorld,
    graph: LiminalWorldGraph,
    start_position: Vec3,
    current_room: Option<usize>,
    previous_room: Option<usize>,
    wrongness: f32,
    anomaly_manager: LiminalAnomalyManager,
    objective: LiminalObjective,
    debug_message: String,
}

impl LiminalState {
    fn new_seeded(seed: u64) -> Self {
        let (graph, start_position, objective) = generate_liminal_office_zone(seed);
        let mut state = Self {
            seed,
            world: VoxelWorld::new(),
            graph,
            start_position,
            current_room: None,
            previous_room: None,
            wrongness: 4.0,
            anomaly_manager: LiminalAnomalyManager::new(),
            objective,
            debug_message: "debug: T force anomaly  F repair light".to_string(),
        };
        state.rebuild_world();
        state.current_room = state.graph.room_at(start_position);
        state
    }

    fn update_player_room(&mut self, camera: &mut Camera) {
        self.apply_hallway_loop(camera);
        let next_room = self.graph.room_at(camera.position);
        if next_room == self.current_room {
            return;
        }

        let exited_room = self.current_room;
        self.previous_room = exited_room;
        self.current_room = next_room;

        if let Some(room_id) = next_room {
            if let Some(room) = self.graph.room_mut(room_id) {
                room.visited = true;
                room.visit_count += 1;
                if let Some(chair) = &mut room.chair {
                    chair.observed = true;
                }
            }
            self.wrongness = (self.wrongness + 0.5).min(100.0);
        }

        let mutated = exited_room.is_some_and(|room_id| {
            self.anomaly_manager.maybe_trigger_on_room_exit(
                room_id,
                next_room,
                &mut self.graph,
                &mut self.wrongness,
                &mut self.debug_message,
            )
        });

        if mutated {
            self.rebuild_world();
        }
    }

    fn interact(&mut self, player_position: Vec3) {
        let Some(room_id) = self.current_room else {
            self.debug_message = "no room target".to_string();
            return;
        };
        if room_id != self.objective.target_room {
            self.debug_message = "nothing to repair here".to_string();
            return;
        }

        let Some(room) = self.graph.room_mut(room_id) else {
            return;
        };
        let Some(light) = &mut room.light else {
            self.debug_message = "no light fixture found".to_string();
            return;
        };
        if light.repaired {
            self.debug_message = "light already repaired".to_string();
            return;
        }
        if horizontal_distance(player_position, light.position) > LIMINAL_INTERACTION_RANGE {
            self.debug_message = "move closer to the light".to_string();
            return;
        }

        light.repaired = true;
        self.objective.completed = true;
        self.wrongness = (self.wrongness + 3.0).min(100.0);
        self.debug_message = "objective complete: light repaired".to_string();
        self.rebuild_world();
    }

    fn force_next_anomaly(&mut self) {
        if self.anomaly_manager.force_next(
            &mut self.graph,
            &mut self.wrongness,
            &mut self.debug_message,
        ) {
            self.rebuild_world();
        }
    }

    fn current_room_label(&self) -> String {
        self.current_room
            .and_then(|id| self.graph.room(id))
            .map(|room| format!("{} {}", room.id, room.sign_text))
            .unwrap_or_else(|| "none".to_string())
    }

    fn current_room_type_label(&self) -> &'static str {
        self.current_room
            .and_then(|id| self.graph.room(id))
            .map(|room| room.room_type.label())
            .unwrap_or("unknown")
    }

    fn rebuild_world(&mut self) {
        self.world = build_liminal_world(&self.graph);
    }

    fn apply_hallway_loop(&self, camera: &mut Camera) {
        if !self.anomaly_manager.hallway_loop_active() || self.current_room != Some(0) {
            return;
        }

        let Some(hallway) = self.graph.room(0) else {
            return;
        };
        if camera.position.x > hallway.bounds.max_x as f32 - 1.5 {
            camera.position.x = hallway.bounds.min_x as f32 + 2.5;
        } else if camera.position.x < hallway.bounds.min_x as f32 + 1.5 {
            camera.position.x = hallway.bounds.max_x as f32 - 2.5;
        }
    }
}

#[derive(Clone, Debug)]
struct PreviewAsset {
    name: &'static str,
    world: VoxelWorld,
    center: Vec3,
    radius: f32,
}

#[derive(Clone, Debug)]
struct VoxelSandboxState {
    world: VoxelWorld,
    selected_block: usize,
    palette: Vec<VoxelMaterial>,
}

impl VoxelSandboxState {
    fn new() -> Self {
        Self {
            world: build_voxel_sandbox_world(),
            selected_block: 0,
            palette: vec![
                VoxelMaterial::Grass,
                VoxelMaterial::Dirt,
                VoxelMaterial::Stone,
                VoxelMaterial::Sand,
                VoxelMaterial::Wood,
                VoxelMaterial::Leaves,
                VoxelMaterial::Glass,
                VoxelMaterial::Beacon,
            ],
        }
    }

    fn selected_material(&self) -> VoxelMaterial {
        self.palette[self.selected_block]
    }

    fn select_block(&mut self, index: usize) {
        if index < self.palette.len() {
            self.selected_block = index;
        }
    }

    fn remove_block(&mut self, camera: &Camera) {
        if let Some(hit) = raycast(
            &self.world,
            Ray::new(camera.position, camera.forward()),
            SANDBOX_REACH,
        ) {
            self.world.clear(hit.coord);
        }
    }

    fn place_block(&mut self, camera: &Camera) {
        if let Some(hit) = raycast(
            &self.world,
            Ray::new(camera.position, camera.forward()),
            SANDBOX_REACH,
        ) {
            let coord = offset_coord(hit.coord, placement_normal(hit.normal, camera.forward()));
            if can_place_sandbox_block(&self.world, coord, camera.position) {
                self.world
                    .set(coord, VoxelCell::new(self.selected_material()));
            }
        }
    }
}

impl PreviewAsset {
    fn new(name: &'static str, world: VoxelWorld) -> Self {
        let (center, radius) = asset_bounds(&world);
        Self {
            name,
            world,
            center,
            radius,
        }
    }
}

#[derive(Clone, Debug)]
struct AssetViewerState {
    assets: Vec<PreviewAsset>,
    selected: usize,
    camera: Camera,
    distance: f32,
}

impl AssetViewerState {
    fn new() -> Self {
        let assets = build_asset_catalog();
        let camera = asset_viewer_start_camera(&assets[0], ASSET_VIEWER_DEFAULT_DISTANCE);
        let mut viewer = Self {
            assets,
            selected: 0,
            camera,
            distance: ASSET_VIEWER_DEFAULT_DISTANCE,
        };
        viewer.reset_distance();
        viewer
    }

    fn selected_asset(&self) -> &PreviewAsset {
        &self.assets[self.selected]
    }

    fn select(&mut self, index: usize) {
        if index >= self.assets.len() {
            return;
        }
        self.selected = index;
        self.camera =
            asset_viewer_start_camera(self.selected_asset(), ASSET_VIEWER_DEFAULT_DISTANCE);
        self.reset_distance();
    }

    fn select_next(&mut self) {
        self.select((self.selected + 1) % self.assets.len());
    }

    fn select_previous(&mut self) {
        self.select((self.selected + self.assets.len() - 1) % self.assets.len());
    }

    fn update(&mut self, input: &PlayerInput, dt: f32) {
        let speed = if input.boost { BOOST_MULTIPLIER } else { 1.0 };
        let rotation_step = ASSET_VIEWER_ROTATE_SPEED * speed * dt;

        let mut yaw_delta = 0.0;
        let mut pitch_delta = 0.0;
        if input.right {
            yaw_delta += rotation_step;
        }
        if input.left {
            yaw_delta -= rotation_step;
        }
        if input.forward {
            pitch_delta += rotation_step;
        }
        if input.backward {
            pitch_delta -= rotation_step;
        }
        self.camera.rotate_local_yaw_pitch(yaw_delta, pitch_delta);
        if input.roll_left {
            self.camera.roll_by(ASSET_VIEWER_ROLL_SPEED * speed * dt);
        }
        if input.roll_right {
            self.camera.roll_by(-ASSET_VIEWER_ROLL_SPEED * speed * dt);
        }
        if input.up {
            self.distance -= ASSET_VIEWER_ZOOM_SPEED * speed * dt;
        }
        if input.down {
            self.distance += ASSET_VIEWER_ZOOM_SPEED * speed * dt;
        }

        self.clamp_view();
        self.sync_camera_position();
    }

    fn rotate_with_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.camera.rotate_local_yaw_pitch(
            delta_x * ASSET_VIEWER_MOUSE_SENSITIVITY,
            delta_y * ASSET_VIEWER_MOUSE_SENSITIVITY,
        );
        self.clamp_view();
        self.sync_camera_position();
    }

    fn camera(&self) -> Camera {
        self.camera
    }

    fn reset_distance(&mut self) {
        self.distance = ASSET_VIEWER_DEFAULT_DISTANCE;
        self.clamp_view();
        self.sync_camera_position();
    }

    fn clamp_view(&mut self) {
        let min_distance = ASSET_VIEWER_MIN_DISTANCE.max(self.selected_asset().radius * 1.15);
        self.distance = self.distance.clamp(min_distance, ASSET_VIEWER_MAX_DISTANCE);
        self.camera.max_distance = (self.distance + self.selected_asset().radius * 3.0).max(60.0);
    }

    fn sync_camera_position(&mut self) {
        self.camera.position = self.selected_asset().center - self.camera.forward() * self.distance;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ZombiesWeaponKind {
    M1911,
    WallRifle,
}

impl ZombiesWeaponKind {
    fn label(self) -> &'static str {
        match self {
            Self::M1911 => "M1911",
            Self::WallRifle => "WALL RIFLE",
        }
    }

    fn damage(self) -> i32 {
        match self {
            Self::M1911 => 45,
            Self::WallRifle => 72,
        }
    }

    fn magazine_size(self) -> i32 {
        match self {
            Self::M1911 => ZOMBIE_MAG_SIZE,
            Self::WallRifle => 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ZombiesDoorKind {
    Building,
    CornField,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ZombiesDoor {
    kind: ZombiesDoorKind,
    position: Vec3,
    open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WallWeapon {
    position: Vec3,
    bought: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Zombie {
    position: Vec3,
    health: i32,
    max_health: i32,
    attack_cooldown: f32,
}

impl Zombie {
    fn new(position: Vec3, round: u32) -> Self {
        let max_health = 90 + (round.saturating_sub(1) as i32 * 34);
        Self {
            position,
            health: max_health,
            max_health,
            attack_cooldown: 0.0,
        }
    }

    fn is_alive(self) -> bool {
        self.health > 0
    }

    fn contains_voxel(self, coord: VoxelCoord) -> bool {
        zombie_body_contains_voxel(self.position, coord)
    }
}

#[derive(Clone, Debug)]
struct ZombiesState {
    zombies: Vec<Zombie>,
    bullet_traces: Vec<BulletTrace>,
    round: u32,
    queued_spawns: u32,
    spawn_timer: f32,
    round_break_timer: f32,
    player_hits: i32,
    damage_flash_timer: f32,
    points: i32,
    total_points: i32,
    kills: u32,
    ammo_reserve: i32,
    ammo_in_mag: i32,
    weapon: ZombiesWeaponKind,
    shot_flash_timer: f32,
    sprint: f32,
    sprint_locked: bool,
    doors: Vec<ZombiesDoor>,
    wall_weapon: WallWeapon,
    game_over: bool,
}

impl ZombiesState {
    fn new() -> Self {
        let mut state = Self {
            zombies: Vec::new(),
            bullet_traces: Vec::new(),
            round: 0,
            queued_spawns: 0,
            spawn_timer: 0.0,
            round_break_timer: 0.0,
            player_hits: 0,
            damage_flash_timer: 0.0,
            points: 500,
            total_points: 0,
            kills: 0,
            ammo_reserve: ZOMBIE_START_AMMO,
            ammo_in_mag: ZOMBIE_MAG_SIZE,
            weapon: ZombiesWeaponKind::M1911,
            shot_flash_timer: 0.0,
            sprint: 1.0,
            sprint_locked: false,
            doors: vec![
                ZombiesDoor {
                    kind: ZombiesDoorKind::Building,
                    position: Vec3::new(0.5, 0.0, -25.0),
                    open: false,
                },
                ZombiesDoor {
                    kind: ZombiesDoorKind::CornField,
                    position: Vec3::new(-36.0, 0.0, 9.0),
                    open: false,
                },
            ],
            wall_weapon: WallWeapon {
                position: Vec3::new(33.0, 0.0, -18.0),
                bought: false,
            },
            game_over: false,
        };
        state.start_next_round();
        state
    }

    fn update_player(
        &mut self,
        camera: &mut Camera,
        input: &PlayerInput,
        world: &VoxelWorld,
        dt: f32,
    ) {
        if self.game_over {
            return;
        }

        if self.sprint_locked && self.sprint >= 1.0 {
            self.sprint_locked = false;
        }

        let sprinting = input.boost
            && !self.sprint_locked
            && self.sprint > f32::EPSILON
            && moving_on_ground(input);
        let speed = if sprinting {
            ZOMBIE_WALK_SPEED * ZOMBIE_SPRINT_MULTIPLIER
        } else {
            ZOMBIE_WALK_SPEED
        };
        if sprinting {
            self.sprint = (self.sprint - ZOMBIE_SPRINT_DRAIN * dt).max(0.0);
            if self.sprint <= f32::EPSILON {
                self.sprint = 0.0;
                self.sprint_locked = true;
            }
        } else {
            self.sprint = (self.sprint + ZOMBIE_SPRINT_RECHARGE * dt).min(1.0);
            if self.sprint >= 1.0 {
                self.sprint_locked = false;
            }
        }

        let mut walking_input = *input;
        walking_input.boost = false;
        update_walking_camera_with_profile(
            camera,
            &walking_input,
            world,
            WalkProfile {
                eye_height: ZOMBIE_EYE_HEIGHT,
                speed,
                collision_radius: ZOMBIE_COLLISION_RADIUS,
            },
            dt,
        );
    }

    fn update_rounds_and_zombies(
        &mut self,
        world: &VoxelWorld,
        player_position: Vec3,
        dt: f32,
    ) -> bool {
        self.shot_flash_timer = (self.shot_flash_timer - dt).max(0.0);
        self.damage_flash_timer = (self.damage_flash_timer - dt).max(0.0);
        for trace in &mut self.bullet_traces {
            trace.time_left = (trace.time_left - dt).max(0.0);
        }
        self.bullet_traces
            .retain(|trace| trace.time_left > f32::EPSILON);

        if self.game_over {
            return false;
        }

        if self.alive_count() == 0 && self.queued_spawns == 0 {
            self.round_break_timer -= dt;
            if self.round_break_timer <= 0.0 {
                self.start_next_round();
            }
        }

        let navigation =
            ZombieNavigationField::build(world, player_position, zombie_walk_profile());
        self.spawn_timer -= dt;
        while self.queued_spawns > 0 && self.spawn_timer <= 0.0 {
            if !self.spawn_one(world) {
                break;
            }
            self.queued_spawns -= 1;
            self.spawn_timer += ZOMBIE_SPAWN_INTERVAL;
        }

        let mut player_hurt = false;
        for zombie in &mut self.zombies {
            if !zombie.is_alive() {
                continue;
            }

            zombie.attack_cooldown = (zombie.attack_cooldown - dt).max(0.0);
            let distance = horizontal_distance(zombie.position, player_position);
            if distance <= ZOMBIE_ATTACK_RANGE {
                if zombie.attack_cooldown <= 0.0 {
                    self.player_hits += 1;
                    self.damage_flash_timer = ZOMBIE_HIT_FLASH_TIME;
                    zombie.attack_cooldown = ZOMBIE_ATTACK_COOLDOWN;
                    player_hurt = true;
                    if self.player_hits >= ZOMBIE_MAX_HITS {
                        self.game_over = true;
                    }
                }
                continue;
            }

            if let Some(step) = navigation
                .as_ref()
                .and_then(|field| field.next_step(zombie.position))
            {
                let candidate =
                    zombie.position + horizontal(step - zombie.position) * ZOMBIE_WALK_SPEED * dt;
                let candidate = Vec3::new(candidate.x, ZOMBIE_EYE_HEIGHT, candidate.z);
                if can_walk_to_on_ground(world, candidate, zombie_walk_profile()) {
                    zombie.position.x = candidate.x;
                    zombie.position.z = candidate.z;
                }
            }
        }

        player_hurt
    }

    fn fire(&mut self, world: &VoxelWorld, camera: &Camera) -> Vec<SoundEffect> {
        if self.game_over {
            return Vec::new();
        }
        if self.ammo_in_mag <= 0 {
            self.reload();
            return Vec::new();
        }

        self.ammo_in_mag -= 1;
        self.shot_flash_timer = SHOT_FLASH_TIME;
        self.bullet_traces.push(BulletTrace::from_camera(camera));
        let mut effects = vec![SoundEffect::Gunshot];

        let render_world = zombies_world_with_zombies(world, self);
        let Some(hit) = raycast(
            &render_world,
            Ray::new(camera.position, camera.forward()),
            WEAPON_RANGE,
        ) else {
            return effects;
        };
        let Some(index) = self.zombie_index_for_voxel(hit.coord) else {
            return effects;
        };

        let zombie = &mut self.zombies[index];
        zombie.health = (zombie.health - self.weapon.damage()).max(0);
        self.points += ZOMBIE_HIT_POINTS;
        self.total_points += ZOMBIE_HIT_POINTS;
        if zombie.health == 0 {
            self.points += ZOMBIE_KILL_POINTS;
            self.total_points += ZOMBIE_KILL_POINTS;
            self.kills += 1;
            effects.push(SoundEffect::EnemyDeath);
        } else {
            effects.push(SoundEffect::EnemyHit);
        }
        effects
    }

    fn reload(&mut self) {
        if self.ammo_reserve <= 0 || self.ammo_in_mag >= self.weapon.magazine_size() {
            return;
        }
        let needed = self.weapon.magazine_size() - self.ammo_in_mag;
        let loaded = needed.min(self.ammo_reserve);
        self.ammo_in_mag += loaded;
        self.ammo_reserve -= loaded;
    }

    fn interact(&mut self, world: &mut VoxelWorld, player_position: Vec3) {
        if self.game_over {
            return;
        }
        for door in &mut self.doors {
            if !door.open
                && self.points >= ZOMBIE_DOOR_COST
                && horizontal_distance(player_position, door.position) < 5.5
            {
                self.points -= ZOMBIE_DOOR_COST;
                door.open = true;
                clear_zombies_door(world, door.kind);
                return;
            }
        }

        if !self.wall_weapon.bought
            && self.points >= ZOMBIE_WALL_WEAPON_COST
            && horizontal_distance(player_position, self.wall_weapon.position) < 6.5
        {
            self.points -= ZOMBIE_WALL_WEAPON_COST;
            self.wall_weapon.bought = true;
            self.weapon = ZombiesWeaponKind::WallRifle;
            self.ammo_in_mag = self.weapon.magazine_size();
            self.ammo_reserve = 160;
        }
    }

    fn start_next_round(&mut self) {
        self.round += 1;
        self.queued_spawns = 5 + self.round * 3;
        self.spawn_timer = 0.0;
        self.round_break_timer = ZOMBIE_ROUND_BREAK_TIME;
        self.zombies.retain(|zombie| zombie.is_alive());
    }

    fn spawn_one(&mut self, world: &VoxelWorld) -> bool {
        let spawns = zombie_spawn_points(self, world);
        if spawns.is_empty() {
            return false;
        }
        let index = (self.zombies.len() + self.round as usize * 3) % spawns.len();
        self.zombies.push(Zombie::new(spawns[index], self.round));
        true
    }

    fn zombie_index_for_voxel(&self, coord: VoxelCoord) -> Option<usize> {
        self.zombies
            .iter()
            .enumerate()
            .filter(|(_, zombie)| zombie.is_alive())
            .find_map(|(index, zombie)| zombie.contains_voxel(coord).then_some(index))
    }

    fn alive_count(&self) -> usize {
        self.zombies
            .iter()
            .filter(|zombie| zombie.is_alive())
            .count()
    }

    fn health_left(&self) -> i32 {
        (ZOMBIE_MAX_HITS - self.player_hits).max(0)
    }

    fn rounds_survived(&self) -> u32 {
        if self.game_over {
            self.round.saturating_sub(1)
        } else {
            self.round
        }
    }
}

fn moving_on_ground(input: &PlayerInput) -> bool {
    input.forward || input.backward || input.left || input.right
}

fn zombie_walk_profile() -> WalkProfile {
    WalkProfile {
        eye_height: ZOMBIE_EYE_HEIGHT,
        speed: ZOMBIE_WALK_SPEED,
        collision_radius: ZOMBIE_COLLISION_RADIUS,
    }
}

fn can_walk_to_on_ground(city: &VoxelWorld, position: Vec3, profile: WalkProfile) -> bool {
    terrain_surface_y(city, position.x, position.z).is_some()
        && can_walk_to_with_profile(city, position, profile)
}

fn zombie_spawn_candidates(state: &ZombiesState, world: &VoxelWorld) -> Vec<Vec3> {
    let mut points = Vec::new();
    let zones = [
        (-74, -48, -74, -40),
        (48, 74, -74, -40),
        (-74, -48, 8, 74),
        (12, 38, 42, 74),
    ];

    for (min_x, max_x, min_z, max_z) in zones {
        let mut x = min_x;
        while x <= max_x {
            let mut z = min_z;
            while z <= max_z {
                let position = Vec3::new(x as f32 + 0.5, ZOMBIE_EYE_HEIGHT, z as f32 + 0.5);
                if zombie_spawn_position_is_valid(world, position) {
                    points.push(position);
                }
                z += 4;
            }
            x += 4;
        }
    }

    if state
        .doors
        .iter()
        .any(|door| door.kind == ZombiesDoorKind::Building && door.open)
    {
        for x in 8..=36 {
            for z in 40..=66 {
                let position = Vec3::new(x as f32 + 0.5, ZOMBIE_EYE_HEIGHT, z as f32 + 0.5);
                if zombie_spawn_position_is_valid(world, position) {
                    points.push(position);
                }
            }
        }
    }

    if state
        .doors
        .iter()
        .any(|door| door.kind == ZombiesDoorKind::CornField && door.open)
    {
        for x in -70..=-40 {
            for z in 40..=74 {
                let position = Vec3::new(x as f32 + 0.5, ZOMBIE_EYE_HEIGHT, z as f32 + 0.5);
                if zombie_spawn_position_is_valid(world, position) {
                    points.push(position);
                }
            }
        }
    }

    points.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal))
    });
    points.dedup_by(|a, b| a.x == b.x && a.z == b.z);
    points
}

fn zombie_spawn_position_is_valid(world: &VoxelWorld, position: Vec3) -> bool {
    if !can_walk_to_on_ground(world, position, zombie_walk_profile()) {
        return false;
    }

    let base_x = position.x.floor() as i32;
    let base_z = position.z.floor() as i32;
    for offset_x in -ZOMBIE_SPAWN_CLEARANCE_RADIUS..=ZOMBIE_SPAWN_CLEARANCE_RADIUS {
        for offset_z in -ZOMBIE_SPAWN_CLEARANCE_RADIUS..=ZOMBIE_SPAWN_CLEARANCE_RADIUS {
            let sample = Vec3::new(
                base_x as f32 + 0.5 + offset_x as f32,
                ZOMBIE_EYE_HEIGHT,
                base_z as f32 + 0.5 + offset_z as f32,
            );
            if !can_walk_to_on_ground(world, sample, zombie_walk_profile()) {
                return false;
            }
        }
    }

    true
}

fn zombie_spawn_points(state: &ZombiesState, world: &VoxelWorld) -> Vec<Vec3> {
    zombie_spawn_candidates(state, world)
}

struct ZombieNavigationField {
    min_x: i32,
    min_z: i32,
    width: usize,
    height: usize,
    distances: Vec<u16>,
}

impl ZombieNavigationField {
    fn build(world: &VoxelWorld, target: Vec3, profile: WalkProfile) -> Option<Self> {
        let bounds = world.bounds()?;
        let min_x = bounds.min.x - 2;
        let max_x = bounds.max.x + 2;
        let min_z = bounds.min.z - 2;
        let max_z = bounds.max.z + 2;
        let width = (max_x - min_x + 1) as usize;
        let height = (max_z - min_z + 1) as usize;
        let mut distances = vec![u16::MAX; width * height];
        let start_x = target.x.floor() as i32;
        let start_z = target.z.floor() as i32;

        if !zombie_cell_is_walkable(world, start_x, start_z, profile) {
            return None;
        }

        let mut queue = VecDeque::new();
        let start_index = Self::index_raw(min_x, min_z, width, height, start_x, start_z)?;
        distances[start_index] = 0;
        queue.push_back((start_x, start_z));

        while let Some((x, z)) = queue.pop_front() {
            let current_index = Self::index_raw(min_x, min_z, width, height, x, z)?;
            let current_distance = distances[current_index];

            for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
                if !zombie_cell_is_walkable(world, nx, nz, profile) {
                    continue;
                }

                let Some(index) = Self::index_raw(min_x, min_z, width, height, nx, nz) else {
                    continue;
                };
                let next_distance = current_distance.saturating_add(1);
                if next_distance < distances[index] {
                    distances[index] = next_distance;
                    queue.push_back((nx, nz));
                }
            }
        }

        Some(Self {
            min_x,
            min_z,
            width,
            height,
            distances,
        })
    }

    fn next_step(&self, position: Vec3) -> Option<Vec3> {
        let x = position.x.floor() as i32;
        let z = position.z.floor() as i32;
        let current = self.distance(x, z)?;
        if current == 0 {
            return Some(Vec3::new(x as f32 + 0.5, ZOMBIE_EYE_HEIGHT, z as f32 + 0.5));
        }

        let mut best: Option<(u16, i32, i32)> = None;
        for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
            let Some(distance) = self.distance(nx, nz) else {
                continue;
            };
            if distance < current
                && best
                    .map(|(best_distance, _, _)| distance < best_distance)
                    .unwrap_or(true)
            {
                best = Some((distance, nx, nz));
            }
        }

        best.map(|(_, nx, nz)| Vec3::new(nx as f32 + 0.5, ZOMBIE_EYE_HEIGHT, nz as f32 + 0.5))
    }

    fn distance(&self, x: i32, z: i32) -> Option<u16> {
        let index = Self::index_raw(self.min_x, self.min_z, self.width, self.height, x, z)?;
        Some(self.distances[index])
    }

    fn index_raw(
        min_x: i32,
        min_z: i32,
        width: usize,
        height: usize,
        x: i32,
        z: i32,
    ) -> Option<usize> {
        let dx = x - min_x;
        let dz = z - min_z;
        if dx < 0 || dz < 0 || dx as usize >= width || dz as usize >= height {
            return None;
        }
        Some(dx as usize + dz as usize * width)
    }
}

fn zombie_cell_is_walkable(world: &VoxelWorld, x: i32, z: i32, profile: WalkProfile) -> bool {
    let position = Vec3::new(x as f32 + 0.5, profile.eye_height, z as f32 + 0.5);
    can_walk_to_on_ground(world, position, profile)
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

fn asset_digit_index(key: &PhysicalKey) -> Option<usize> {
    match key {
        PhysicalKey::Code(KeyCode::Digit1) => Some(0),
        PhysicalKey::Code(KeyCode::Digit2) => Some(1),
        PhysicalKey::Code(KeyCode::Digit3) => Some(2),
        PhysicalKey::Code(KeyCode::Digit4) => Some(3),
        PhysicalKey::Code(KeyCode::Digit5) => Some(4),
        PhysicalKey::Code(KeyCode::Digit6) => Some(5),
        PhysicalKey::Code(KeyCode::Digit7) => Some(6),
        PhysicalKey::Code(KeyCode::Digit8) => Some(7),
        PhysicalKey::Code(KeyCode::Digit9) => Some(8),
        _ => None,
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

fn update_sandbox_camera(camera: &mut Camera, input: &PlayerInput, world: &VoxelWorld, dt: f32) {
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
    if input.up {
        movement = movement + Vec3::new(0.0, 1.0, 0.0);
    }
    if input.down {
        movement = movement - Vec3::new(0.0, 1.0, 0.0);
    }

    if movement.length() <= f32::EPSILON {
        return;
    }

    let speed = if input.boost {
        SANDBOX_SPEED * WALK_BOOST_MULTIPLIER
    } else {
        SANDBOX_SPEED
    };
    let step = movement.normalized() * speed * dt;
    camera.position = move_sandbox_with_collision(camera.position, step, world);
}

fn move_sandbox_with_collision(position: Vec3, step: Vec3, world: &VoxelWorld) -> Vec3 {
    let full = position + step;
    if can_fly_to_in_sandbox(world, full) {
        return full;
    }

    let mut resolved = position;
    for axis_step in [
        Vec3::new(step.x, 0.0, 0.0),
        Vec3::new(0.0, step.y, 0.0),
        Vec3::new(0.0, 0.0, step.z),
    ] {
        let candidate = resolved + axis_step;
        if can_fly_to_in_sandbox(world, candidate) {
            resolved = candidate;
        }
    }

    resolved
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

fn can_fly_to_in_sandbox(world: &VoxelWorld, position: Vec3) -> bool {
    let samples = [
        (0.0, 0.0),
        (-SANDBOX_COLLISION_RADIUS, -SANDBOX_COLLISION_RADIUS),
        (-SANDBOX_COLLISION_RADIUS, SANDBOX_COLLISION_RADIUS),
        (SANDBOX_COLLISION_RADIUS, -SANDBOX_COLLISION_RADIUS),
        (SANDBOX_COLLISION_RADIUS, SANDBOX_COLLISION_RADIUS),
    ];
    samples.iter().all(|(offset_x, offset_z)| {
        let x = (position.x + offset_x).floor() as i32;
        let z = (position.z + offset_z).floor() as i32;
        let min_y = (position.y - SANDBOX_EYE_HEIGHT + 0.25).floor() as i32;
        let max_y = position.y.floor() as i32;
        (min_y..=max_y).all(|y| world.get(VoxelCoord::new(x, y, z)).is_none())
    })
}

fn can_place_sandbox_block(world: &VoxelWorld, coord: VoxelCoord, camera_position: Vec3) -> bool {
    world.get(coord).is_none() && !sandbox_player_occupies(camera_position, coord)
}

fn sandbox_player_occupies(position: Vec3, coord: VoxelCoord) -> bool {
    let min_y = (position.y - SANDBOX_EYE_HEIGHT + 0.25).floor() as i32;
    let max_y = position.y.floor() as i32;
    if coord.y < min_y || coord.y > max_y {
        return false;
    }

    let dx = coord.x as f32 + 0.5 - position.x;
    let dz = coord.z as f32 + 0.5 - position.z;
    (dx * dx + dz * dz).sqrt() <= SANDBOX_COLLISION_RADIUS + 0.5
}

fn offset_coord(coord: VoxelCoord, normal: Vec3) -> VoxelCoord {
    VoxelCoord::new(
        coord.x + normal.x.round() as i32,
        coord.y + normal.y.round() as i32,
        coord.z + normal.z.round() as i32,
    )
}

fn placement_normal(hit_normal: Vec3, camera_forward: Vec3) -> Vec3 {
    if hit_normal.length() > f32::EPSILON {
        return hit_normal;
    }

    let direction = camera_forward.normalized();
    let abs_x = direction.x.abs();
    let abs_y = direction.y.abs();
    let abs_z = direction.z.abs();
    if abs_x >= abs_y && abs_x >= abs_z {
        Vec3::new(-direction.x.signum(), 0.0, 0.0)
    } else if abs_y >= abs_z {
        Vec3::new(0.0, -direction.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, -direction.z.signum())
    }
}

fn terrain_surface_y(world: &VoxelWorld, x: f32, z: f32) -> Option<i32> {
    let x = x.floor() as i32;
    let z = z.floor() as i32;
    (0..=32)
        .rev()
        .find(|y| world.get(VoxelCoord::new(x, *y, z)).is_some())
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

fn generate_drone_gate_course(seed: u64, config: DroneGateRunnerConfig) -> DroneGateCourse {
    let mut rng = LiminalRng::new(seed);
    let mut gates = Vec::new();
    let gate_count = 14;

    for index in 0..gate_count {
        let t = index as f32;
        let wave_x = (t * 0.76).sin() * DRONE_GATE_COURSE_WIDTH;
        let wave_y = 18.0 + (t * 0.59).cos() * DRONE_GATE_COURSE_HEIGHT;
        let jitter_x = rng.range_f32(-18.0, 18.0);
        let jitter_y = rng.range_f32(-6.0, 8.0);
        gates.push(Vec3::new(
            wave_x + jitter_x,
            (wave_y + jitter_y).max(config.gate_radius as f32 + 4.0),
            index as f32 * config.spacing,
        ));
    }

    DroneGateCourse {
        seed,
        name: "Relay Spine",
        gates,
    }
}

fn build_drone_gate_runner_world(state: &DroneGateRunnerState) -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_drone_course_grid(&mut world, state);
    for (index, gate) in state.course.gates.iter().copied().enumerate() {
        stamp_drone_gate(
            &mut world,
            gate,
            state.config,
            index == state.active_gate,
            index < state.active_gate || state.laps > 0,
        );
    }
    stamp_drone_start_marker(&mut world, state.start_position);
    world
}

fn stamp_drone_course_grid(world: &mut VoxelWorld, state: &DroneGateRunnerState) {
    let Some(first) = state.course.gates.first().copied() else {
        return;
    };
    let Some(last) = state.course.gates.last().copied() else {
        return;
    };
    let z_min = first.z.floor() as i32 - 56;
    let z_max = last.z.ceil() as i32 + 80;

    for z in (z_min..=z_max).step_by(32) {
        for x in (-128..=128).step_by(32) {
            world.set(
                VoxelCoord::new(x, -18, z),
                VoxelCell::new(VoxelMaterial::Glass),
            );
        }
    }

    for gate in &state.course.gates {
        fill_cuboid(
            world,
            VoxelCoord::new(gate.x.floor() as i32 - 1, -16, gate.z.floor() as i32 - 1),
            VoxelCoord::new(gate.x.floor() as i32 + 1, -10, gate.z.floor() as i32 + 1),
            VoxelMaterial::ShipHull,
        );
    }
}

fn stamp_drone_gate(
    world: &mut VoxelWorld,
    center: Vec3,
    config: DroneGateRunnerConfig,
    active: bool,
    completed: bool,
) {
    let material = if active {
        VoxelMaterial::Beacon
    } else if completed {
        VoxelMaterial::Glass
    } else {
        VoxelMaterial::Gate
    };
    let radius = config.gate_radius as f32;

    for segment in 0..DRONE_GATE_RING_SEGMENTS {
        let radians = segment as f32 / DRONE_GATE_RING_SEGMENTS as f32 * std::f32::consts::TAU;
        let x = center.x + radians.cos() * radius;
        let y = center.y + radians.sin() * radius;
        fill_cuboid(
            world,
            VoxelCoord::new(
                x.round() as i32 - config.tube_radius,
                y.round() as i32 - config.tube_radius,
                center.z.round() as i32 - DRONE_GATE_DEPTH,
            ),
            VoxelCoord::new(
                x.round() as i32 + config.tube_radius,
                y.round() as i32 + config.tube_radius,
                center.z.round() as i32 + DRONE_GATE_DEPTH,
            ),
            material,
        );
    }

    fill_cuboid(
        world,
        VoxelCoord::new(
            center.x.round() as i32 - 1,
            center.y.round() as i32 + config.gate_radius + 2,
            center.z.round() as i32 - 1,
        ),
        VoxelCoord::new(
            center.x.round() as i32 + 1,
            center.y.round() as i32 + config.gate_radius + 5,
            center.z.round() as i32 + 1,
        ),
        material,
    );
}

fn stamp_drone_start_marker(world: &mut VoxelWorld, position: Vec3) {
    fill_cuboid(
        world,
        VoxelCoord::new(
            position.x.floor() as i32 - 3,
            position.y.floor() as i32 - 3,
            position.z.floor() as i32 - 3,
        ),
        VoxelCoord::new(
            position.x.floor() as i32 + 3,
            position.y.floor() as i32 - 1,
            position.z.floor() as i32 + 3,
        ),
        VoxelMaterial::ShipHull,
    );
}

#[derive(Clone, Copy, Debug)]
struct LiminalRng {
    state: u64,
}

impl LiminalRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(0x5851_F42D_4C95_7F2D)
            .wrapping_add(0x1405_7B7E_F767_814F);
        (self.state >> 32) as u32
    }

    fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next_u32() % (max - min + 1) as u32) as i32
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        let unit = self.next_u32() as f32 / u32::MAX as f32;
        min + (max - min) * unit
    }
}

fn generate_liminal_office_zone(seed: u64) -> (LiminalWorldGraph, Vec3, LiminalObjective) {
    let mut rng = LiminalRng::new(seed);
    let mut rooms = Vec::new();
    let hallway_bounds = LiminalBounds {
        min_x: -76,
        max_x: 76,
        min_z: -LIMINAL_HALL_HALF_WIDTH,
        max_z: LIMINAL_HALL_HALF_WIDTH,
    };
    rooms.push(LiminalRoom {
        id: 0,
        room_type: LiminalRoomType::Hallway,
        bounds: hallway_bounds,
        sign_text: "MAIN HALL".to_string(),
        original_sign_text: "MAIN HALL".to_string(),
        visited: false,
        visit_count: 0,
        chair: None,
        light: None,
    });

    let room_types = [
        LiminalRoomType::Office,
        LiminalRoomType::ConferenceRoom,
        LiminalRoomType::Bathroom,
        LiminalRoomType::BreakRoom,
        LiminalRoomType::UtilityRoom,
        LiminalRoomType::Office,
        LiminalRoomType::ConferenceRoom,
    ];
    let slots = [-60, -38, -16, 6, 28, 50, 70];
    let mut type_counts: HashMap<LiminalRoomType, usize> = HashMap::new();
    for (index, room_type) in room_types.into_iter().enumerate() {
        let width = rng.range_i32(14, 20);
        let depth = rng.range_i32(13, 19);
        let center_x = slots[index];
        let north = index % 2 == 0;
        let bounds = if north {
            LiminalBounds {
                min_x: center_x - width / 2,
                max_x: center_x + width / 2,
                min_z: -LIMINAL_HALL_HALF_WIDTH - depth - 1,
                max_z: -LIMINAL_HALL_HALF_WIDTH - 1,
            }
        } else {
            LiminalBounds {
                min_x: center_x - width / 2,
                max_x: center_x + width / 2,
                min_z: LIMINAL_HALL_HALF_WIDTH + 1,
                max_z: LIMINAL_HALL_HALF_WIDTH + depth + 1,
            }
        };
        let count = type_counts.entry(room_type).or_insert(0);
        *count += 1;
        let room_id = rooms.len();
        let center = bounds.center();
        let chair = matches!(
            room_type,
            LiminalRoomType::Office | LiminalRoomType::ConferenceRoom | LiminalRoomType::BreakRoom
        )
        .then_some(LiminalChair {
            position: Vec3::new(center.x + rng.range_i32(-3, 3) as f32, 0.0, center.z),
            facing: if north {
                BarFacing::South
            } else {
                BarFacing::North
            },
            observed: false,
            rotated: false,
        });
        let light = Some(LiminalLight {
            position: Vec3::new(center.x, WALK_EYE_HEIGHT, center.z),
            repaired: false,
        });
        let label = match room_type {
            LiminalRoomType::Office => format!("OFFICE {}", 100 + room_id),
            LiminalRoomType::ConferenceRoom => {
                format!("CONFERENCE {}", ('A' as u8 + *count as u8 - 1) as char)
            }
            LiminalRoomType::Bathroom => "RESTROOM".to_string(),
            LiminalRoomType::BreakRoom => "BREAK ROOM".to_string(),
            LiminalRoomType::UtilityRoom => "UTILITY".to_string(),
            LiminalRoomType::Hallway => "HALLWAY".to_string(),
        };
        rooms.push(LiminalRoom {
            id: room_id,
            room_type,
            bounds,
            sign_text: label.clone(),
            original_sign_text: label,
            visited: false,
            visit_count: 0,
            chair,
            light,
        });
    }

    let mut connections = Vec::new();
    for room_id in 1..rooms.len() {
        connections.push(LiminalConnection {
            a: 0,
            b: room_id,
            connection_type: LiminalConnectionType::Door,
        });
    }
    connections.push(LiminalConnection {
        a: 0,
        b: 0,
        connection_type: LiminalConnectionType::Hallway,
    });
    connections.push(LiminalConnection {
        a: 0,
        b: 0,
        connection_type: LiminalConnectionType::Loop,
    });

    let objective = LiminalObjective {
        target_room: rooms
            .iter()
            .find(|room| room.room_type == LiminalRoomType::UtilityRoom)
            .map(|room| room.id)
            .unwrap_or(1),
        completed: false,
    };
    let start_position = Vec3::new(hallway_bounds.min_x as f32 + 8.5, WALK_EYE_HEIGHT, 0.5);
    (
        LiminalWorldGraph { rooms, connections },
        start_position,
        objective,
    )
}

fn build_liminal_world(graph: &LiminalWorldGraph) -> VoxelWorld {
    let mut world = VoxelWorld::new();
    for room in &graph.rooms {
        stamp_liminal_room_shell(&mut world, room);
    }
    for room in &graph.rooms {
        if room.room_type != LiminalRoomType::Hallway {
            clear_liminal_door(&mut world, room);
            stamp_liminal_sign(&mut world, room);
            stamp_liminal_room_props(&mut world, room);
        }
        if let Some(light) = &room.light {
            stamp_liminal_light(&mut world, light);
        }
    }
    world
}

fn stamp_liminal_room_shell(world: &mut VoxelWorld, room: &LiminalRoom) {
    let floor = match room.room_type {
        LiminalRoomType::Bathroom | LiminalRoomType::UtilityRoom => VoxelMaterial::Stone,
        LiminalRoomType::BreakRoom => VoxelMaterial::Wood,
        _ => VoxelMaterial::Habitat,
    };
    fill_cuboid(
        world,
        VoxelCoord::new(room.bounds.min_x, 0, room.bounds.min_z),
        VoxelCoord::new(room.bounds.max_x, 0, room.bounds.max_z),
        floor,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(room.bounds.min_x, 1, room.bounds.min_z),
        VoxelCoord::new(room.bounds.max_x, LIMINAL_ROOM_HEIGHT, room.bounds.min_z),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(room.bounds.min_x, 1, room.bounds.max_z),
        VoxelCoord::new(room.bounds.max_x, LIMINAL_ROOM_HEIGHT, room.bounds.max_z),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(room.bounds.min_x, 1, room.bounds.min_z),
        VoxelCoord::new(room.bounds.min_x, LIMINAL_ROOM_HEIGHT, room.bounds.max_z),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(room.bounds.max_x, 1, room.bounds.min_z),
        VoxelCoord::new(room.bounds.max_x, LIMINAL_ROOM_HEIGHT, room.bounds.max_z),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(
            room.bounds.min_x,
            LIMINAL_ROOM_HEIGHT + 1,
            room.bounds.min_z,
        ),
        VoxelCoord::new(
            room.bounds.max_x,
            LIMINAL_ROOM_HEIGHT + 1,
            room.bounds.max_z,
        ),
        VoxelMaterial::ShipHull,
    );
}

fn clear_liminal_door(world: &mut VoxelWorld, room: &LiminalRoom) {
    let center_x = ((room.bounds.min_x + room.bounds.max_x) / 2).clamp(-74, 74);
    let door_z = if room.bounds.max_z < 0 {
        room.bounds.max_z
    } else {
        room.bounds.min_z
    };
    clear_cuboid(
        world,
        VoxelCoord::new(center_x - 2, 1, door_z),
        VoxelCoord::new(center_x + 2, 5, door_z),
    );
    clear_cuboid(
        world,
        VoxelCoord::new(
            center_x - 2,
            1,
            if door_z < 0 { door_z + 1 } else { door_z - 1 },
        ),
        VoxelCoord::new(
            center_x + 2,
            5,
            if door_z < 0 { door_z + 1 } else { door_z - 1 },
        ),
    );
    fill_cuboid(
        world,
        VoxelCoord::new(center_x - 2, 6, door_z),
        VoxelCoord::new(center_x + 2, 6, door_z),
        VoxelMaterial::Wood,
    );
}

fn stamp_liminal_sign(world: &mut VoxelWorld, room: &LiminalRoom) {
    let center_x = (room.bounds.min_x + room.bounds.max_x) / 2;
    let sign_z = if room.bounds.max_z < 0 {
        room.bounds.max_z + 1
    } else {
        room.bounds.min_z - 1
    };
    let material = if room.sign_text == room.original_sign_text {
        VoxelMaterial::Beacon
    } else {
        VoxelMaterial::Glass
    };
    fill_cuboid(
        world,
        VoxelCoord::new(center_x - 3, 6, sign_z),
        VoxelCoord::new(center_x + 3, 6, sign_z),
        material,
    );
}

fn stamp_liminal_room_props(world: &mut VoxelWorld, room: &LiminalRoom) {
    let center = room.bounds.center();
    match room.room_type {
        LiminalRoomType::Office => {
            fill_cuboid(
                world,
                VoxelCoord::new(center.x as i32 - 4, 2, center.z as i32 - 3),
                VoxelCoord::new(center.x as i32 + 4, 3, center.z as i32 + 1),
                VoxelMaterial::Wood,
            );
        }
        LiminalRoomType::ConferenceRoom => {
            fill_cuboid(
                world,
                VoxelCoord::new(center.x as i32 - 7, 2, center.z as i32 - 2),
                VoxelCoord::new(center.x as i32 + 7, 3, center.z as i32 + 2),
                VoxelMaterial::Wood,
            );
        }
        LiminalRoomType::Bathroom => {
            for x in [
                room.bounds.min_x + 4,
                room.bounds.min_x + 8,
                room.bounds.min_x + 12,
            ] {
                fill_cuboid(
                    world,
                    VoxelCoord::new(x, 1, room.bounds.min_z + 2),
                    VoxelCoord::new(x + 1, 5, room.bounds.min_z + 7),
                    VoxelMaterial::Glass,
                );
            }
        }
        LiminalRoomType::BreakRoom => {
            fill_cuboid(
                world,
                VoxelCoord::new(room.bounds.max_x - 5, 1, room.bounds.min_z + 2),
                VoxelCoord::new(room.bounds.max_x - 2, 4, room.bounds.max_z - 2),
                VoxelMaterial::Habitat,
            );
            stamp_bottle(world, center.x as i32 - 3, 4, center.z as i32);
        }
        LiminalRoomType::UtilityRoom => {
            fill_cuboid(
                world,
                VoxelCoord::new(room.bounds.min_x + 2, 1, room.bounds.min_z + 2),
                VoxelCoord::new(room.bounds.min_x + 5, 6, room.bounds.max_z - 2),
                VoxelMaterial::ShipHull,
            );
        }
        LiminalRoomType::Hallway => {}
    }

    if let Some(chair) = &room.chair {
        stamp_chair(
            world,
            chair.position.x.floor() as i32,
            chair.position.z.floor() as i32,
            chair.facing,
        );
    }
}

fn stamp_liminal_light(world: &mut VoxelWorld, light: &LiminalLight) {
    let material = if light.repaired {
        VoxelMaterial::Beacon
    } else {
        VoxelMaterial::Glass
    };
    fill_cuboid(
        world,
        VoxelCoord::new(
            light.position.x.floor() as i32 - 1,
            LIMINAL_ROOM_HEIGHT,
            light.position.z.floor() as i32,
        ),
        VoxelCoord::new(
            light.position.x.floor() as i32 + 1,
            LIMINAL_ROOM_HEIGHT,
            light.position.z.floor() as i32,
        ),
        material,
    );
}

fn rotate_facing_clockwise(facing: BarFacing) -> BarFacing {
    match facing {
        BarFacing::North => BarFacing::East,
        BarFacing::East => BarFacing::South,
        BarFacing::South => BarFacing::West,
        BarFacing::West => BarFacing::North,
    }
}

fn build_zombies_map(state: &ZombiesState) -> VoxelWorld {
    let mut world = VoxelWorld::new();

    fill_cuboid(
        &mut world,
        VoxelCoord::new(-84, 0, -84),
        VoxelCoord::new(84, 0, 84),
        VoxelMaterial::Regolith,
    );
    stamp_zombies_boundary(&mut world);
    stamp_zombies_street(&mut world);
    stamp_zombies_building(&mut world, state);
    stamp_zombies_corn_field(&mut world, state);
    stamp_zombies_wall_weapon(&mut world, &state.wall_weapon);

    world
}

fn stamp_zombies_boundary(world: &mut VoxelWorld) {
    fill_cuboid(
        world,
        VoxelCoord::new(-84, 1, -84),
        VoxelCoord::new(84, 10, -82),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-84, 1, 82),
        VoxelCoord::new(84, 10, 84),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-84, 1, -84),
        VoxelCoord::new(-82, 10, 84),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(82, 1, -84),
        VoxelCoord::new(84, 10, 84),
        VoxelMaterial::Basalt,
    );
}

fn stamp_zombies_street(world: &mut VoxelWorld) {
    fill_cuboid(
        world,
        VoxelCoord::new(-76, 0, -76),
        VoxelCoord::new(74, 0, -38),
        VoxelMaterial::Stone,
    );
    for x in (-66..=66).step_by(18) {
        fill_cuboid(
            world,
            VoxelCoord::new(x, 1, -58),
            VoxelCoord::new(x + 5, 3, -52),
            VoxelMaterial::ShipHull,
        );
        fill_cuboid(
            world,
            VoxelCoord::new(x + 1, 4, -57),
            VoxelCoord::new(x + 4, 5, -53),
            VoxelMaterial::Glass,
        );
    }
    for x in (-70..=70).step_by(14) {
        fill_cuboid(
            world,
            VoxelCoord::new(x, 1, -72),
            VoxelCoord::new(x + 1, 8, -72),
            VoxelMaterial::ShipHull,
        );
        world.set(
            VoxelCoord::new(x, 9, -72),
            VoxelCell::new(VoxelMaterial::Beacon),
        );
    }
}

fn stamp_zombies_building(world: &mut VoxelWorld, state: &ZombiesState) {
    fill_cuboid(
        world,
        VoxelCoord::new(-28, 1, -28),
        VoxelCoord::new(42, 11, -26),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-28, 1, 34),
        VoxelCoord::new(42, 11, 36),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-28, 1, -28),
        VoxelCoord::new(-26, 11, 36),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(40, 1, -28),
        VoxelCoord::new(42, 11, 36),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(-24, 0, -24),
        VoxelCoord::new(38, 0, 32),
        VoxelMaterial::Habitat,
    );
    clear_cuboid(
        world,
        VoxelCoord::new(-4, 1, -28),
        VoxelCoord::new(6, 9, -25),
    );
    if !door_open(state, ZombiesDoorKind::Building) {
        fill_cuboid(
            world,
            VoxelCoord::new(-4, 1, -27),
            VoxelCoord::new(6, 8, -26),
            VoxelMaterial::Glass,
        );
    }

    stamp_table(world, -12, -8);
    stamp_table(world, 14, 10);
    stamp_jukebox_at(world, -22, 1, 25);
    stamp_dart_board_at(world, -27, 7, 15);
    stamp_bottle(world, 30, 1, -14);
    stamp_ash_tray(world, 7, 1, 22);
    fill_cuboid(
        world,
        VoxelCoord::new(26, 1, -22),
        VoxelCoord::new(34, 5, -12),
        VoxelMaterial::Habitat,
    );
}

fn stamp_zombies_corn_field(world: &mut VoxelWorld, state: &ZombiesState) {
    fill_cuboid(
        world,
        VoxelCoord::new(-80, 1, 2),
        VoxelCoord::new(-38, 9, 4),
        VoxelMaterial::Wood,
    );
    clear_cuboid(
        world,
        VoxelCoord::new(-55, 1, 2),
        VoxelCoord::new(-45, 8, 4),
    );
    if !door_open(state, ZombiesDoorKind::CornField) {
        fill_cuboid(
            world,
            VoxelCoord::new(-55, 1, 3),
            VoxelCoord::new(-45, 8, 4),
            VoxelMaterial::Wood,
        );
    }

    for z in (10..=76).step_by(5) {
        for x in (-78..=-36).step_by(4) {
            if (x + z) % 11 != 0 {
                stamp_corn_stalk(world, x, z, 10 + (hash_i32_pair(x, z) % 5) as i32);
            }
        }
    }
}

fn stamp_zombies_wall_weapon(world: &mut VoxelWorld, wall_weapon: &WallWeapon) {
    let x = wall_weapon.position.x.floor() as i32;
    let z = wall_weapon.position.z.floor() as i32;
    let material = if wall_weapon.bought {
        VoxelMaterial::Glass
    } else {
        VoxelMaterial::Beacon
    };
    fill_cuboid(
        world,
        VoxelCoord::new(x - 4, 4, z),
        VoxelCoord::new(x + 4, 5, z),
        material,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x + 1, 2, z),
        VoxelCoord::new(x + 4, 3, z),
        material,
    );
}

fn door_open(state: &ZombiesState, kind: ZombiesDoorKind) -> bool {
    state
        .doors
        .iter()
        .any(|door| door.kind == kind && door.open)
}

fn clear_zombies_door(world: &mut VoxelWorld, kind: ZombiesDoorKind) {
    match kind {
        ZombiesDoorKind::Building => clear_cuboid(
            world,
            VoxelCoord::new(-4, 1, -28),
            VoxelCoord::new(6, 9, -25),
        ),
        ZombiesDoorKind::CornField => clear_cuboid(
            world,
            VoxelCoord::new(-55, 1, 2),
            VoxelCoord::new(-45, 8, 4),
        ),
    }
}

fn stamp_jukebox_at(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    fill_cuboid(
        world,
        VoxelCoord::new(x - 4, y, z - 3),
        VoxelCoord::new(x + 4, y + 8, z + 4),
        VoxelMaterial::Habitat,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x - 3, y + 4, z - 4),
        VoxelCoord::new(x + 3, y + 7, z - 4),
        VoxelMaterial::Glass,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x - 2, y + 6, z - 5),
        VoxelCoord::new(x + 2, y + 6, z - 5),
        VoxelMaterial::Beacon,
    );
}

fn stamp_dart_board_at(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    for dy in -3_i32..=3 {
        for dz in -3_i32..=3 {
            let distance = dy.abs() + dz.abs();
            if distance <= 4 {
                world.set(
                    VoxelCoord::new(x, y + dy, z + dz),
                    VoxelCell::new(if distance <= 1 {
                        VoxelMaterial::Beacon
                    } else if distance == 2 {
                        VoxelMaterial::Glass
                    } else {
                        VoxelMaterial::Habitat
                    }),
                );
            }
        }
    }
}

fn build_voxel_sandbox_world() -> VoxelWorld {
    let mut world = VoxelWorld::new();

    for z in -SANDBOX_HALF_EXTENT..=SANDBOX_HALF_EXTENT {
        for x in -SANDBOX_HALF_EXTENT..=SANDBOX_HALF_EXTENT {
            let height = sandbox_height_at(x, z);
            let top_material = sandbox_surface_material(x, z, height);
            for y in 0..=height {
                let material = if y == height {
                    top_material
                } else if y >= height - 2 {
                    VoxelMaterial::Dirt
                } else {
                    VoxelMaterial::Stone
                };
                world.set(VoxelCoord::new(x, y, z), VoxelCell::new(material));
            }

            if should_place_tree(x, z, height) {
                stamp_sandbox_tree(&mut world, x, height + 1, z);
            }
        }
    }

    world
}

fn sandbox_height_at(x: i32, z: i32) -> i32 {
    let ridge = ((x as f32 * 0.11).sin() * 2.4 + (z as f32 * 0.09).cos() * 2.0).round() as i32;
    let rough = ((hash_i32_pair(x.div_euclid(4), z.div_euclid(4)) % 5) as i32) - 2;
    (5 + ridge + rough).clamp(1, 13)
}

fn sandbox_surface_material(x: i32, z: i32, height: i32) -> VoxelMaterial {
    if height <= 3 {
        VoxelMaterial::Sand
    } else if hash_i32_pair(x, z) % 23 == 0 {
        VoxelMaterial::Regolith
    } else {
        VoxelMaterial::Grass
    }
}

fn should_place_tree(x: i32, z: i32, height: i32) -> bool {
    height > 4 && x.abs() > 8 && z.abs() > 8 && hash_i32_pair(x, z) % 89 == 0
}

fn stamp_sandbox_tree(world: &mut VoxelWorld, x: i32, y: i32, z: i32) {
    for trunk_y in y..=(y + 4) {
        world.set(
            VoxelCoord::new(x, trunk_y, z),
            VoxelCell::new(VoxelMaterial::Wood),
        );
    }

    for leaf_y in (y + 3)..=(y + 6) {
        let radius = if leaf_y == y + 6 { 1 } else { 2 };
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dz * dz <= radius * radius + 1 {
                    world.set(
                        VoxelCoord::new(x + dx, leaf_y, z + dz),
                        VoxelCell::new(VoxelMaterial::Leaves),
                    );
                }
            }
        }
    }
}

fn hash_i32_pair(x: i32, z: i32) -> u32 {
    let mut hash = x as u32;
    hash = hash.wrapping_mul(0x85eb_ca6b) ^ z as u32;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^ (hash >> 13)
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

fn build_asset_catalog() -> Vec<PreviewAsset> {
    let mut assets = vec![
        PreviewAsset::new(
            "standing bar patron",
            build_person_asset(BarPose::Standing, VoxelMaterial::SiliconLife),
        ),
        PreviewAsset::new(
            "seated bar patron",
            build_person_asset(BarPose::Sitting, VoxelMaterial::Glass),
        ),
        PreviewAsset::new("corn stalk", build_corn_stalk_asset()),
        PreviewAsset::new("table set", build_table_set_asset()),
        PreviewAsset::new("jukebox", build_jukebox_asset()),
        PreviewAsset::new("dart board", build_dart_board_asset()),
        PreviewAsset::new("bottle", build_bottle_asset()),
        PreviewAsset::new("ash tray", build_ash_tray_asset()),
        PreviewAsset::new("zombie", build_zombie_asset()),
    ];

    for (name, material) in [
        ("grass block", VoxelMaterial::Grass),
        ("dirt block", VoxelMaterial::Dirt),
        ("stone block", VoxelMaterial::Stone),
        ("sand block", VoxelMaterial::Sand),
        ("wood block", VoxelMaterial::Wood),
        ("leaf block", VoxelMaterial::Leaves),
        ("glass block", VoxelMaterial::Glass),
        ("beacon block", VoxelMaterial::Beacon),
    ] {
        assets.push(PreviewAsset::new(name, build_block_asset(material)));
    }

    assets.push(PreviewAsset::new("gun", build_weapon_asset()));
    assets.push(PreviewAsset::new(
        "drone race gate",
        build_drone_gate_asset(),
    ));

    assets
}

fn asset_bounds(world: &VoxelWorld) -> (Vec3, f32) {
    let Some(bounds) = world.bounds() else {
        return (Vec3::ZERO, ASSET_VIEWER_MIN_DISTANCE);
    };
    let center = Vec3::new(
        (bounds.min.x + bounds.max.x + 1) as f32 * 0.5,
        (bounds.min.y + bounds.max.y + 1) as f32 * 0.5,
        (bounds.min.z + bounds.max.z + 1) as f32 * 0.5,
    );
    let extents = Vec3::new(
        (bounds.max.x - bounds.min.x + 1) as f32,
        (bounds.max.y - bounds.min.y + 1) as f32,
        (bounds.max.z - bounds.min.z + 1) as f32,
    );
    (center, extents.length() * 0.5)
}

fn asset_viewer_start_camera(asset: &PreviewAsset, distance: f32) -> Camera {
    look_at(
        asset.center + Vec3::new(0.0, 0.2_f32.sin() * distance, -0.2_f32.cos() * distance),
        asset.center,
    )
    .with_fov_y(48.0_f32.to_radians())
    .with_max_distance((distance + asset.radius * 3.0).max(60.0))
}

fn build_person_asset(pose: BarPose, accent: VoxelMaterial) -> VoxelWorld {
    let mut world = VoxelWorld::new();
    let index = match accent {
        VoxelMaterial::SiliconLife => 0,
        VoxelMaterial::Glass => 1,
        _ => 2,
    };
    stamp_bar_person(&mut world, 0, 0, BarFacing::South, pose, index);
    world
}

fn build_block_asset(material: VoxelMaterial) -> VoxelWorld {
    let mut world = VoxelWorld::new();
    for z in -2..=1 {
        for y in 0..=3 {
            for x in -2..=1 {
                world.set(VoxelCoord::new(x, y, z), VoxelCell::new(material));
            }
        }
    }
    world
}

fn build_corn_stalk_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_corn_stalk(&mut world, 0, 0, CORN_STALK_BASE_HEIGHT + 4);
    world
}

fn build_table_set_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_table(&mut world, 0, 0);
    stamp_chair(&mut world, -7, 0, BarFacing::East);
    stamp_chair(&mut world, 7, 0, BarFacing::West);
    stamp_chair(&mut world, 0, -7, BarFacing::South);
    world
}

fn build_jukebox_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-5, 1, -4),
        VoxelCoord::new(5, 11, 5),
        VoxelMaterial::Habitat,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-4, 6, -5),
        VoxelCoord::new(4, 11, -5),
        VoxelMaterial::Glass,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-3, 8, -6),
        VoxelCoord::new(3, 9, -6),
        VoxelMaterial::Beacon,
    );
    for x in [-3, 0, 3] {
        fill_cuboid(
            &mut world,
            VoxelCoord::new(x, 2, -6),
            VoxelCoord::new(x, 4, -6),
            VoxelMaterial::ShipHull,
        );
    }
    world
}

fn build_dart_board_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    for y in 1_i32..=9 {
        for x in -4_i32..=4 {
            let distance = (y - 5).abs() + x.abs();
            if distance <= 5 {
                let material = if distance <= 1 {
                    VoxelMaterial::Beacon
                } else if distance == 2 {
                    VoxelMaterial::Glass
                } else {
                    VoxelMaterial::Habitat
                };
                world.set(VoxelCoord::new(x, y, 0), VoxelCell::new(material));
            }
        }
    }
    world
}

fn build_bottle_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_bottle(&mut world, 0, 1, 0);
    world
}

fn build_ash_tray_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_ash_tray(&mut world, 0, 1, 0);
    world
}

fn build_zombie_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_zombie_body(&mut world, Vec3::ZERO, VoxelMaterial::CarbonLife);
    world
}

fn build_drone_gate_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();
    stamp_drone_gate(
        &mut world,
        Vec3::new(0.0, DRONE_GATE_FRAME_RADIUS as f32 + 2.0, 0.0),
        DroneGateRunnerConfig::default(),
        true,
        false,
    );
    world
}

fn build_weapon_asset() -> VoxelWorld {
    let mut world = VoxelWorld::new();

    fill_cuboid(
        &mut world,
        VoxelCoord::new(-18, 2, 0),
        VoxelCoord::new(-9, 8, 4),
        VoxelMaterial::CarbonLife,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-10, 4, 0),
        VoxelCoord::new(1, 7, 5),
        VoxelMaterial::CarbonLife,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-5, 3, 0),
        VoxelCoord::new(7, 10, 5),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(2, 7, 0),
        VoxelCoord::new(28, 10, 3),
        VoxelMaterial::ShipHull,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(10, 10, 1),
        VoxelCoord::new(25, 13, 4),
        VoxelMaterial::ShipHull,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(12, 9, -1),
        VoxelCoord::new(20, 11, 1),
        VoxelMaterial::Glass,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(24, 8, 0),
        VoxelCoord::new(32, 9, 3),
        VoxelMaterial::Beacon,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(30, 7, 1),
        VoxelCoord::new(38, 8, 2),
        VoxelMaterial::Beacon,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-2, 10, 1),
        VoxelCoord::new(4, 13, 4),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-5, 1, 2),
        VoxelCoord::new(0, 6, 5),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-15, 0, 2),
        VoxelCoord::new(-6, 4, 5),
        VoxelMaterial::CarbonLife,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-20, 0, 1),
        VoxelCoord::new(-14, 3, 4),
        VoxelMaterial::CarbonLife,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-3, 0, 0),
        VoxelCoord::new(8, 2, 2),
        VoxelMaterial::CarbonLife,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(1, 2, 1),
        VoxelCoord::new(5, 4, 4),
        VoxelMaterial::CarbonLife,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(8, 6, 4),
        VoxelCoord::new(28, 8, 5),
        VoxelMaterial::ShipHull,
    );
    clear_cuboid(
        &mut world,
        VoxelCoord::new(-4, 4, 2),
        VoxelCoord::new(1, 6, 3),
    );
    clear_cuboid(
        &mut world,
        VoxelCoord::new(5, 9, 1),
        VoxelCoord::new(8, 9, 2),
    );

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

fn clear_cuboid(world: &mut VoxelWorld, a: VoxelCoord, b: VoxelCoord) {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    let min_z = a.z.min(b.z);
    let max_z = a.z.max(b.z);

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                world.clear(VoxelCoord::new(x, y, z));
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

fn zombies_world_with_zombies(base: &VoxelWorld, zombies: &ZombiesState) -> VoxelWorld {
    let mut world = base.clone();
    for zombie in &zombies.zombies {
        if zombie.is_alive() {
            stamp_zombie(&mut world, zombie);
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

fn stamp_zombie(world: &mut VoxelWorld, zombie: &Zombie) {
    let wounded = zombie.health * 2 < zombie.max_health;
    let accent = if wounded {
        VoxelMaterial::Beacon
    } else {
        VoxelMaterial::CarbonLife
    };
    stamp_zombie_body(world, zombie.position, accent);
}

fn stamp_zombie_body(world: &mut VoxelWorld, position: Vec3, accent: VoxelMaterial) {
    let origin = VoxelCoord::new(position.x.floor() as i32, 0, position.z.floor() as i32);
    for (x, y, z, material) in ZOMBIE_BODY_OFFSETS {
        let material = if material == VoxelMaterial::Beacon {
            accent
        } else {
            material
        };
        world.set(
            VoxelCoord::new(origin.x + x, origin.y + y, origin.z + z),
            VoxelCell::new(material),
        );
    }
}

fn zombie_body_contains_voxel(position: Vec3, coord: VoxelCoord) -> bool {
    let origin = VoxelCoord::new(position.x.floor() as i32, 0, position.z.floor() as i32);
    ZOMBIE_BODY_OFFSETS
        .iter()
        .any(|(x, y, z, _)| VoxelCoord::new(origin.x + *x, origin.y + *y, origin.z + *z) == coord)
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
        y: 57,
        z: 10,
        text: "6  ASSET VIEWER".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 61,
        z: 10,
        text: "7  VOXEL SANDBOX".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 65,
        z: 10,
        text: "8  HELIOBOUND ZOMBIES".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 69,
        z: 10,
        text: "9  LIMINAL OFFICE".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 73,
        z: 10,
        text: "0  DRONE GATE RUNNER".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 80,
        z: 10,
        text: "WASD MOVE   SHIFT BOOST   Q/E ROLL   M MENU".to_string(),
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

fn render_asset_viewer_scene(scene: &mut Scene, viewer: &AssetViewerState, mouse_captured: bool) {
    let asset = viewer.selected_asset();
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "ASSET VIEWER  {} / {}  {}  zoom {:.1}  mouse {}  M menu",
            viewer.selected + 1,
            viewer.assets.len(),
            asset.name,
            viewer.distance,
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: "1-9 select  N/P cycle  A/D yaw  W/S pitch  Q/E roll  Space/Ctrl zoom".to_string(),
        style: TextStyle::default(),
    });
    for (index, preview) in viewer.assets.iter().enumerate() {
        let marker = if index == viewer.selected { '>' } else { ' ' };
        scene.overlays.push(Overlay {
            x: 2,
            y: 9 + index as i32,
            z: 120,
            text: format!("{}{} {}", marker, index + 1, preview.name),
            style: TextStyle::default(),
        });
    }
}

fn render_voxel_sandbox_scene(
    scene: &mut Scene,
    sandbox: &VoxelSandboxState,
    mouse_captured: bool,
) {
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "VOXEL SANDBOX  block {} {}  mouse {}  M menu",
            sandbox.selected_block + 1,
            material_label(sandbox.selected_material()),
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 4,
        z: 120,
        text: sandbox_palette_text(sandbox),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 6,
        z: 120,
        text: "WASD move  Space/Ctrl rise/drop  Shift boost  left remove  right place".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: VIEWPORT.width as i32 / 2,
        y: VIEWPORT.height as i32 / 2,
        z: 150,
        text: "+".to_string(),
        style: TextStyle::default(),
    });
}

fn sandbox_palette_text(sandbox: &VoxelSandboxState) -> String {
    sandbox
        .palette
        .iter()
        .enumerate()
        .map(|(index, material)| {
            if index == sandbox.selected_block {
                format!("[{}:{}]", index + 1, material_label(*material))
            } else {
                format!("{}:{}", index + 1, material_label(*material))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn material_label(material: VoxelMaterial) -> &'static str {
    match material {
        VoxelMaterial::Regolith => "regolith",
        VoxelMaterial::Basalt => "basalt",
        VoxelMaterial::Ocean => "water",
        VoxelMaterial::Ice => "ice",
        VoxelMaterial::Grass => "grass",
        VoxelMaterial::Dirt => "dirt",
        VoxelMaterial::Stone => "stone",
        VoxelMaterial::Sand => "sand",
        VoxelMaterial::Wood => "wood",
        VoxelMaterial::Leaves => "leaves",
        VoxelMaterial::Zombie => "zombie",
        VoxelMaterial::CornStalk => "corn",
        VoxelMaterial::CarbonLife => "carbon",
        VoxelMaterial::SiliconLife => "silicon",
        VoxelMaterial::Habitat => "habitat",
        VoxelMaterial::ShipHull => "hull",
        VoxelMaterial::Glass => "glass",
        VoxelMaterial::Beacon => "beacon",
        VoxelMaterial::Gate => "gate",
    }
}

fn render_drone_gate_runner_scene(
    scene: &mut Scene,
    runner: &DroneGateRunnerState,
    mouse_captured: bool,
) {
    let active = runner.active_gate_position().unwrap_or(Vec3::ZERO);
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
            "DRONE GATE RUNNER  map {}  gate {}/{}  mouse {}",
            runner.course.name,
            runner.active_gate + 1,
            runner.course.gates.len(),
            if mouse_captured { "locked" } else { "free" }
        ),
        style: hud_style(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: format!(
            "laps {}  passed {}  time {:05.1}  next {:.0},{:.0},{:.0}",
            runner.laps, runner.passed_gates, runner.elapsed, active.x, active.y, active.z
        ),
        style: hud_style(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 8,
        z: 120,
        text: "WASD strafe/forward  Space/Ctrl vertical  Shift boost  Q/E roll  M menu".to_string(),
        style: hud_style(),
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

fn render_liminal_scene(scene: &mut Scene, liminal: &LiminalState, mouse_captured: bool) {
    let hud = hud_style();
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "LIMINAL OFFICE  room {}  type {}  mouse {}",
            liminal.current_room_label(),
            liminal.current_room_type_label(),
            if mouse_captured { "locked" } else { "free" }
        ),
        style: hud.clone(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: format!(
            "seed {:016x}  wrongness {:04.1}  graph {}r/{}c  anomalies {}",
            liminal.seed,
            liminal.wrongness,
            liminal.graph.rooms.len(),
            liminal.graph.connection_count(),
            liminal
                .anomaly_manager
                .triggered
                .iter()
                .map(|kind| kind.label())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        style: hud.clone(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 8,
        z: 120,
        text: format!(
            "objective: {}",
            liminal.objective.description(&liminal.graph)
        ),
        style: hud.clone(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 11,
        z: 120,
        text: format!(
            "F repair near target light  T force anomaly  {}",
            liminal.debug_message
        ),
        style: hud,
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
    weapon_asset: &PreviewAsset,
    bob_offset: (i32, i32),
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
        cells: weapon_viewmodel_cells(
            scene.viewport,
            weapon_asset,
            shooter.shot_flash_timer > 0.0,
            bob_offset,
        ),
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

fn render_zombies_scene(
    scene: &mut Scene,
    camera: &Camera,
    zombies: &ZombiesState,
    weapon_asset: &PreviewAsset,
    bob_offset: (i32, i32),
    mouse_captured: bool,
) {
    scene.layers.push(Layer {
        name: "bullets".to_string(),
        z: 35,
        cells: bullet_cells(scene.viewport, camera, &zombies.bullet_traces),
    });
    scene.layers.push(Layer {
        name: "weapon".to_string(),
        z: 40,
        cells: weapon_viewmodel_cells(
            scene.viewport,
            weapon_asset,
            zombies.shot_flash_timer > 0.0,
            bob_offset,
        ),
    });
    scene.layers.push(Layer {
        name: "reticle".to_string(),
        z: 50,
        cells: reticle_cells(scene.viewport),
    });
    if zombies.damage_flash_timer > 0.0 {
        scene.layers.push(Layer {
            name: "damage".to_string(),
            z: 60,
            cells: damage_flash_cells(scene.viewport),
        });
    }

    let hud = hud_style();
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "ROUND {}  ZOMBIES {}  SPAWNING {}  M menu",
            zombies.round,
            zombies.alive_count(),
            zombies.queued_spawns
        ),
        style: hud.clone(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: format!(
            "HITS LEFT {}  SPRINT {:>3}%{}  MOUSE {}",
            zombies.health_left(),
            (zombies.sprint * 100.0).round() as i32,
            if zombies.sprint_locked { " LOCKED" } else { "" },
            if mouse_captured { "locked" } else { "free" }
        ),
        style: hud.clone(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 8,
        z: 120,
        text: format!(
            "POINTS {}  KILLS {}  TOTAL {}",
            zombies.points, zombies.kills, zombies.total_points
        ),
        style: hud.clone(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 11,
        z: 120,
        text: format!(
            "{}  AMMO {}/{}  F buy/open  R reload",
            zombies.weapon.label(),
            zombies.ammo_in_mag,
            zombies.ammo_reserve
        ),
        style: hud.clone(),
    });

    if zombies.game_over {
        let center_x = scene.viewport.width as i32 / 2 - 21;
        let center_y = scene.viewport.height as i32 / 2 - 5;
        for (offset, text) in [
            "GAME OVER".to_string(),
            format!("ROUNDS SURVIVED {}", zombies.rounds_survived()),
            format!("KILLS {}", zombies.kills),
            format!("TOTAL POINTS {}", zombies.total_points),
            "M MENU".to_string(),
        ]
        .into_iter()
        .enumerate()
        {
            scene.overlays.push(Overlay {
                x: center_x,
                y: center_y + offset as i32 * 3,
                z: 180,
                text,
                style: hud.clone(),
            });
        }
    }
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

fn weapon_viewmodel_cells(
    viewport: Viewport,
    weapon_asset: &PreviewAsset,
    flash: bool,
    bob_offset: (i32, i32),
) -> Vec<SceneCell> {
    let (bob_x, bob_y) = bob_offset;
    let render_width = WEAPON_VIEW_RENDER_WIDTH;
    let render_height = WEAPON_VIEW_RENDER_HEIGHT;
    let start_x = viewport.width as i32 - render_width as i32 - WEAPON_VIEW_SCREEN_RIGHT_MARGIN
        + WEAPON_VIEW_SCREEN_SHIFT_X
        + bob_x;
    let start_y = viewport.height as i32 - render_height as i32 - WEAPON_VIEW_SCREEN_BOTTOM_MARGIN
        + WEAPON_VIEW_SCREEN_SHIFT_Y
        + bob_y;
    let camera = weapon_viewmodel_camera(weapon_asset);
    let material_map = MaterialGlyphMap;
    let mut cells = Vec::new();

    for y in 0..render_height {
        for x in 0..render_width {
            let ray = camera.ray_for_cell(x, y, render_width, render_height);
            if let Some(hit) = raycast(
                &weapon_asset.world,
                ray,
                (weapon_asset.radius * WEAPON_VIEW_CAMERA_DISTANCE_SCALE).max(64.0),
            ) {
                cells.push(SceneCell {
                    x: start_x + x as i32,
                    y: start_y + y as i32,
                    glyph: material_map.glyph_for(hit),
                    style: material_map.style_for(hit),
                });
            }
        }
    }

    if flash {
        add_weapon_muzzle_flash(&mut cells, start_x, start_y, render_width, render_height);
    }

    cells
}

fn weapon_viewmodel_camera(asset: &PreviewAsset) -> Camera {
    let radius = asset.radius.max(8.0);
    let eye = asset.center
        + Vec3::new(
            radius * WEAPON_VIEW_CAMERA_RIGHT,
            radius * WEAPON_VIEW_CAMERA_UP,
            -radius * WEAPON_VIEW_CAMERA_BACK,
        );
    let target = asset.center
        + Vec3::new(
            -radius * WEAPON_VIEW_TARGET_LEFT,
            radius * WEAPON_VIEW_TARGET_UP,
            radius * WEAPON_VIEW_TARGET_FORWARD,
        );
    look_at(eye, target)
        .with_fov_y(WEAPON_VIEW_CAMERA_FOV.to_radians())
        .with_max_distance((radius * WEAPON_VIEW_CAMERA_DISTANCE_SCALE).max(64.0))
}

fn add_weapon_muzzle_flash(
    cells: &mut Vec<SceneCell>,
    start_x: i32,
    start_y: i32,
    width: usize,
    height: usize,
) {
    let tip_x = start_x + width as i32 - 6;
    let tip_y = start_y + height as i32 / 2 - 1;
    for (dx, dy, glyph) in [(0, 0, '*'), (1, 0, '+'), (0, 1, '+'), (2, -1, '*')] {
        cells.push(SceneCell {
            x: tip_x + dx,
            y: tip_y + dy,
            glyph,
            style: TextStyle {
                fg: Some("#ffda63".to_string()),
                bg: None,
                bold: true,
            },
        });
    }
}

fn damage_flash_cells(viewport: Viewport) -> Vec<SceneCell> {
    let mut cells = Vec::new();
    for y in 0..viewport.height as i32 {
        for x in 0..viewport.width as i32 {
            let near_edge =
                x < 5 || y < 4 || x >= viewport.width as i32 - 5 || y >= viewport.height as i32 - 4;
            if near_edge && (x + y) % 2 == 0 {
                cells.push(SceneCell {
                    x,
                    y,
                    glyph: '#',
                    style: TextStyle {
                        fg: Some("#b1182b".to_string()),
                        bg: None,
                        bold: true,
                    },
                });
            }
        }
    }
    cells
}

fn hud_style() -> TextStyle {
    TextStyle {
        fg: Some("#f3ead2".to_string()),
        bg: Some("#000000".to_string()),
        bold: true,
    }
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
            "damage" => [0xb1, 0x18, 0x2b, 0xff],
            _ => [0xe6, 0xee, 0xf3, 0xff],
        };
        let opaque_cells = layer_cells_are_opaque(layer.name.as_str());

        for cell in &layer.cells {
            if opaque_cells && cell.glyph != ' ' {
                fill_rect(
                    frame,
                    width,
                    height,
                    cell.x * CHAR_WIDTH as i32,
                    cell.y * CHAR_HEIGHT as i32,
                    CHAR_WIDTH as i32,
                    CHAR_HEIGHT as i32,
                    [0x08, 0x0b, 0x10, 0xff],
                );
            }
            draw_glyph(
                frame,
                width,
                height,
                cell.x,
                cell.y,
                cell.glyph,
                style_color(&cell.style).unwrap_or(color),
            );
        }
    }

    for overlay in &scene.overlays {
        if let Some(bg) = style_bg_color(&overlay.style) {
            fill_rect(
                frame,
                width,
                height,
                overlay.x * CHAR_WIDTH as i32 - 4,
                overlay.y * CHAR_HEIGHT as i32 - 2,
                overlay.text.chars().count() as i32 * CHAR_WIDTH as i32 + 8,
                CHAR_HEIGHT as i32 + 4,
                bg,
            );
        }
        draw_text(
            frame,
            width,
            height,
            overlay.x,
            overlay.y,
            &overlay.text,
            style_color(&overlay.style).unwrap_or([0xf0, 0xc6, 0x5b, 0xff]),
        );
    }
}

fn layer_cells_are_opaque(layer_name: &str) -> bool {
    matches!(
        layer_name,
        "voxels" | "planet" | "enemies" | "weapon" | "damage"
    )
}

fn style_color(style: &TextStyle) -> Option<[u8; 4]> {
    style.fg.as_deref().and_then(parse_hex_color)
}

fn style_bg_color(style: &TextStyle) -> Option<[u8; 4]> {
    style.bg.as_deref().and_then(parse_hex_color)
}

fn parse_hex_color(color: &str) -> Option<[u8; 4]> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b, 0xff])
}

fn clear(frame: &mut [u8], color: [u8; 4]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
}

fn fill_rect(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    rect_width: i32,
    rect_height: i32,
    color: [u8; 4],
) {
    for py in y..(y + rect_height) {
        for px in x..(x + rect_width) {
            set_pixel(frame, width, height, px, py, color);
        }
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
    fn menu_can_start_asset_viewer_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit6), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::AssetViewer);
        assert!(!app.asset_viewer.assets.is_empty());
        assert!(app
            .asset_viewer
            .assets
            .iter()
            .any(|asset| asset.name == "gun"));
    }

    #[test]
    fn menu_can_start_voxel_sandbox_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit7), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::VoxelSandbox);
        assert!(app.sandbox.world.voxel_count() > 10_000);
    }

    #[test]
    fn menu_can_start_zombies_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit8), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::Zombies);
        assert_eq!(app.zombies.round, 1);
        assert!(app.zombies_map.voxel_count() > 10_000);
    }

    #[test]
    fn menu_can_start_liminal_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit9), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::Liminal);
        assert_eq!(app.liminal.seed, LIMINAL_SEED);
        assert!(app.liminal.world.voxel_count() > 8_000);
    }

    #[test]
    fn menu_can_start_drone_gate_runner_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit0), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::DroneGateRunner);
        assert_eq!(app.drone_gate_runner.course.seed, DRONE_GATE_SEED);
        assert!(app.camera.max_distance >= DRONE_GATE_VIEW_DISTANCE);
    }

    #[test]
    fn drone_gate_course_generation_is_seeded() {
        let config = DroneGateRunnerConfig::default();
        let a = generate_drone_gate_course(1234, config);
        let b = generate_drone_gate_course(1234, config);
        let c = generate_drone_gate_course(4321, config);

        assert_eq!(a.gates, b.gates);
        assert_ne!(a.gates, c.gates);
        assert!(a.gates.len() >= 10);
    }

    #[test]
    fn drone_runner_advances_when_player_flies_through_active_gate() {
        let mut runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let gate = runner.active_gate_position().unwrap();
        runner.previous_position = Vec3::new(gate.x, gate.y, gate.z - 12.0);

        runner.update(Vec3::new(gate.x, gate.y, gate.z + 12.0), 0.016);

        assert_eq!(runner.active_gate, 1);
        assert_eq!(runner.passed_gates, 1);
    }

    #[test]
    fn drone_runner_uses_unrestricted_flight_controls() {
        let mut app = AppState::new();
        app.start_drone_gate_runner();
        let start = app.camera.position;
        app.input = PlayerInput {
            forward: true,
            roll_left: true,
            ..PlayerInput::default()
        };

        app.frame(1.0, true);

        assert!(app.camera.position != start);
        assert!(app.camera.roll_radians > 0.0);
    }

    #[test]
    fn drone_runner_world_lights_active_gate() {
        let runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let gate = runner.active_gate_position().unwrap();
        let world = runner.render_world();

        assert_eq!(
            world.get(VoxelCoord::new(
                gate.x.round() as i32 + DRONE_GATE_FRAME_RADIUS,
                gate.y.round() as i32,
                gate.z.round() as i32
            )),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
        assert!(world.voxel_count() > 1_000);
    }

    #[test]
    fn liminal_generation_is_seeded_and_has_required_room_types() {
        let a = LiminalState::new_seeded(1234);
        let b = LiminalState::new_seeded(1234);
        let c = LiminalState::new_seeded(5678);

        assert_eq!(a.graph.rooms.len(), b.graph.rooms.len());
        assert_eq!(a.graph.rooms[1].bounds, b.graph.rooms[1].bounds);
        assert_ne!(a.graph.rooms[1].bounds, c.graph.rooms[1].bounds);
        for room_type in [
            LiminalRoomType::Hallway,
            LiminalRoomType::Office,
            LiminalRoomType::ConferenceRoom,
            LiminalRoomType::Bathroom,
            LiminalRoomType::BreakRoom,
            LiminalRoomType::UtilityRoom,
        ] {
            assert!(a.graph.rooms.iter().any(|room| room.room_type == room_type));
        }
        assert!(a
            .graph
            .connections
            .iter()
            .any(|connection| connection.connection_type == LiminalConnectionType::Door));
    }

    #[test]
    fn liminal_detects_room_entry_by_room_id() {
        let mut liminal = LiminalState::new_seeded(LIMINAL_SEED);
        let office_center = liminal
            .graph
            .rooms
            .iter()
            .find(|room| room.room_type == LiminalRoomType::Office)
            .unwrap()
            .bounds
            .center();
        let mut camera = Camera::new(office_center);

        liminal.update_player_room(&mut camera);

        let current = liminal.current_room.unwrap();
        assert_eq!(
            liminal.graph.room(current).unwrap().room_type,
            LiminalRoomType::Office
        );
        assert!(liminal.graph.room(current).unwrap().visited);
    }

    #[test]
    fn liminal_sign_changes_after_player_leaves_room() {
        let mut liminal = LiminalState::new_seeded(LIMINAL_SEED);
        let room_id = 1;
        let room_center = liminal.graph.room(room_id).unwrap().bounds.center();
        let original_sign = liminal.graph.room(room_id).unwrap().sign_text.clone();
        let mut camera = Camera::new(room_center);
        liminal.update_player_room(&mut camera);

        camera.position = liminal.graph.room(0).unwrap().bounds.center();
        liminal.update_player_room(&mut camera);

        let changed_sign = &liminal.graph.room(room_id).unwrap().sign_text;
        assert_ne!(changed_sign, &original_sign);
        assert!(liminal
            .anomaly_manager
            .triggered
            .contains(&LiminalAnomalyKind::RoomSignChange));
    }

    #[test]
    fn liminal_observed_chair_can_rotate_while_outside_room() {
        let mut liminal = LiminalState::new_seeded(LIMINAL_SEED);
        let room_id = liminal
            .graph
            .rooms
            .iter()
            .find(|room| room.chair.is_some())
            .unwrap()
            .id;
        let room_center = liminal.graph.room(room_id).unwrap().bounds.center();
        let mut camera = Camera::new(room_center);
        liminal.update_player_room(&mut camera);
        let before = liminal
            .graph
            .room(room_id)
            .unwrap()
            .chair
            .as_ref()
            .unwrap()
            .facing as u8;

        camera.position = liminal.graph.room(0).unwrap().bounds.center();
        liminal.update_player_room(&mut camera);
        liminal.force_next_anomaly();

        let chair = liminal.graph.room(room_id).unwrap().chair.as_ref().unwrap();
        assert_ne!(chair.facing as u8, before);
        assert!(chair.rotated);
    }

    #[test]
    fn liminal_hallway_loop_wraps_player_at_hall_end() {
        let mut liminal = LiminalState::new_seeded(LIMINAL_SEED);
        liminal.force_next_anomaly();
        liminal.force_next_anomaly();
        liminal.force_next_anomaly();
        let hallway = liminal.graph.room(0).unwrap().bounds;
        let mut camera = Camera::new(Vec3::new(
            hallway.max_x as f32,
            WALK_EYE_HEIGHT,
            hallway.center().z,
        ));
        liminal.current_room = Some(0);

        liminal.update_player_room(&mut camera);

        assert!(camera.position.x < hallway.min_x as f32 + 4.0);
    }

    #[test]
    fn liminal_repairs_target_light_objective() {
        let mut liminal = LiminalState::new_seeded(LIMINAL_SEED);
        let target = liminal.objective.target_room;
        let target_center = liminal.graph.room(target).unwrap().bounds.center();
        let mut camera = Camera::new(target_center);
        liminal.update_player_room(&mut camera);

        liminal.interact(camera.position);

        assert!(liminal.objective.completed);
        assert!(
            liminal
                .graph
                .room(target)
                .unwrap()
                .light
                .as_ref()
                .unwrap()
                .repaired
        );
        assert!(liminal.wrongness > 4.0);
    }

    #[test]
    fn voxel_sandbox_selects_palette_blocks() {
        let mut app = AppState::new();
        app.start_voxel_sandbox();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit4), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::None);
        assert_eq!(app.sandbox.selected_material(), VoxelMaterial::Sand);
    }

    #[test]
    fn voxel_sandbox_removes_center_raycast_block() {
        let mut world = VoxelWorld::new();
        let coord = VoxelCoord::new(0, 2, 0);
        world.set(coord, VoxelCell::new(VoxelMaterial::Stone));
        let mut sandbox = VoxelSandboxState {
            world,
            selected_block: 0,
            palette: vec![VoxelMaterial::Grass],
        };
        let camera = Camera::new(Vec3::new(0.5, 2.5, -3.5)).looking_at(0.0, 0.0);

        sandbox.remove_block(&camera);

        assert_eq!(sandbox.world.get(coord), None);
    }

    #[test]
    fn voxel_sandbox_places_block_on_hit_face() {
        let mut world = VoxelWorld::new();
        world.set(
            VoxelCoord::new(0, 2, 0),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        let mut sandbox = VoxelSandboxState {
            world,
            selected_block: 0,
            palette: vec![VoxelMaterial::Wood],
        };
        let camera = Camera::new(Vec3::new(0.5, 2.5, -3.5)).looking_at(0.0, 0.0);

        sandbox.place_block(&camera);

        assert_eq!(
            sandbox.world.get(VoxelCoord::new(0, 2, -1)),
            Some(VoxelCell::new(VoxelMaterial::Wood))
        );
    }

    #[test]
    fn asset_viewer_catalog_includes_sandbox_blocks() {
        let viewer = AssetViewerState::new();

        assert!(viewer
            .assets
            .iter()
            .any(|asset| asset.name == "grass block"));
        assert!(viewer
            .assets
            .iter()
            .any(|asset| asset.name == "beacon block"));
        assert!(viewer.assets.iter().any(|asset| asset.name == "zombie"));
    }

    #[test]
    fn asset_viewer_can_cycle_to_block_assets() {
        let mut viewer = AssetViewerState::new();

        for _ in 0..9 {
            viewer.select_next();
        }

        assert_eq!(viewer.selected_asset().name, "grass block");
        viewer.select_previous();
        assert_eq!(viewer.selected_asset().name, "zombie");
    }

    #[test]
    fn parses_hex_text_style_color_for_window_renderer() {
        assert_eq!(parse_hex_color("#67b847"), Some([0x67, 0xb8, 0x47, 0xff]));
        assert_eq!(parse_hex_color("67b847"), None);
    }

    #[test]
    fn hud_style_has_black_background_for_readability() {
        assert_eq!(style_bg_color(&hud_style()), Some([0, 0, 0, 0xff]));
    }

    #[test]
    fn opaque_voxel_cells_clear_background_stars() {
        let mut scene = Scene::new(Viewport {
            width: 1,
            height: 1,
        });
        scene.layers.push(Layer {
            name: "background".to_string(),
            z: 0,
            cells: vec![SceneCell {
                x: 0,
                y: 0,
                glyph: '*',
                style: TextStyle::default(),
            }],
        });
        scene.layers.push(Layer {
            name: "voxels".to_string(),
            z: 10,
            cells: vec![SceneCell {
                x: 0,
                y: 0,
                glyph: '.',
                style: TextStyle::default(),
            }],
        });
        let bg_color = [0x50, 0x58, 0x66, 0xff];
        let mut star_only = vec![0; CHAR_WIDTH * CHAR_HEIGHT * 4];
        draw_glyph(&mut star_only, CHAR_WIDTH, CHAR_HEIGHT, 0, 0, '*', bg_color);
        let mut covered = vec![0; CHAR_WIDTH * CHAR_HEIGHT * 4];
        render_scene(&scene, &mut covered, CHAR_WIDTH, CHAR_HEIGHT);

        let leaked_star_pixels = star_only
            .chunks_exact(4)
            .zip(covered.chunks_exact(4))
            .filter(|(star, pixel)| **star == bg_color && **pixel == bg_color)
            .count();

        assert_eq!(leaked_star_pixels, 0);
    }

    #[test]
    fn asset_viewer_selects_numbered_asset() {
        let mut app = AppState::new();
        app.start_asset_viewer();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit3), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::None);
        assert_eq!(app.asset_viewer.selected, 2);
        assert_eq!(app.asset_viewer.selected_asset().name, "corn stalk");
    }

    #[test]
    fn asset_viewer_starts_at_same_zoom_for_each_asset() {
        let mut viewer = AssetViewerState::new();
        let initial_distance = viewer.distance;

        viewer.select(4);
        let jukebox_distance = viewer.distance;
        viewer.select(0);
        let patron_distance = viewer.distance;

        assert_eq!(initial_distance, ASSET_VIEWER_DEFAULT_DISTANCE);
        assert_eq!(jukebox_distance, ASSET_VIEWER_DEFAULT_DISTANCE);
        assert_eq!(patron_distance, ASSET_VIEWER_DEFAULT_DISTANCE);
    }

    #[test]
    fn asset_viewer_zoom_clamps_around_selected_asset() {
        let mut viewer = AssetViewerState::new();
        viewer.update(
            &PlayerInput {
                up: true,
                ..PlayerInput::default()
            },
            10.0,
        );
        let near = viewer.distance;
        viewer.update(
            &PlayerInput {
                down: true,
                ..PlayerInput::default()
            },
            10.0,
        );

        assert!(near >= viewer.selected_asset().radius * 1.15);
        assert!(viewer.distance <= ASSET_VIEWER_MAX_DISTANCE);
    }

    #[test]
    fn asset_viewer_allows_full_pitch_rotation() {
        let mut viewer = AssetViewerState::new();
        let center = viewer.selected_asset().center;
        viewer.rotate_with_mouse(0.0, -2_000.0);
        let camera = viewer.camera();

        assert!(camera.forward().y.abs() > 0.1);
        assert!((center - (camera.position + camera.forward() * viewer.distance)).length() < 0.001);
    }

    #[test]
    fn asset_viewer_keeps_local_right_after_overhead_flip() {
        let mut viewer = AssetViewerState::new();

        viewer.rotate_with_mouse(0.0, -800.0);
        let right_before = viewer.camera().right();
        viewer.rotate_with_mouse(20.0, 0.0);
        let right_after = viewer.camera().right();

        assert!(right_before.dot(right_after) > 0.99);
    }

    #[test]
    fn asset_viewer_scene_draws_selected_asset() {
        let mut app = AppState::new();
        app.start_asset_viewer();

        let scene = app.frame(0.0, true);

        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "voxels" && !layer.cells.is_empty()));
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("ASSET VIEWER")));
    }

    #[test]
    fn asset_viewer_m_returns_to_menu() {
        let mut app = AppState::new();
        app.start_asset_viewer();

        let action = app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyM), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::EnterMenu);
    }

    #[test]
    fn weapon_asset_has_gun_silhouette() {
        let gun = build_weapon_asset();

        assert!(gun.voxel_count() > 700);
        assert_eq!(
            gun.get(VoxelCoord::new(34, 8, 1)),
            Some(VoxelCell::new(VoxelMaterial::Beacon))
        );
        assert_eq!(
            gun.get(VoxelCoord::new(-12, 5, 2)),
            Some(VoxelCell::new(VoxelMaterial::CarbonLife))
        );
        assert_eq!(
            gun.get(VoxelCoord::new(18, 11, 2)),
            Some(VoxelCell::new(VoxelMaterial::ShipHull))
        );
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
    fn zombies_fire_awards_hit_and_kill_points_with_ammo() {
        let mut app = AppState::new();
        app.start_zombies();
        app.zombies.zombies = vec![Zombie::new(Vec3::new(0.5, 0.0, -38.5), 1)];
        app.zombies.zombies[0].health = app.zombies.weapon.damage();
        let ammo_before = app.zombies.ammo_in_mag;

        app.fire_weapon();

        assert_eq!(app.zombies.ammo_in_mag, ammo_before - 1);
        assert_eq!(app.zombies.kills, 1);
        assert_eq!(
            app.zombies.points,
            500 + ZOMBIE_HIT_POINTS + ZOMBIE_KILL_POINTS
        );
        assert_eq!(
            app.zombies.total_points,
            ZOMBIE_HIT_POINTS + ZOMBIE_KILL_POINTS
        );
    }

    #[test]
    fn zombies_rounds_spawn_progressively_more_and_tougher_zombies() {
        let mut zombies = ZombiesState::new();
        zombies.zombies.clear();
        zombies.queued_spawns = 0;
        zombies.round_break_timer = 0.0;

        zombies.update_rounds_and_zombies(
            &build_zombies_map(&zombies),
            zombies_start_camera().position,
            0.1,
        );

        assert_eq!(zombies.round, 2);
        assert_eq!(zombies.queued_spawns, 10);
        assert_eq!(zombies.zombies.len(), 1);
        assert!(zombies.zombies[0].max_health > Zombie::new(Vec3::ZERO, 1).max_health);
    }

    #[test]
    fn zombies_doors_spend_points_and_open_new_area() {
        let mut zombies = ZombiesState::new();
        let mut world = build_zombies_map(&zombies);
        zombies.points = ZOMBIE_DOOR_COST;

        zombies.interact(&mut world, Vec3::new(0.5, WALK_EYE_HEIGHT, -27.5));

        assert!(door_open(&zombies, ZombiesDoorKind::Building));
        assert_eq!(zombies.points, 0);
        assert_eq!(world.get(VoxelCoord::new(0, 4, -27)), None);
    }

    #[test]
    fn zombies_wall_weapon_upgrades_weapon_and_refills_ammo() {
        let mut zombies = ZombiesState::new();
        let mut world = build_zombies_map(&zombies);
        zombies.points = ZOMBIE_WALL_WEAPON_COST;

        zombies.interact(&mut world, Vec3::new(33.0, WALK_EYE_HEIGHT, -18.0));

        assert_eq!(zombies.weapon, ZombiesWeaponKind::WallRifle);
        assert_eq!(zombies.points, 0);
        assert_eq!(
            zombies.ammo_in_mag,
            ZombiesWeaponKind::WallRifle.magazine_size()
        );
        assert!(zombies.ammo_reserve > ZOMBIE_START_AMMO);
    }

    #[test]
    fn zombies_take_three_hits_to_end_game() {
        let mut zombies = ZombiesState::new();
        zombies.zombies = vec![Zombie::new(Vec3::new(0.7, 0.0, -66.5), 1)];
        let world = build_zombies_map(&zombies);
        let player = zombies_start_camera().position;

        for _ in 0..3 {
            zombies.zombies[0].attack_cooldown = 0.0;
            zombies.update_rounds_and_zombies(&world, player, 0.1);
        }

        assert_eq!(zombies.health_left(), 0);
        assert!(zombies.game_over);
    }

    #[test]
    fn zombies_spawn_candidates_stay_on_walkable_ground() {
        let zombies = ZombiesState::new();
        let world = build_zombies_map(&zombies);
        let spawns = zombie_spawn_candidates(&zombies, &world);

        assert!(!spawns.is_empty());
        assert!(spawns
            .iter()
            .all(|position| zombie_spawn_position_is_valid(&world, *position)));
    }

    #[test]
    fn zombies_navigation_routes_around_walls() {
        let mut world = VoxelWorld::new();
        fill_cuboid(
            &mut world,
            VoxelCoord::new(-8, 0, -8),
            VoxelCoord::new(8, 0, 8),
            VoxelMaterial::Regolith,
        );
        fill_cuboid(
            &mut world,
            VoxelCoord::new(0, 1, -8),
            VoxelCoord::new(0, 4, -1),
            VoxelMaterial::Basalt,
        );
        fill_cuboid(
            &mut world,
            VoxelCoord::new(0, 1, 1),
            VoxelCoord::new(0, 4, 8),
            VoxelMaterial::Basalt,
        );

        let player = Vec3::new(4.5, ZOMBIE_EYE_HEIGHT, 4.5);
        let nav = ZombieNavigationField::build(&world, player, zombie_walk_profile())
            .expect("navigation field should exist");
        let next = nav
            .next_step(Vec3::new(-1.5, ZOMBIE_EYE_HEIGHT, 4.5))
            .expect("zombie should have a route");

        assert!(next.x < 0.0);
        assert!(can_walk_to_on_ground(&world, next, zombie_walk_profile()));
    }

    #[test]
    fn zombies_render_smaller_body() {
        assert!(!zombie_body_contains_voxel(
            Vec3::ZERO,
            VoxelCoord::new(3, 2, 0)
        ));
        assert!(!zombie_body_contains_voxel(
            Vec3::ZERO,
            VoxelCoord::new(0, 7, 0)
        ));
    }

    #[test]
    fn zombies_sprint_drops_to_walk_speed_when_exhausted() {
        let mut zombies = ZombiesState::new();
        zombies.sprint = 0.0;
        zombies.sprint_locked = true;
        let world = VoxelWorld::new();
        let mut camera = zombies_start_camera();
        let start = camera.position;
        let input = PlayerInput {
            forward: true,
            boost: true,
            ..Default::default()
        };

        zombies.update_player(&mut camera, &input, &world, 1.0);

        assert!(zombies.sprint_locked);
        assert!(zombies.sprint > 0.0);
        assert!(zombies.sprint < 1.0);
        let traveled = (camera.position - start).length();
        assert!(traveled > ZOMBIE_WALK_SPEED * 0.95);
        assert!(traveled < ZOMBIE_WALK_SPEED * 1.05);
    }

    #[test]
    fn zombies_sprint_unlocks_only_after_full_recharge() {
        let mut zombies = ZombiesState::new();
        zombies.sprint = 0.0;
        zombies.sprint_locked = true;
        let world = VoxelWorld::new();
        let mut camera = zombies_start_camera();
        let input = PlayerInput {
            forward: true,
            boost: true,
            ..Default::default()
        };

        zombies.update_player(&mut camera, &input, &world, 4.0);
        assert!(zombies.sprint_locked);
        assert!(zombies.sprint < 1.0);

        zombies.update_player(&mut camera, &input, &world, 1.0);

        assert!(!zombies.sprint_locked);
        assert_eq!(zombies.sprint, 1.0);
    }

    #[test]
    fn zombies_scene_draws_bottom_right_weapon_and_hud() {
        let mut app = AppState::new();
        app.start_zombies();

        let scene = app.frame(0.0, true);
        let weapon = scene
            .layers
            .iter()
            .find(|layer| layer.name == "weapon")
            .expect("zombies scene should draw weapon");

        assert!(weapon
            .cells
            .iter()
            .any(|cell| cell.x > VIEWPORT.width as i32 - 35));
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("ROUND") && overlay.style.bg.is_some()));
    }

    #[test]
    fn zombies_are_inserted_as_voxel_objects() {
        let mut app = AppState::new();
        app.start_zombies();
        app.zombies.zombies = vec![Zombie::new(Vec3::new(0.5, 0.0, -38.5), 1)];
        let world = zombies_world_with_zombies(&app.zombies_map, &app.zombies);

        assert_eq!(
            world.get(VoxelCoord::new(0, 3, -39)),
            Some(VoxelCell::new(VoxelMaterial::Zombie))
        );
        assert!(app
            .zombies
            .zombie_index_for_voxel(VoxelCoord::new(0, 3, -39))
            .is_some());
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
