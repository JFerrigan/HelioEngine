use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use font8x8::{UnicodeFonts, BASIC_FONTS};
use heliobound_audio::{EcholocationInterference, GameAudio, SoundEffect};
use heliobound_core::{
    AssetCatalog, Camera, CityConfig, CityGenerator, DoomMapConfig, DoomMapGenerator, MapCatalog,
    PlanetConfig, ProceduralPlanet, Ray, Vec3, VoxelCell, VoxelCoord, VoxelMaterial, VoxelWorld,
};
use heliobound_gfx::{
    raycast, GraphicsConfig, Layer, MaterialGlyphMap, Overlay, Scene, SceneBuilder, SceneCell,
    TextStyle, Viewport,
};
use pixels::{PixelsBuilder, SurfaceTexture};
use serde::Deserialize;
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
const DRONE_GATE_FLIGHT_SPEED: f32 = 100.0;
const DRONE_GATE_STRAFE_SPEED: f32 = 42.0;
const DRONE_GATE_ACCELERATION: f32 = 180.0;
const DRONE_GATE_IDLE_DRAG: f32 = 2.4;
const DRONE_GATE_TURN_DRAG: f32 = 0.35;
const DRONE_GATE_BOOST_MULTIPLIER: f32 = 1.75;
const WALK_SPEED: f32 = 15.0;
const BOOST_MULTIPLIER: f32 = 8.0;
const WALK_BOOST_MULTIPLIER: f32 = 2.25;
const WALK_EYE_HEIGHT: f32 = 3.2;
const WALK_COLLISION_RADIUS: f32 = 0.34;
const WALK_JUMP_SPEED: f32 = 10.5;
const WALK_GRAVITY: f32 = 30.0;
const CITY_FIGURE_EYE_HEIGHT: f32 = WALK_EYE_HEIGHT;
const CITY_FIGURE_SPEED: f32 = 4.0;
const CITY_FIGURE_GAZE_DISTANCE: f32 = 70.0;
const CITY_FIGURE_GAZE_DOT: f32 = 0.93;
#[cfg(test)]
const ENEMY_EYE_HEIGHT: f32 = WALK_EYE_HEIGHT;
const CLOWN_SPEED: f32 = 5.5;
const CLOWN_ATTACK_RANGE: f32 = 2.4;
const CLOWN_ATTACK_DAMAGE: i32 = 8;
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
const MAP_VIEWER_ROLL_SPEED: f32 = 1.2;
const MAP_VIEWER_ZOOM_SPEED: f32 = 55.0;
const MAP_VIEWER_MOUSE_SENSITIVITY: f32 = 0.0045;
const MAP_VIEWER_FREE_SPEED_MULTIPLIER: f32 = 2.0;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnemyType {
    Clown,
    Zombie,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EnemyProfile {
    eye_height: f32,
    speed: f32,
    collision_radius: f32,
    attack_range: f32,
    attack_damage: i32,
    attack_cooldown: f32,
    base_health: i32,
    health_per_round: i32,
}

impl EnemyType {
    fn profile(self) -> EnemyProfile {
        match self {
            Self::Clown => EnemyProfile {
                eye_height: WALK_EYE_HEIGHT,
                speed: CLOWN_SPEED,
                collision_radius: WALK_COLLISION_RADIUS,
                attack_range: CLOWN_ATTACK_RANGE,
                attack_damage: CLOWN_ATTACK_DAMAGE,
                attack_cooldown: 1.1,
                base_health: 100,
                health_per_round: 0,
            },
            Self::Zombie => EnemyProfile {
                eye_height: ZOMBIE_EYE_HEIGHT,
                speed: ZOMBIE_WALK_SPEED,
                collision_radius: ZOMBIE_COLLISION_RADIUS,
                attack_range: 2.1,
                attack_damage: 0,
                attack_cooldown: ZOMBIE_ATTACK_COOLDOWN,
                base_health: 90,
                health_per_round: 34,
            },
        }
    }

    #[cfg(test)]
    fn label(self) -> &'static str {
        match self {
            Self::Clown => "clown",
            Self::Zombie => "zombie",
        }
    }
}
const DRONE_GATE_SEED: u64 = 0xD20A_6A7E_2026_0826;
const DRONE_GATE_VIEW_DISTANCE: f32 = 900.0;
const DRONE_GATE_START_BACK: f32 = 42.0;
const DRONE_GATE_FRAME_RADIUS: i32 = 7;
const DRONE_GATE_INNER_RADIUS: f32 = 5.7;
const DRONE_GATE_TUBE_RADIUS: i32 = 1;
const DRONE_GATE_DEPTH: i32 = 2;
const DRONE_GATE_PASS_DISTANCE: f32 = 7.5;
const DRONE_GATE_RING_SEGMENTS: usize = 32;
const DRONE_GATE_COURSE_GATE_COUNT: usize = 14;
const DRONE_GATE_COURSE_WIDTH: f32 = 80.0;
const DRONE_GATE_COURSE_HEIGHT: f32 = 28.0;
const DRONE_GATE_COURSE_SPACING: f32 = 58.0;
const DRONE_GATE_COURSE_BASE_ALTITUDE: f32 = 18.0;
const DRONE_GATE_COURSE_LATERAL_WAVE: f32 = 0.76;
const DRONE_GATE_COURSE_VERTICAL_WAVE: f32 = 0.59;
const DRONE_GATE_COURSE_LATERAL_JITTER: f32 = 18.0;
const DRONE_GATE_COURSE_VERTICAL_JITTER_DOWN: f32 = 6.0;
const DRONE_GATE_COURSE_VERTICAL_JITTER_UP: f32 = 8.0;
const DRONE_GATE_COURSE_YAW_BEND: f32 = 0.42;
const DRONE_GATE_COURSE_PITCH_BEND: f32 = 0.18;
const DRONE_GATE_COURSE_SPACING_JITTER: f32 = 0.12;
const DRONE_GATE_COURSE_LOOKAHEAD: usize = 12;
const DRONE_GATE_COURSE_APPEND_COUNT: usize = 16;
const DRONE_GATE_RENDER_PAST: usize = 4;
const ECHOLOCATION_SEED: u64 = 0xEC40_10CA_7100_0001;
const ECHOLOCATION_PING_SPEED: f32 = 10.0;
const ECHOLOCATION_PING_MAX_RANGE: f32 = 92.0;
const ECHOLOCATION_PING_COOLDOWN_SECONDS: f32 = 1.0;
const ECHO_CHARGED_PULSE_SECONDS: f32 = 1.5;
const ECHO_CHARGED_PULSE_MAX_RANGE: f32 = 160.0;
const ECHO_PURSUER_SPEED: f32 = 3.0;
const ECHO_PURSUER_CONTACT_RADIUS: f32 = 0.72;
const ECHO_PURSUER_STEP_SECONDS: f32 = 0.52;
const ECHO_PURSUER_INVESTIGATE_SECONDS: f32 = 8.0;
const ECHO_PURSUER_FOOTSTEP_HEARING_RANGE: f32 = 12.0;
// Prints need to remain long enough for the player to round a corner and see
// a recently traversed corridor, but are still transient rather than a map.
const ECHO_FOOTPRINT_LIFETIME: f32 = 4.0;
const ECHO_STEP_WAVE_SPEED: f32 = 5.0;
const ECHO_STEP_WAVE_MAX_RADIUS: f32 = 2.2;
const ECHO_FOOTPRINT_SURFACE_Y: f32 = 1.02;
// At normal walking speed this creates an audible footfall about four times a
// second, while boost remains deliberately louder/more frequent.
const ECHO_PLAYER_STEP_DISTANCE: f32 = 2.6;
const ECHO_PUZZLE_SIGNAL_SPEED: f32 = 6.0;
const ECHO_RECEIVER_OUTPUT_SECONDS: f32 = 3.0;
const ECHO_RECEIVER_COORD: VoxelCoord = VoxelCoord::new(-36, 1, 0);
const ECHO_DOOR_X: i32 = -21;
const ECHOLOCATION_WALK_PROFILE: WalkProfile = WalkProfile {
    speed: 10.0,
    collision_radius: 0.45,
    eye_height: WALK_EYE_HEIGHT,
};

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
                WindowEvent::MouseInput { state, button, .. } => {
                    if mouse_captured {
                        app.handle_mouse_button(button, state);
                        play_audio_events(&mut audio, app.drain_audio_events());
                    } else if state == ElementState::Pressed && app.mode != AppMode::Menu {
                        mouse_captured = set_mouse_captured(&window, true);
                        if mouse_captured && app.mode == AppMode::EchoLocation {
                            app.handle_mouse_button(button, state);
                        }
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
                                audio.set_echolocation_interference(
                                    EcholocationInterference::Inactive,
                                );
                            }
                        }
                        KeyboardAction::EnterMenu => {
                            app.enter_menu();
                            audio.leave_ambience();
                            audio.set_echolocation_interference(EcholocationInterference::Inactive);
                            mouse_captured = set_mouse_captured(&window, false);
                        }
                        KeyboardAction::StartScene => {
                            // Restarts and mode changes must not carry an old search bed
                            // into the new simulation before its first redraw.
                            audio.set_echolocation_interference(EcholocationInterference::Inactive);
                            update_mode_audio(&mut audio, app.mode);
                            mouse_captured = set_mouse_captured(&window, false);
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
                    audio.set_echolocation_interference(app.echolocation_audio_state());
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
    MapViewer,
    VoxelSandbox,
    Zombies,
    Liminal,
    DroneGateRunner,
    EchoLocation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardAction {
    None,
    Exit,
    ReleaseMouse,
    EnterMenu,
    StartScene,
}

struct AppState {
    mode: AppMode,
    planet: ProceduralPlanet,
    city: VoxelWorld,
    /// Immutable startup blueprints shared by game sessions and the map viewer.
    map_catalog: MapCatalog,
    doom_map: VoxelWorld,
    bar_scene: VoxelWorld,
    corn_maze: CornMazeState,
    asset_viewer: AssetViewerState,
    map_viewer: Option<MapViewerState>,
    sandbox: VoxelSandboxState,
    zombies_map: VoxelWorld,
    zombies: ZombiesState,
    liminal: LiminalState,
    drone_gate_runner: DroneGateRunnerState,
    echolocation: EchoLocationState,
    weapon_asset: PreviewAsset,
    planet_builder: SceneBuilder,
    city_builder: SceneBuilder,
    map_builder: SceneBuilder,
    camera: Camera,
    input: PlayerInput,
    walk_motion: WalkMotion,
    city_figures: CityFigureState,
    shooter: ShooterState,
    viewmodel_bob: ViewmodelBob,
    audio_events: Vec<SoundEffect>,
    drone_course_nonce: u64,
    randomize_drone_course: bool,
    drone_course_runs: u64,
    tick: u64,
}

impl AppState {
    fn new() -> Self {
        Self::new_with_drone_course_nonce(runtime_seed_nonce(), true)
    }

    fn new_with_drone_course_nonce(drone_course_nonce: u64, randomize_drone_course: bool) -> Self {
        let initial_drone_seed = drone_course_seed(drone_course_nonce, 0);
        let assets = AssetCatalog::discover(
            asset_directory().unwrap_or_else(|| Path::new("assets/voxel-assets").to_owned()),
        );
        let map_catalog = MapCatalog::discover(
            map_directory().unwrap_or_else(|| Path::new("assets/voxel-maps").to_owned()),
            &assets,
        );
        let doom_map = build_doom_map();
        let bar_scene = build_bar_scene();
        let zombies_map = map_catalog
            .get("zombies")
            .map(|map| map.fresh_session().world)
            .unwrap_or_else(|| build_zombies_map(&ZombiesState::new()));
        Self {
            mode: AppMode::Menu,
            planet: build_demo_planet(),
            city: build_demo_city(),
            map_catalog,
            doom_map,
            bar_scene,
            corn_maze: CornMazeState::new(),
            asset_viewer: AssetViewerState::new(),
            map_viewer: None,
            sandbox: VoxelSandboxState::new(),
            zombies_map,
            zombies: ZombiesState::new(),
            liminal: LiminalState::new_seeded(LIMINAL_SEED),
            drone_gate_runner: DroneGateRunnerState::new_seeded(initial_drone_seed),
            echolocation: EchoLocationState::new_seeded(ECHOLOCATION_SEED),
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
            map_builder: SceneBuilder::new(
                GraphicsConfig {
                    viewport: VIEWPORT,
                    max_distance: f32::INFINITY,
                },
                MaterialGlyphMap,
            ),
            camera: planet_start_camera(),
            input: PlayerInput::default(),
            walk_motion: WalkMotion::default(),
            city_figures: CityFigureState::new(),
            shooter: ShooterState::new(),
            viewmodel_bob: ViewmodelBob::default(),
            audio_events: Vec::new(),
            drone_course_nonce,
            randomize_drone_course,
            drone_course_runs: 0,
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
                (AppMode::Menu, PhysicalKey::Code(KeyCode::KeyE)) => {
                    self.start_echolocation();
                    return KeyboardAction::StartScene;
                }
                (AppMode::Menu, PhysicalKey::Code(KeyCode::KeyV)) => {
                    self.start_map_viewer();
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
                (AppMode::MapViewer, PhysicalKey::Code(KeyCode::KeyM)) => {
                    return KeyboardAction::EnterMenu;
                }
                (AppMode::MapViewer, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::ReleaseMouse;
                }
                (AppMode::MapViewer, key) => {
                    let viewer = self
                        .map_viewer
                        .as_mut()
                        .expect("map viewer mode requires viewer state");
                    if let Some(index) = asset_digit_index(key) {
                        viewer.select(index);
                        self.camera = viewer.camera();
                        return KeyboardAction::None;
                    }
                    match key {
                        PhysicalKey::Code(KeyCode::KeyN) | PhysicalKey::Code(KeyCode::Period) => {
                            viewer.select_next();
                            self.camera = viewer.camera();
                            return KeyboardAction::None;
                        }
                        PhysicalKey::Code(KeyCode::KeyP) | PhysicalKey::Code(KeyCode::Comma) => {
                            viewer.select_previous();
                            self.camera = viewer.camera();
                            return KeyboardAction::None;
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            viewer.reset_view();
                            self.camera = viewer.camera();
                            return KeyboardAction::None;
                        }
                        PhysicalKey::Code(KeyCode::KeyO) => {
                            viewer.toggle_view();
                            self.camera = viewer.camera();
                            return KeyboardAction::None;
                        }
                        PhysicalKey::Code(KeyCode::KeyC) => {
                            viewer.toggle_ceilings();
                            return KeyboardAction::None;
                        }
                        _ => {}
                    }
                }
                (AppMode::VoxelSandbox, PhysicalKey::Code(KeyCode::KeyM)) => {
                    return KeyboardAction::EnterMenu;
                }
                (AppMode::VoxelSandbox, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::ReleaseMouse;
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
                (AppMode::EchoLocation, PhysicalKey::Code(KeyCode::KeyR))
                    if self.echolocation.run_status == EchoRunStatus::Dead =>
                {
                    self.start_echolocation();
                    return KeyboardAction::StartScene;
                }
                (AppMode::EchoLocation, PhysicalKey::Code(KeyCode::Tab)) => {
                    self.echolocation.toggle_tuning();
                    return KeyboardAction::None;
                }
                (AppMode::EchoLocation, PhysicalKey::Code(KeyCode::KeyV)) => {
                    self.echolocation.toggle_full_map();
                    return KeyboardAction::None;
                }
                (AppMode::EchoLocation, PhysicalKey::Code(KeyCode::KeyM)) => {
                    return KeyboardAction::EnterMenu;
                }
                (AppMode::EchoLocation, PhysicalKey::Code(KeyCode::Escape)) => {
                    return KeyboardAction::ReleaseMouse;
                }
                (AppMode::EchoLocation, PhysicalKey::Code(key)) => {
                    let action = match key {
                        KeyCode::BracketRight => Some(EchoTuningAction::IncreaseRange),
                        KeyCode::BracketLeft => Some(EchoTuningAction::DecreaseRange),
                        KeyCode::Equal => Some(EchoTuningAction::IncreaseSpeed),
                        KeyCode::Minus => Some(EchoTuningAction::DecreaseSpeed),
                        KeyCode::Period => Some(EchoTuningAction::IncreaseStrength),
                        KeyCode::Comma => Some(EchoTuningAction::DecreaseStrength),
                        KeyCode::KeyR => Some(EchoTuningAction::ResetDefaults),
                        _ => None,
                    };
                    if let Some(action) = action {
                        self.echolocation.apply_tuning(action);
                        return KeyboardAction::None;
                    }
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
        if self.mode == AppMode::MapViewer {
            self.map_viewer = None;
        }
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
        self.walk_motion = WalkMotion::default();
        self.city_figures = CityFigureState::new();
    }

    fn start_shooter(&mut self) {
        self.mode = AppMode::CityShooter;
        self.camera = doom_start_camera();
        self.input = PlayerInput::default();
        self.walk_motion = WalkMotion::default();
        self.shooter = ShooterState::new();
        self.viewmodel_bob = ViewmodelBob::default();
    }

    fn start_corn_maze(&mut self) {
        self.mode = AppMode::CornMaze;
        self.corn_maze = CornMazeState::new();
        self.camera = corn_maze_start_camera(&self.corn_maze);
        self.input = PlayerInput::default();
        self.walk_motion = WalkMotion::default();
    }

    fn start_bar(&mut self) {
        self.mode = AppMode::BarScene;
        self.bar_scene = build_bar_scene();
        self.camera = bar_start_camera();
        self.input = PlayerInput::default();
        self.walk_motion = WalkMotion::default();
    }

    fn start_asset_viewer(&mut self) {
        self.mode = AppMode::AssetViewer;
        self.asset_viewer = AssetViewerState::new();
        self.camera = self.asset_viewer.camera();
        self.input = PlayerInput::default();
    }

    fn start_map_viewer(&mut self) {
        self.mode = AppMode::MapViewer;
        self.map_viewer = Some(MapViewerState::new(build_map_catalog_from(
            &self.map_catalog,
        )));
        self.camera = self
            .map_viewer
            .as_ref()
            .expect("new map viewer has state")
            .camera();
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
        self.zombies_map = self
            .map_catalog
            .get("zombies")
            .map(|map| map.fresh_session().world)
            .unwrap_or_else(|| build_zombies_map(&self.zombies));
        self.camera = self
            .map_catalog
            .get("zombies")
            .map(compiled_start_camera)
            .unwrap_or_else(zombies_start_camera);
        self.input = PlayerInput::default();
        self.walk_motion = WalkMotion::default();
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
        self.drone_course_runs = self.drone_course_runs.wrapping_add(1);
        if self.randomize_drone_course {
            self.drone_course_nonce = mix_seed(self.drone_course_nonce ^ runtime_seed_nonce());
        }
        self.drone_gate_runner = DroneGateRunnerState::new_seeded(drone_course_seed(
            self.drone_course_nonce,
            self.drone_course_runs,
        ));
        self.camera = drone_gate_runner_start_camera(&self.drone_gate_runner);
        self.input = PlayerInput::default();
    }

    fn start_echolocation(&mut self) {
        self.mode = AppMode::EchoLocation;
        self.echolocation = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        self.camera = echolocation_start_camera(&self.echolocation);
        self.input = PlayerInput::default();
        self.walk_motion = WalkMotion::default();
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
                update_jumping_walking_camera(
                    &mut self.camera,
                    &mut self.input,
                    &mut self.walk_motion,
                    &self.city,
                    STANDARD_WALK_PROFILE,
                    dt,
                );
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
                update_jumping_walking_camera(
                    &mut self.camera,
                    &mut self.input,
                    &mut self.walk_motion,
                    &self.doom_map,
                    STANDARD_WALK_PROFILE,
                    dt,
                );
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
                update_jumping_walking_camera(
                    &mut self.camera,
                    &mut self.input,
                    &mut self.walk_motion,
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
            AppMode::MapViewer => {
                let viewer = self
                    .map_viewer
                    .as_mut()
                    .expect("map viewer mode requires viewer state");
                viewer.update(&self.input, dt);
                self.camera = viewer.camera();
                let mut scene =
                    self.map_builder
                        .build(viewer.render_world(), &self.camera, self.tick);
                render_map_viewer_scene(&mut scene, viewer, mouse_captured);
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
                self.zombies.update_player(
                    &mut self.camera,
                    &mut self.input,
                    &mut self.walk_motion,
                    &self.zombies_map,
                    dt,
                );
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
                if let Some(effect) =
                    self.drone_gate_runner
                        .update_camera(&mut self.camera, &self.input, dt)
                {
                    self.audio_events.push(effect);
                }
                let world = self.drone_gate_runner.render_world();
                let mut scene = self.city_builder.build(&world, &self.camera, self.tick);
                render_drone_gate_runner_scene(&mut scene, &self.drone_gate_runner, mouse_captured);
                scene
            }
            AppMode::EchoLocation => {
                if self.echolocation.run_status == EchoRunStatus::Active {
                    let before_move = self.camera.position;
                    update_jumping_walking_camera(
                        &mut self.camera,
                        &mut self.input,
                        &mut self.walk_motion,
                        &self.echolocation.world,
                        ECHOLOCATION_WALK_PROFILE,
                        dt,
                    );
                    self.audio_events
                        .extend(self.echolocation.update_player_footsteps(
                            horizontal_distance(before_move, self.camera.position),
                            self.camera.position,
                        ));
                }
                let echo_update = self.echolocation.update_with_pursuer_from_listener(
                    dt,
                    self.camera.position,
                    self.camera.right(),
                );
                self.audio_events.extend(echo_update.sound_events);
                if let Some(position) = echo_update.corrected_player_position {
                    self.camera.position = position;
                }
                let mut scene = self.city_builder.build_with_visibility(
                    &self.echolocation.world,
                    &self.camera,
                    self.tick,
                    |hit| self.echolocation.face_is_revealed(hit.coord, hit.normal),
                );
                render_echolocation_scene(
                    &mut scene,
                    &self.echolocation,
                    &self.camera,
                    mouse_captured,
                );
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
            | AppMode::Liminal
            | AppMode::EchoLocation => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Clamped)
            }
            AppMode::AssetViewer => self.asset_viewer.rotate_with_mouse(delta_x, delta_y),
            AppMode::MapViewer => {
                let viewer = self
                    .map_viewer
                    .as_mut()
                    .expect("map viewer mode requires viewer state");
                viewer.rotate_with_mouse(delta_x, delta_y);
                self.camera = viewer.camera();
            }
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match (self.mode, button, state) {
            (AppMode::CityShooter, MouseButton::Left, ElementState::Pressed) => self.fire_weapon(),
            (AppMode::Zombies, MouseButton::Left, ElementState::Pressed) => self.fire_weapon(),
            (AppMode::EchoLocation, MouseButton::Left, ElementState::Pressed) => {
                self.echolocation.begin_pulse_charge()
            }
            (AppMode::EchoLocation, MouseButton::Left, ElementState::Released) => {
                if self.echolocation.release_pulse_charge(self.camera.position) {
                    self.audio_events.push(SoundEffect::EchoPing);
                }
            }
            (AppMode::VoxelSandbox, MouseButton::Left, ElementState::Pressed) => {
                self.sandbox.remove_block(&self.camera)
            }
            (AppMode::VoxelSandbox, MouseButton::Right, ElementState::Pressed) => {
                self.sandbox.place_block(&self.camera)
            }
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
        } else if self.mode == AppMode::EchoLocation {
            if self.echolocation.emit_ping(self.camera.position) {
                self.audio_events.push(SoundEffect::EchoPing);
            }
        }
    }

    fn drain_audio_events(&mut self) -> Vec<SoundEffect> {
        self.audio_events.drain(..).collect()
    }

    fn echolocation_audio_state(&self) -> EcholocationInterference {
        if self.mode != AppMode::EchoLocation
            || self.echolocation.run_status != EchoRunStatus::Active
        {
            return EcholocationInterference::Inactive;
        }
        let effect = echo_search_effect(&self.echolocation, self.camera.position);
        if effect.intensity <= 0.0 {
            EcholocationInterference::Inactive
        } else {
            let direction = horizontal(self.echolocation.pursuer.position - self.camera.position);
            EcholocationInterference::Active {
                intensity: effect.intensity,
                pursuer_pan: direction
                    .normalized()
                    .dot(self.camera.right())
                    .clamp(-1.0, 1.0),
                corruption_level: effect.corruption_level,
            }
        }
    }
}

fn update_mode_audio(audio: &mut GameAudio, mode: AppMode) {
    match mode {
        AppMode::CityWalk => audio.enter_city_mode(),
        AppMode::CornMaze => audio.enter_corn_maze_mode(),
        AppMode::BarScene | AppMode::VoxelSandbox => audio.enter_bar_mode(),
        AppMode::CityShooter | AppMode::Zombies => audio.enter_doom_mode(),
        AppMode::PlanetFlight | AppMode::Liminal => audio.enter_doom_mode(),
        AppMode::DroneGateRunner => audio.enter_drone_mode(),
        AppMode::EchoLocation => audio.enter_doom_mode(),
        AppMode::Menu | AppMode::AssetViewer | AppMode::MapViewer => audio.leave_ambience(),
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
    Camera::new(Vec3::new(
        0.0,
        PLANET_START_Y_OFFSET,
        -(envelope_radius + PLANET_START_ALTITUDE),
    ))
    .looking_at(0.0, std::f32::consts::FRAC_PI_2)
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
    let first_gate = runner
        .course
        .gates
        .first()
        .map(|target| target.position)
        .unwrap_or(Vec3::ZERO);
    look_at(runner.start_position, first_gate)
        .with_fov_y(72.0_f32.to_radians())
        .with_max_distance(DRONE_GATE_VIEW_DISTANCE)
}

fn echolocation_start_camera(echo: &EchoLocationState) -> Camera {
    look_at(
        echo.start_position,
        echo.start_position + Vec3::new(0.0, 0.0, 8.0),
    )
    .with_fov_y(66.0_f32.to_radians())
    .with_max_distance(110.0)
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
    pan_forward: bool,
    pan_backward: bool,
    pan_left: bool,
    pan_right: bool,
    boost: bool,
    jump_requested: bool,
}

/// State shared by every grounded first-person mode that supports jumping.
/// Keeping it separate from input makes a press a single jump rather than a
/// continuous hover while Space is held.
#[derive(Clone, Copy, Debug, Default)]
struct WalkMotion {
    vertical_velocity: f32,
    airborne: bool,
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
    course: DroneCourseGenerationConfig,
    flight: DroneFlightModel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DroneCourseGenerationConfig {
    gate_count: usize,
    lateral_amplitude: f32,
    vertical_amplitude: f32,
    base_altitude: f32,
    lateral_wave_frequency: f32,
    vertical_wave_frequency: f32,
    lateral_jitter: f32,
    vertical_jitter_down: f32,
    vertical_jitter_up: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DroneFlightModel {
    max_speed: f32,
    strafe_speed: f32,
    acceleration: f32,
    idle_drag: f32,
    turn_drag: f32,
    boost_multiplier: f32,
}

impl Default for DroneGateRunnerConfig {
    fn default() -> Self {
        Self {
            gate_radius: DRONE_GATE_FRAME_RADIUS,
            inner_radius: DRONE_GATE_INNER_RADIUS,
            tube_radius: DRONE_GATE_TUBE_RADIUS,
            pass_distance: DRONE_GATE_PASS_DISTANCE,
            spacing: DRONE_GATE_COURSE_SPACING,
            course: DroneCourseGenerationConfig {
                gate_count: DRONE_GATE_COURSE_GATE_COUNT,
                lateral_amplitude: DRONE_GATE_COURSE_WIDTH,
                vertical_amplitude: DRONE_GATE_COURSE_HEIGHT,
                base_altitude: DRONE_GATE_COURSE_BASE_ALTITUDE,
                lateral_wave_frequency: DRONE_GATE_COURSE_LATERAL_WAVE,
                vertical_wave_frequency: DRONE_GATE_COURSE_VERTICAL_WAVE,
                lateral_jitter: DRONE_GATE_COURSE_LATERAL_JITTER,
                vertical_jitter_down: DRONE_GATE_COURSE_VERTICAL_JITTER_DOWN,
                vertical_jitter_up: DRONE_GATE_COURSE_VERTICAL_JITTER_UP,
            },
            flight: DroneFlightModel {
                max_speed: DRONE_GATE_FLIGHT_SPEED,
                strafe_speed: DRONE_GATE_STRAFE_SPEED,
                acceleration: DRONE_GATE_ACCELERATION,
                idle_drag: DRONE_GATE_IDLE_DRAG,
                turn_drag: DRONE_GATE_TURN_DRAG,
                boost_multiplier: DRONE_GATE_BOOST_MULTIPLIER,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DroneGateTarget {
    position: Vec3,
    normal: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
struct DroneGateCourse {
    seed: u64,
    name: &'static str,
    gates: Vec<DroneGateTarget>,
}

#[derive(Clone, Copy, Debug)]
struct DroneGateCourseCursor {
    rng: LiminalRng,
    position: Vec3,
    direction: Vec3,
    next_index: usize,
    min_altitude: f32,
}

#[derive(Clone, Debug)]
struct DroneGateRunnerState {
    config: DroneGateRunnerConfig,
    course: DroneGateCourse,
    course_cursor: DroneGateCourseCursor,
    active_gate: usize,
    passed_gates: u64,
    start_position: Vec3,
    previous_position: Vec3,
    velocity: Vec3,
    best_streak: u64,
    elapsed: f32,
}

impl DroneGateRunnerState {
    fn new_seeded(seed: u64) -> Self {
        let config = DroneGateRunnerConfig::default();
        let (course, course_cursor) = generate_drone_gate_course_with_cursor(seed, config);
        let first_gate = course
            .gates
            .first()
            .map(|target| target.position)
            .unwrap_or(Vec3::ZERO);
        let second_gate = course
            .gates
            .get(1)
            .map(|target| target.position)
            .unwrap_or(first_gate + Vec3::new(0.0, 0.0, config.spacing));
        let direction = horizontal(second_gate - first_gate);
        let start_position = first_gate - direction * DRONE_GATE_START_BACK;
        let start_position = Vec3::new(start_position.x, first_gate.y, start_position.z);
        Self {
            config,
            course,
            course_cursor,
            active_gate: 0,
            passed_gates: 0,
            start_position,
            previous_position: start_position,
            velocity: Vec3::ZERO,
            best_streak: 0,
            elapsed: 0.0,
        }
    }

    fn update_camera(
        &mut self,
        camera: &mut Camera,
        input: &PlayerInput,
        dt: f32,
    ) -> Option<SoundEffect> {
        apply_flight_roll(camera, input, dt);
        self.update_velocity(camera, input, dt);
        camera.position = camera.position + self.velocity * dt;
        self.update(camera.position, dt)
    }

    fn update_velocity(&mut self, camera: &Camera, input: &PlayerInput, dt: f32) {
        let mut desired = Vec3::ZERO;

        if input.forward {
            desired = desired + camera.forward();
        }
        if input.backward {
            desired = desired - camera.forward();
        }
        if input.right {
            desired = desired + camera.right();
        }
        if input.left {
            desired = desired - camera.right();
        }
        if input.up {
            desired = desired + camera.up();
        }
        if input.down {
            desired = desired - camera.up();
        }

        let flight = self.config.flight;
        if desired.length() > f32::EPSILON {
            let has_primary_thrust = input.forward || input.backward;
            let target_speed = if has_primary_thrust {
                flight.max_speed
            } else {
                flight.strafe_speed
            };
            let target_speed = if input.boost {
                target_speed * flight.boost_multiplier
            } else {
                target_speed
            };
            let target_velocity = desired.normalized() * target_speed;
            self.velocity = approach_vec3(self.velocity, target_velocity, flight.acceleration * dt);
            self.velocity = self.velocity * (1.0 - flight.turn_drag * dt).clamp(0.0, 1.0);
        } else {
            self.velocity = self.velocity * (1.0 - flight.idle_drag * dt).clamp(0.0, 1.0);
            if self.velocity.length() < 0.05 {
                self.velocity = Vec3::ZERO;
            }
        }
    }

    fn update(&mut self, player_position: Vec3, dt: f32) -> Option<SoundEffect> {
        self.elapsed += dt;
        if self.course.gates.is_empty() {
            self.previous_position = player_position;
            return None;
        }

        if self.crossed_active_gate(self.previous_position, player_position) {
            self.advance_gate();
            self.previous_position = player_position;
            return Some(SoundEffect::GateSuccess);
        }
        self.previous_position = player_position;
        None
    }

    fn crossed_active_gate(&self, from: Vec3, to: Vec3) -> bool {
        let gate = self.course.gates[self.active_gate];
        let previous_plane = (from - gate.position).dot(gate.normal);
        let current_plane = (to - gate.position).dot(gate.normal);
        let plane_position = to - gate.normal * current_plane;
        let radial = (plane_position - gate.position).length();
        if radial > self.config.inner_radius {
            return false;
        }

        previous_plane.abs().min(current_plane.abs()) <= self.config.pass_distance
            || previous_plane.signum() != current_plane.signum()
    }

    fn advance_gate(&mut self) {
        self.passed_gates += 1;
        self.best_streak = self.best_streak.max(self.passed_gates);
        self.active_gate += 1;
        self.ensure_gate_lookahead();
    }

    fn ensure_gate_lookahead(&mut self) {
        while self.course.gates.len().saturating_sub(self.active_gate + 1)
            < DRONE_GATE_COURSE_LOOKAHEAD
        {
            self.append_gates(DRONE_GATE_COURSE_APPEND_COUNT);
        }
    }

    fn append_gates(&mut self, count: usize) {
        let mut positions: Vec<Vec3> = self
            .course
            .gates
            .iter()
            .map(|target| target.position)
            .collect();
        positions.extend((0..count).map(|_| self.course_cursor.next_position(self.config)));
        self.course.gates = drone_gate_targets_from_positions(&positions);
    }

    fn active_gate_position(&self) -> Option<Vec3> {
        self.course
            .gates
            .get(self.active_gate)
            .map(|target| target.position)
    }

    fn render_world(&self) -> VoxelWorld {
        build_drone_gate_runner_world(self)
    }

    fn visible_gate_range(&self) -> (usize, usize) {
        let start = self.active_gate.saturating_sub(DRONE_GATE_RENDER_PAST);
        let end = (self.active_gate + DRONE_GATE_COURSE_LOOKAHEAD + 1).min(self.course.gates.len());
        (start, end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoLocationConfig {
    ping_speed: f32,
    max_range: f32,
    initial_energy: f32,
    minimum_visible_energy: f32,
    echo_strength: f32,
    distance_attenuation: f32,
    max_active_waves: usize,
    reveal_seconds: f32,
}

impl Default for EchoLocationConfig {
    fn default() -> Self {
        Self {
            ping_speed: ECHOLOCATION_PING_SPEED,
            max_range: ECHOLOCATION_PING_MAX_RANGE,
            initial_energy: 1.0,
            minimum_visible_energy: 0.08,
            echo_strength: 0.0,
            distance_attenuation: 0.055,
            max_active_waves: 24,
            reveal_seconds: 1.2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EchoImpact {
    solid_voxel: VoxelCoord,
    cell: VoxelCell,
    source_air_cell: VoxelCoord,
    arrival_distance_milli: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct EchoWave {
    age: f32,
    energy: f32,
    bounce_depth: u8,
    original_emission_position: Vec3,
    heard_by_pursuer: bool,
    impacts: Vec<EchoImpact>,
    next_impact: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EchoPursuerMode {
    Wander,
    Investigate {
        last_heard_position: Vec3,
        remaining_seconds: f32,
    },
}

impl EchoWave {
    fn radius(&self, config: EchoLocationConfig) -> f32 {
        self.age * config.ping_speed
    }

    fn energy_at(&self, distance: f32, config: EchoLocationConfig) -> f32 {
        self.energy / (1.0 + config.distance_attenuation * distance.max(0.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EchoTuningAction {
    IncreaseRange,
    DecreaseRange,
    IncreaseSpeed,
    DecreaseSpeed,
    IncreaseStrength,
    DecreaseStrength,
    ResetDefaults,
}

#[derive(Clone, Copy, Debug)]
struct EchoReveal {
    cell: VoxelCell,
    strength: f32,
    remaining_seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EchoRunStatus {
    Active,
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoFootprint {
    position: Vec3,
    remaining_seconds: f32,
    left: bool,
    travel_direction: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
struct EchoStepWave {
    origin: Vec3,
    age: f32,
    impacts: Vec<EchoImpact>,
    next_impact: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoPursuer {
    position: Vec3,
    step_timer: f32,
    travel_direction: Vec3,
    mode: EchoPursuerMode,
    target_position: Option<Vec3>,
    wander_idle_remaining: f32,
    selection_rng: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoReceiver {
    coord: VoxelCoord,
    output_seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoPipePoint {
    position: Vec3,
    distance: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct EchoPuzzleDoor {
    voxels: Vec<VoxelCoord>,
    normal: Vec3,
    starting_side_anchor: Vec3,
    far_side_anchor: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoEmissionInterval {
    start: f32,
    end: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EchoDoorTransition {
    Opened,
    Closed,
}

#[derive(Clone, Debug, PartialEq)]
struct EchoPuzzle {
    receiver: EchoReceiver,
    pipe: Vec<EchoPipePoint>,
    door: EchoPuzzleDoor,
    time: f32,
    emissions: Vec<EchoEmissionInterval>,
    door_open: bool,
    transitions: Vec<EchoDoorTransition>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct EchoFrameUpdate {
    sound_events: Vec<SoundEffect>,
    corrected_player_position: Option<Vec3>,
}

impl EchoPuzzle {
    fn new() -> Self {
        let pipe = (ECHO_RECEIVER_COORD.x..=ECHO_DOOR_X)
            .map(|x| EchoPipePoint {
                position: Vec3::new(x as f32 + 0.5, ECHO_FOOTPRINT_SURFACE_Y + 0.03, 0.5),
                distance: (x - ECHO_RECEIVER_COORD.x) as f32,
            })
            .collect();
        let mut door_voxels = Vec::new();
        for y in 1..=4 {
            for z in -1..=1 {
                door_voxels.push(VoxelCoord::new(ECHO_DOOR_X, y, z));
            }
        }
        Self {
            receiver: EchoReceiver {
                coord: ECHO_RECEIVER_COORD,
                output_seconds: ECHO_RECEIVER_OUTPUT_SECONDS,
            },
            pipe,
            door: EchoPuzzleDoor {
                voxels: door_voxels,
                normal: Vec3::new(1.0, 0.0, 0.0),
                starting_side_anchor: Vec3::new(
                    ECHO_DOOR_X as f32 - 0.5,
                    ECHOLOCATION_WALK_PROFILE.eye_height,
                    0.5,
                ),
                far_side_anchor: Vec3::new(
                    ECHO_DOOR_X as f32 + 1.5,
                    ECHOLOCATION_WALK_PROFILE.eye_height,
                    0.5,
                ),
            },
            time: 0.0,
            emissions: Vec::new(),
            door_open: false,
            transitions: Vec::new(),
        }
    }

    fn endpoint_distance(&self) -> f32 {
        self.pipe.last().map(|point| point.distance).unwrap_or(0.0)
    }

    fn signal_active_at(&self, distance: f32, time: f32) -> bool {
        let source_time = time - distance / ECHO_PUZZLE_SIGNAL_SPEED;
        self.emissions
            .iter()
            .any(|interval| source_time >= interval.start && source_time < interval.end)
    }

    fn receiver_active(&self) -> bool {
        self.signal_active_at(0.0, self.time)
    }

    fn door_powered(&self) -> bool {
        self.signal_active_at(self.endpoint_distance(), self.time)
    }

    fn record_activation(&mut self, time: f32) {
        let end = time + self.receiver.output_seconds;
        if let Some(current) = self.emissions.last_mut() {
            if time <= current.end {
                current.end = current.end.max(end);
                return;
            }
        }
        self.emissions
            .push(EchoEmissionInterval { start: time, end });
    }
}

#[derive(Clone, Debug)]
struct EchoLocationState {
    seed: u64,
    config: EchoLocationConfig,
    world: VoxelWorld,
    start_position: Vec3,
    waves: Vec<EchoWave>,
    revealed: HashMap<VoxelCoord, EchoReveal>,
    ping_cooldown_remaining: f32,
    pulse_charge_seconds: Option<f32>,
    player_step_distance: f32,
    tuning_open: bool,
    show_full_map: bool,
    pursuer: EchoPursuer,
    footprints: Vec<EchoFootprint>,
    step_waves: Vec<EchoStepWave>,
    run_status: EchoRunStatus,
    static_burst_timer: f32,
    static_burst_counter: u32,
    puzzle: EchoPuzzle,
}

impl EchoLocationState {
    fn new_seeded(seed: u64) -> Self {
        let (world, start_position, puzzle) = build_echolocation_map(seed);
        // Seed the pursuer from the far side of the bulkhead. It remains
        // dormant behind the closed puzzle door until the player opens it,
        // rather than spawning in the introductory room.
        let pursuer_position = echo_pursuer_spawn_position(&world, puzzle.door.far_side_anchor)
            .expect("echolocation map must contain a walkable far-side pursuer spawn");
        Self {
            seed,
            config: EchoLocationConfig::default(),
            world,
            start_position,
            waves: Vec::new(),
            revealed: HashMap::new(),
            ping_cooldown_remaining: 0.0,
            pulse_charge_seconds: None,
            player_step_distance: 0.0,
            tuning_open: false,
            show_full_map: false,
            pursuer: EchoPursuer {
                position: pursuer_position,
                step_timer: 0.0,
                travel_direction: Vec3::new(0.0, 0.0, 1.0),
                mode: EchoPursuerMode::Wander,
                target_position: None,
                wander_idle_remaining: 0.0,
                selection_rng: seed ^ 0xA5A5_7D31_9E37_79B9,
            },
            footprints: Vec::new(),
            step_waves: Vec::new(),
            run_status: EchoRunStatus::Active,
            static_burst_timer: 0.0,
            static_burst_counter: 0,
            puzzle,
        }
    }

    fn emit_ping(&mut self, origin: Vec3) -> bool {
        self.emit_ping_with_range(origin, self.config.max_range)
    }

    fn emit_ping_with_range(&mut self, origin: Vec3, range: f32) -> bool {
        if self.run_status != EchoRunStatus::Active || self.ping_cooldown_remaining > 0.0 {
            return false;
        }
        if self.waves.len() >= self.config.max_active_waves {
            let drop_index = self
                .waves
                .iter()
                .position(|wave| wave.bounce_depth > 0)
                .unwrap_or(0);
            self.waves.remove(drop_index);
        }
        let source = voxel_coord_at(origin);
        self.waves.push(build_echo_wave(
            &self.world,
            source,
            self.config.initial_energy,
            0,
            range,
            origin,
        ));
        self.ping_cooldown_remaining = ECHOLOCATION_PING_COOLDOWN_SECONDS;
        true
    }

    fn begin_pulse_charge(&mut self) {
        if self.run_status == EchoRunStatus::Active && self.ping_cooldown_remaining <= 0.0 {
            self.pulse_charge_seconds.get_or_insert(0.0);
        }
    }

    fn release_pulse_charge(&mut self, origin: Vec3) -> bool {
        let charge = self.pulse_charge_seconds.take().unwrap_or(0.0);
        let amount = (charge / ECHO_CHARGED_PULSE_SECONDS).clamp(0.0, 1.0);
        let range =
            self.config.max_range + (ECHO_CHARGED_PULSE_MAX_RANGE - self.config.max_range) * amount;
        if !self.emit_ping_with_range(origin, range) {
            return false;
        }
        true
    }

    fn update_player_footsteps(
        &mut self,
        horizontal_distance: f32,
        player_position: Vec3,
    ) -> Vec<SoundEffect> {
        if self.run_status != EchoRunStatus::Active || horizontal_distance <= f32::EPSILON {
            return Vec::new();
        }
        self.player_step_distance += horizontal_distance;
        let mut effects = Vec::new();
        while self.player_step_distance >= ECHO_PLAYER_STEP_DISTANCE {
            self.player_step_distance -= ECHO_PLAYER_STEP_DISTANCE;
            let foot_position = Vec3::new(
                player_position.x,
                ECHO_FOOTPRINT_SURFACE_Y,
                player_position.z,
            );
            self.step_waves.push(EchoStepWave {
                origin: foot_position,
                age: 0.0,
                impacts: build_echo_wave(
                    &self.world,
                    echo_pursuer_foot_source(foot_position),
                    1.0,
                    0,
                    ECHO_STEP_WAVE_MAX_RADIUS,
                    foot_position,
                )
                .impacts,
                next_impact: 0,
            });
            self.notify_pursuer_of_footstep(foot_position);
            effects.push(SoundEffect::PlayerFootstep);
        }
        effects
    }

    /// Retained for deterministic echo-wave tests; gameplay supplies the live player position.
    #[cfg(test)]
    fn update(&mut self, dt: f32) {
        let _ = self.update_with_pursuer(dt, self.start_position);
    }

    #[cfg(test)]
    fn update_with_pursuer(&mut self, dt: f32, player_position: Vec3) -> Vec<SoundEffect> {
        self.update_with_pursuer_from_listener(dt, player_position, Vec3::new(1.0, 0.0, 0.0))
            .sound_events
    }

    fn update_with_pursuer_from_listener(
        &mut self,
        dt: f32,
        player_position: Vec3,
        listener_right: Vec3,
    ) -> EchoFrameUpdate {
        let dt = dt.max(0.0);
        if self.run_status != EchoRunStatus::Active {
            return EchoFrameUpdate::default();
        }
        let mut effects = Vec::new();
        if let Some(charge) = &mut self.pulse_charge_seconds {
            *charge = (*charge + dt).min(ECHO_CHARGED_PULSE_SECONDS);
        }
        self.update_pursuer(dt, player_position, listener_right, &mut effects);
        if self.run_status == EchoRunStatus::Dead {
            return EchoFrameUpdate {
                sound_events: effects,
                corrected_player_position: None,
            };
        }
        self.update_static_bursts(dt, player_position, listener_right, &mut effects);
        self.ping_cooldown_remaining = (self.ping_cooldown_remaining - dt).max(0.0);
        let config = self.config;
        let mut secondary_sources = Vec::new();
        let mut reveals = Vec::new();
        let mut heard_positions = Vec::new();
        let mut receiver_hits = Vec::new();
        let frame_start = self.puzzle.time;
        for reveal in self.revealed.values_mut() {
            reveal.remaining_seconds -= dt;
        }
        self.revealed
            .retain(|_, reveal| reveal.remaining_seconds > 0.0);
        for wave in &mut self.waves {
            let previous_age = wave.age;
            let previous_radius = wave.radius(config);
            wave.age += dt;
            let radius_milli = (wave.radius(config).max(0.0) * 1000.0) as u32;
            let pursuer_distance =
                horizontal_distance(wave.original_emission_position, self.pursuer.position);
            if !wave.heard_by_pursuer
                && previous_radius <= pursuer_distance
                && wave.radius(config) >= pursuer_distance
            {
                wave.heard_by_pursuer = true;
                heard_positions.push(wave.original_emission_position);
            }
            let can_reflect = wave.bounce_depth < echo_bounce_limit(config.echo_strength);
            while let Some(impact) = wave.impacts.get(wave.next_impact).copied() {
                if impact.arrival_distance_milli > radius_milli {
                    break;
                }
                wave.next_impact += 1;
                let distance = impact.arrival_distance_milli as f32 / 1000.0;
                let impact_energy = wave.energy_at(distance, config);
                reveals.push((
                    impact.solid_voxel,
                    impact.cell,
                    impact_energy,
                    impact.source_air_cell,
                ));
                if impact.solid_voxel == self.puzzle.receiver.coord {
                    receiver_hits.push(
                        frame_start + (distance / config.ping_speed - previous_age).clamp(0.0, dt),
                    );
                }
                if can_reflect {
                    let reflected_energy =
                        impact_energy * echo_reflection_gain(config.echo_strength);
                    if reflected_energy >= config.minimum_visible_energy {
                        secondary_sources.push((
                            impact.source_air_cell,
                            reflected_energy,
                            wave.bounce_depth + 1,
                            wave.original_emission_position,
                        ));
                    }
                }
            }
        }
        for position in heard_positions {
            self.notify_pursuer_of_noise(position);
        }
        // Footsteps use the same surface-return path as the player's pulse, but
        // their range is capped tightly and they never create reflected waves.
        for wave in &mut self.step_waves {
            let previous_age = (wave.age - dt).max(0.0);
            let radius_milli = (wave.age * ECHO_STEP_WAVE_SPEED * 1000.0) as u32;
            while let Some(impact) = wave.impacts.get(wave.next_impact).copied() {
                if impact.arrival_distance_milli > radius_milli {
                    break;
                }
                wave.next_impact += 1;
                reveals.push((impact.solid_voxel, impact.cell, 1.0, impact.source_air_cell));
                if impact.solid_voxel == self.puzzle.receiver.coord {
                    let distance = impact.arrival_distance_milli as f32 / 1000.0;
                    receiver_hits.push(
                        frame_start
                            + (distance / ECHO_STEP_WAVE_SPEED - previous_age).clamp(0.0, dt),
                    );
                }
            }
        }
        for (coord, cell, strength, source_air_cell) in reveals {
            self.record_reveal(coord, cell, strength, source_air_cell);
        }
        self.step_waves.retain(|wave| {
            wave.age * ECHO_STEP_WAVE_SPEED < ECHO_STEP_WAVE_MAX_RADIUS
                && wave.next_impact < wave.impacts.len()
        });
        self.waves
            .retain(|wave| wave.next_impact < wave.impacts.len());
        let slots = config.max_active_waves.saturating_sub(self.waves.len());
        self.waves
            .extend(secondary_sources.into_iter().take(slots).map(
                |(source, energy, bounce_depth, original_emission_position)| {
                    let range = config.max_range * energy.clamp(0.2, 1.0);
                    build_echo_wave(
                        &self.world,
                        source,
                        energy,
                        bounce_depth,
                        range,
                        original_emission_position,
                    )
                },
            ));
        let corrected_player_position = self.update_puzzle(
            dt,
            receiver_hits,
            player_position,
            listener_right,
            &mut effects,
        );
        EchoFrameUpdate {
            sound_events: effects,
            corrected_player_position,
        }
    }

    fn update_puzzle(
        &mut self,
        dt: f32,
        mut receiver_hits: Vec<f32>,
        player_position: Vec3,
        listener_right: Vec3,
        effects: &mut Vec<SoundEffect>,
    ) -> Option<Vec3> {
        let frame_start = self.puzzle.time;
        let frame_end = frame_start + dt;
        self.puzzle.transitions.clear();
        receiver_hits.sort_by(|a, b| a.total_cmp(b));
        for hit_time in receiver_hits {
            self.puzzle.record_activation(hit_time);
            effects.push(spatial_puzzle_effect(
                PuzzleSoundEffect::Receiver,
                echo_receiver_sound_position(self.puzzle.receiver.coord),
                player_position,
                listener_right,
            ));
        }

        let delay = self.puzzle.endpoint_distance() / ECHO_PUZZLE_SIGNAL_SPEED;
        let mut transitions = Vec::new();
        for interval in &self.puzzle.emissions {
            let open_time = interval.start + delay;
            let close_time = interval.end + delay;
            if open_time > frame_start && open_time <= frame_end {
                transitions.push((open_time, EchoDoorTransition::Opened));
            }
            if close_time > frame_start && close_time <= frame_end {
                transitions.push((close_time, EchoDoorTransition::Closed));
            }
        }
        transitions.sort_by(|a, b| {
            a.0.total_cmp(&b.0).then_with(|| match (a.1, b.1) {
                (EchoDoorTransition::Closed, EchoDoorTransition::Opened) => {
                    std::cmp::Ordering::Less
                }
                (EchoDoorTransition::Opened, EchoDoorTransition::Closed) => {
                    std::cmp::Ordering::Greater
                }
                _ => std::cmp::Ordering::Equal,
            })
        });

        let mut corrected_player = player_position;
        let mut player_was_corrected = false;
        for (_, transition) in transitions {
            match transition {
                EchoDoorTransition::Opened if !self.puzzle.door_open => {
                    self.set_puzzle_door_open(true);
                    effects.push(spatial_puzzle_effect(
                        PuzzleSoundEffect::DoorOpen,
                        echo_door_sound_position(),
                        corrected_player,
                        listener_right,
                    ));
                    self.puzzle.transitions.push(transition);
                }
                EchoDoorTransition::Closed if self.puzzle.door_open => {
                    self.set_puzzle_door_open(false);
                    if echo_door_overlaps(
                        &self.puzzle.door,
                        corrected_player,
                        ECHOLOCATION_WALK_PROFILE.collision_radius,
                    ) {
                        corrected_player = echo_door_clear_position(
                            &self.world,
                            &self.puzzle.door,
                            corrected_player,
                            ECHOLOCATION_WALK_PROFILE,
                        );
                        player_was_corrected = true;
                    }
                    if echo_door_overlaps(
                        &self.puzzle.door,
                        self.pursuer.position,
                        ECHOLOCATION_WALK_PROFILE.collision_radius,
                    ) {
                        self.pursuer.position = echo_door_clear_position(
                            &self.world,
                            &self.puzzle.door,
                            self.pursuer.position,
                            ECHOLOCATION_WALK_PROFILE,
                        );
                    }
                    effects.push(spatial_puzzle_effect(
                        PuzzleSoundEffect::DoorClose,
                        echo_door_sound_position(),
                        corrected_player,
                        listener_right,
                    ));
                    self.puzzle.transitions.push(transition);
                }
                _ => {}
            }
        }
        self.puzzle.time = frame_end;
        self.puzzle
            .emissions
            .retain(|interval| interval.end + delay >= frame_end);
        player_was_corrected.then_some(corrected_player)
    }

    fn set_puzzle_door_open(&mut self, open: bool) {
        for coord in &self.puzzle.door.voxels {
            if open {
                self.world.clear(*coord);
            } else {
                self.world
                    .set(*coord, VoxelCell::new(VoxelMaterial::PuzzleDoor));
            }
        }
        self.puzzle.door_open = open;
    }

    fn update_pursuer(
        &mut self,
        dt: f32,
        player_position: Vec3,
        listener_right: Vec3,
        effects: &mut Vec<SoundEffect>,
    ) {
        self.advance_pursuer_mode(dt);
        if let Some(target) = self.pursuer.target_position {
            self.move_pursuer_toward(dt, target);
        }
        self.update_pursuer_footsteps(dt, player_position, listener_right, effects);
        if horizontal_distance(self.pursuer.position, player_position)
            <= ECHO_PURSUER_CONTACT_RADIUS
        {
            self.run_status = EchoRunStatus::Dead;
        }
    }

    fn advance_pursuer_mode(&mut self, dt: f32) {
        let mode = self.pursuer.mode;
        match mode {
            EchoPursuerMode::Wander => {
                self.pursuer.wander_idle_remaining =
                    (self.pursuer.wander_idle_remaining - dt).max(0.0);
                if self.pursuer.wander_idle_remaining <= 0.0
                    && self.pursuer.target_position.is_none()
                {
                    self.pursuer.target_position = self.select_pursuer_destination(None);
                }
                if self.pursuer_target_reached() {
                    self.pursuer.target_position = None;
                    if self.next_pursuer_random() % 4 == 0 {
                        self.pursuer.wander_idle_remaining =
                            0.4 + (self.next_pursuer_random() % 80) as f32 / 100.0;
                    }
                }
            }
            EchoPursuerMode::Investigate {
                last_heard_position,
                remaining_seconds,
            } => {
                let remaining_seconds = (remaining_seconds - dt).max(0.0);
                if remaining_seconds <= 0.0 {
                    self.pursuer.mode = EchoPursuerMode::Wander;
                    self.pursuer.target_position = None;
                    self.pursuer.wander_idle_remaining = 0.0;
                    return;
                }
                self.pursuer.mode = EchoPursuerMode::Investigate {
                    last_heard_position,
                    remaining_seconds,
                };
                if self.pursuer.target_position.is_none() {
                    self.pursuer.target_position = Some(last_heard_position);
                }
                if self.pursuer_target_reached() {
                    self.pursuer.target_position = self
                        .select_pursuer_destination(Some(last_heard_position))
                        .or(Some(last_heard_position));
                }
            }
        }
    }

    fn move_pursuer_toward(&mut self, dt: f32, target: Vec3) {
        let Some(navigation) =
            NavigationField::build(&self.world, target, ECHOLOCATION_WALK_PROFILE)
        else {
            self.pursuer.target_position = None;
            return;
        };
        if let Some(step) = navigation.next_step(self.pursuer.position) {
            // Never overshoot a navigation-cell center: a long render frame
            // otherwise can push the pursuer into a wall-adjacent collision zone
            // where it oscillates instead of entering the next corridor.
            let candidate = approach_vec3(
                self.pursuer.position,
                Vec3::new(step.x, self.pursuer.position.y, step.z),
                ECHO_PURSUER_SPEED * dt,
            );
            let previous = self.pursuer.position;
            self.pursuer.position = move_walking_with_collision(
                self.pursuer.position,
                candidate - self.pursuer.position,
                &self.world,
                ECHOLOCATION_WALK_PROFILE,
            );
            let moved = horizontal(self.pursuer.position - previous);
            if moved.length() > f32::EPSILON {
                self.pursuer.travel_direction = moved.normalized();
            }
        }
    }

    fn update_pursuer_footsteps(
        &mut self,
        dt: f32,
        player_position: Vec3,
        listener_right: Vec3,
        effects: &mut Vec<SoundEffect>,
    ) {
        self.pursuer.step_timer -= dt;
        while self.pursuer.step_timer <= 0.0 {
            self.pursuer.step_timer += ECHO_PURSUER_STEP_SECONDS;
            let side = Vec3::new(
                -self.pursuer.travel_direction.z,
                0.0,
                self.pursuer.travel_direction.x,
            );
            for (foot_offset, left) in [(-0.13, true), (0.13, false)] {
                let foot_position = self.pursuer.position + side * foot_offset
                    - self.pursuer.travel_direction * 0.1
                    + Vec3::new(
                        0.0,
                        ECHO_FOOTPRINT_SURFACE_Y - ECHOLOCATION_WALK_PROFILE.eye_height,
                        0.0,
                    );
                self.footprints.push(EchoFootprint {
                    position: foot_position,
                    remaining_seconds: ECHO_FOOTPRINT_LIFETIME,
                    left,
                    travel_direction: self.pursuer.travel_direction,
                });
                self.step_waves.push(EchoStepWave {
                    origin: foot_position,
                    age: 0.0,
                    impacts: build_echo_wave(
                        &self.world,
                        echo_pursuer_foot_source(foot_position),
                        1.0,
                        0,
                        ECHO_STEP_WAVE_MAX_RADIUS,
                        foot_position,
                    )
                    .impacts,
                    next_impact: 0,
                });
            }
            effects.push(invisible_footstep_effect(
                self.pursuer.position,
                player_position,
                listener_right,
            ));
        }
        for footprint in &mut self.footprints {
            footprint.remaining_seconds -= dt;
        }
        self.footprints
            .retain(|print| print.remaining_seconds > 0.0);
        for wave in &mut self.step_waves {
            wave.age += dt;
        }
    }

    fn notify_pursuer_of_noise(&mut self, position: Vec3) {
        let floor_position =
            Vec3::new(position.x, ECHOLOCATION_WALK_PROFILE.eye_height, position.z);
        self.pursuer.mode = EchoPursuerMode::Investigate {
            last_heard_position: floor_position,
            remaining_seconds: ECHO_PURSUER_INVESTIGATE_SECONDS,
        };
        self.pursuer.target_position = Some(floor_position);
        self.pursuer.wander_idle_remaining = 0.0;
        self.static_burst_timer = 0.0;
        self.static_burst_counter = 0;
    }

    fn update_static_bursts(
        &mut self,
        dt: f32,
        listener_position: Vec3,
        listener_right: Vec3,
        effects: &mut Vec<SoundEffect>,
    ) {
        let effect = echo_search_effect(self, listener_position);
        if effect.corruption_level <= 1 {
            return;
        }
        self.static_burst_timer -= dt;
        while self.static_burst_timer <= 0.0 {
            let hash =
                echo_static_hash((self.seed as u32) ^ 0xBADA_5500, self.static_burst_counter);
            let variation = (hash & 0xffff) as f32 / 65535.0;
            let (min_interval, max_interval, gain) = match effect.corruption_level {
                2 => (2.7, 3.3, 0.025),
                3 => (1.5, 2.0, 0.045),
                4 => (0.7, 1.0, 0.065),
                _ => (0.3, 0.55, 0.085),
            };
            self.static_burst_timer += min_interval + (max_interval - min_interval) * variation;
            self.static_burst_counter = self.static_burst_counter.wrapping_add(1);
            let pan = horizontal(self.pursuer.position - listener_position)
                .normalized()
                .dot(listener_right)
                .clamp(-1.0, 1.0);
            effects.push(SoundEffect::EcholocationStaticBurst {
                pan,
                gain,
                variant: hash >> 16,
            });
        }
    }

    fn notify_pursuer_of_footstep(&mut self, foot_position: Vec3) {
        if horizontal_distance(self.pursuer.position, foot_position)
            <= ECHO_PURSUER_FOOTSTEP_HEARING_RANGE
            && has_line_of_sight(
                &self.world,
                self.pursuer.position,
                foot_position + Vec3::new(0.0, 0.15, 0.0),
            )
        {
            self.notify_pursuer_of_noise(foot_position);
        }
    }

    fn pursuer_target_reached(&self) -> bool {
        self.pursuer
            .target_position
            .map(|target| horizontal_distance(self.pursuer.position, target) <= 0.18)
            .unwrap_or(false)
    }

    fn next_pursuer_random(&mut self) -> u64 {
        let mut value = self.pursuer.selection_rng;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.pursuer.selection_rng = value;
        value
    }

    /// Selects a seeded reachable floor cell. Search destinations remain near
    /// the remembered sound while wandering can use the whole component.
    fn select_pursuer_destination(&mut self, nearby: Option<Vec3>) -> Option<Vec3> {
        let navigation = NavigationField::build(
            &self.world,
            self.pursuer.position,
            ECHOLOCATION_WALK_PROFILE,
        )?;
        let mut candidates = Vec::new();
        for z in navigation.min_z..navigation.min_z + navigation.height as i32 {
            for x in navigation.min_x..navigation.min_x + navigation.width as i32 {
                let Some(distance) = navigation.distance(x, z) else {
                    continue;
                };
                if distance == u16::MAX || distance < 2 {
                    continue;
                }
                let position = Vec3::new(
                    x as f32 + 0.5,
                    ECHOLOCATION_WALK_PROFILE.eye_height,
                    z as f32 + 0.5,
                );
                if nearby
                    .map(|center| horizontal_distance(position, center) <= 6.0)
                    .unwrap_or(true)
                {
                    candidates.push(position);
                }
            }
        }
        (!candidates.is_empty())
            .then(|| candidates[self.next_pursuer_random() as usize % candidates.len()])
    }

    #[cfg(test)]
    fn visible_world(&self, _player_position: Vec3) -> VoxelWorld {
        if self.show_full_map {
            return self.world.clone();
        }
        let mut visible = VoxelWorld::new();
        for (coord, reveal) in &self.revealed {
            if reveal.strength >= self.config.minimum_visible_energy {
                visible.set(*coord, reveal.cell);
            }
        }
        visible
    }

    fn record_reveal(
        &mut self,
        coord: VoxelCoord,
        cell: VoxelCell,
        strength: f32,
        _source_air_cell: VoxelCoord,
    ) {
        if strength < self.config.minimum_visible_energy {
            return;
        }
        self.revealed
            .entry(coord)
            .and_modify(|reveal| {
                reveal.cell = cell;
                reveal.strength = reveal.strength.max(strength);
                reveal.remaining_seconds = self.config.reveal_seconds;
            })
            .or_insert(EchoReveal {
                cell,
                strength,
                remaining_seconds: self.config.reveal_seconds,
            });
    }

    fn face_is_revealed(&self, coord: VoxelCoord, normal: Vec3) -> bool {
        if self.show_full_map {
            return true;
        }
        let voxel_revealed = self
            .revealed
            .get(&coord)
            .map(|reveal| reveal.strength >= self.config.minimum_visible_energy)
            .unwrap_or(false);
        let neighbor = VoxelCoord::new(
            coord.x + normal.x.round() as i32,
            coord.y + normal.y.round() as i32,
            coord.z + normal.z.round() as i32,
        );
        voxel_revealed && self.world.get(neighbor).is_none()
    }

    fn reflected_pulse_count(&self) -> usize {
        self.waves
            .iter()
            .filter(|wave| wave.bounce_depth > 0)
            .count()
    }

    fn toggle_tuning(&mut self) {
        self.tuning_open = !self.tuning_open;
    }

    fn toggle_full_map(&mut self) {
        self.show_full_map = !self.show_full_map;
    }

    fn apply_tuning(&mut self, action: EchoTuningAction) {
        match action {
            EchoTuningAction::IncreaseRange => {
                self.config.max_range = (self.config.max_range + 4.0).min(160.0)
            }
            EchoTuningAction::DecreaseRange => {
                self.config.max_range = (self.config.max_range - 4.0).max(12.0)
            }
            EchoTuningAction::IncreaseSpeed => {
                self.config.ping_speed = (self.config.ping_speed + 2.0).min(100.0)
            }
            EchoTuningAction::DecreaseSpeed => {
                self.config.ping_speed = (self.config.ping_speed - 2.0).max(6.0)
            }
            EchoTuningAction::IncreaseStrength => {
                self.config.echo_strength = (self.config.echo_strength + 0.05).min(1.0)
            }
            EchoTuningAction::DecreaseStrength => {
                self.config.echo_strength = (self.config.echo_strength - 0.05).max(0.0)
            }
            EchoTuningAction::ResetDefaults => self.config = EchoLocationConfig::default(),
        }
    }
}

fn voxel_coord_at(point: Vec3) -> VoxelCoord {
    VoxelCoord::new(
        point.x.floor() as i32,
        point.y.floor() as i32,
        point.z.floor() as i32,
    )
}

fn for_each_voxel(world: &VoxelWorld, mut visit: impl FnMut(VoxelCoord, VoxelCell)) {
    let Some(bounds) = world.bounds() else {
        return;
    };
    for y in bounds.min.y..=bounds.max.y {
        for z in bounds.min.z..=bounds.max.z {
            for x in bounds.min.x..=bounds.max.x {
                let coord = VoxelCoord::new(x, y, z);
                if let Some(cell) = world.get(coord) {
                    visit(coord, cell);
                }
            }
        }
    }
}

/// Removes dense horizontal layers above the world's floor. This captures the
/// large roof/ceiling slabs used by the indoor maps while retaining sparse
/// props, lights, and vertical structures.
fn without_map_ceilings(world: &VoxelWorld) -> VoxelWorld {
    let Some(bounds) = world.bounds() else {
        return world.clone();
    };

    let mut counts_by_y = HashMap::new();
    for_each_voxel(world, |coord, _| {
        *counts_by_y.entry(coord.y).or_insert(0usize) += 1;
    });
    let densest_layer = counts_by_y.values().copied().max().unwrap_or(0);
    let ceiling_levels: HashSet<i32> = counts_by_y
        .into_iter()
        .filter_map(|(y, count)| {
            (y > bounds.min.y && count.saturating_mul(2) >= densest_layer).then_some(y)
        })
        .collect();

    if ceiling_levels.is_empty() {
        return world.clone();
    }

    let mut ceilingless = world.clone();
    let mut remove = Vec::new();
    for_each_voxel(world, |coord, _| {
        if ceiling_levels.contains(&coord.y) {
            remove.push(coord);
        }
    });
    for coord in remove {
        ceilingless.clear(coord);
    }
    ceilingless
}

fn build_echo_wave(
    world: &VoxelWorld,
    source: VoxelCoord,
    energy: f32,
    bounce_depth: u8,
    max_range: f32,
    original_emission_position: Vec3,
) -> EchoWave {
    EchoWave {
        age: 0.0,
        energy,
        bounce_depth,
        original_emission_position,
        heard_by_pursuer: false,
        impacts: echo_impacts(world, source, max_range),
        next_impact: 0,
    }
}

fn echo_impacts(world: &VoxelWorld, source: VoxelCoord, max_range: f32) -> Vec<EchoImpact> {
    let Some(bounds) = world.bounds() else {
        return Vec::new();
    };
    if !echo_coord_in_bounds(source, bounds.min, bounds.max) || world.get(source).is_some() {
        return Vec::new();
    }

    let max_distance = (max_range.max(0.0) * 1000.0).floor() as u32;
    let mut queue = BinaryHeap::new();
    let mut distances = HashMap::new();
    let mut impacts = HashMap::<VoxelCoord, EchoImpact>::new();
    distances.insert(source, 0_u32);
    queue.push((Reverse(0_u32), source));

    while let Some((Reverse(distance), air_cell)) = queue.pop() {
        if distances.get(&air_cell).copied() != Some(distance) {
            continue;
        }

        for (dx, dy, dz) in [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            let solid_voxel = echo_offset_coord(air_cell, dx, dy, dz);
            let arrival_distance_milli = distance.saturating_add(1000);
            if arrival_distance_milli > max_distance {
                continue;
            }
            let Some(cell) = world.get(solid_voxel) else {
                continue;
            };
            let candidate = EchoImpact {
                solid_voxel,
                cell,
                source_air_cell: air_cell,
                arrival_distance_milli,
            };
            if impacts.get(&solid_voxel).map_or(true, |current| {
                arrival_distance_milli < current.arrival_distance_milli
            }) {
                impacts.insert(solid_voxel, candidate);
            }
        }

        for dx in -1_i32..=1 {
            for dy in -1_i32..=1 {
                for dz in -1_i32..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let neighbor = echo_offset_coord(air_cell, dx, dy, dz);
                    if !echo_coord_in_bounds(neighbor, bounds.min, bounds.max)
                        || world.get(neighbor).is_some()
                        || !echo_diagonal_move_is_clear(world, air_cell, dx, dy, dz)
                    {
                        continue;
                    }
                    let changed_axes = dx.unsigned_abs() + dy.unsigned_abs() + dz.unsigned_abs();
                    let step_cost = match changed_axes {
                        1 => 1000,
                        2 => 1414,
                        3 => 1732,
                        _ => unreachable!(),
                    };
                    let next_distance = distance + step_cost;
                    if next_distance > max_distance
                        || distances
                            .get(&neighbor)
                            .is_some_and(|known| *known <= next_distance)
                    {
                        continue;
                    }
                    distances.insert(neighbor, next_distance);
                    queue.push((Reverse(next_distance), neighbor));
                }
            }
        }
    }

    let mut impacts: Vec<_> = impacts.into_values().collect();
    impacts.sort_by_key(|impact| (impact.arrival_distance_milli, impact.solid_voxel));
    impacts
}

fn echo_coord_in_bounds(coord: VoxelCoord, min: VoxelCoord, max: VoxelCoord) -> bool {
    (min.x..=max.x).contains(&coord.x)
        && (min.y..=max.y).contains(&coord.y)
        && (min.z..=max.z).contains(&coord.z)
}

fn echo_offset_coord(coord: VoxelCoord, dx: i32, dy: i32, dz: i32) -> VoxelCoord {
    VoxelCoord::new(coord.x + dx, coord.y + dy, coord.z + dz)
}

fn echo_diagonal_move_is_clear(
    world: &VoxelWorld,
    source: VoxelCoord,
    dx: i32,
    dy: i32,
    dz: i32,
) -> bool {
    let mut axes = [(0, 0, 0); 3];
    let mut axis_count = 0;
    for axis in [(dx, 0, 0), (0, dy, 0), (0, 0, dz)] {
        if axis != (0, 0, 0) {
            axes[axis_count] = axis;
            axis_count += 1;
        }
    }
    if axis_count <= 1 {
        return true;
    }

    let full_mask = (1_usize << axis_count) - 1;
    for mask in 1..full_mask {
        let mut offset = (0, 0, 0);
        for (index, axis) in axes[..axis_count].iter().enumerate() {
            if mask & (1 << index) != 0 {
                offset.0 += axis.0;
                offset.1 += axis.1;
                offset.2 += axis.2;
            }
        }
        if world
            .get(echo_offset_coord(source, offset.0, offset.1, offset.2))
            .is_some()
        {
            return false;
        }
    }
    true
}

fn echo_reflection_gain(strength: f32) -> f32 {
    0.15 + strength.clamp(0.0, 1.0) * 0.85
}

fn echo_bounce_limit(strength: f32) -> u8 {
    match strength {
        strength if strength < 0.25 => 0,
        strength if strength < 0.65 => 1,
        strength if strength < 0.90 => 2,
        _ => 3,
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
    name: String,
    world: VoxelWorld,
    center: Vec3,
    radius: f32,
    voxel_size: f32,
    dimensions: [i32; 3],
    source: AssetSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetSource {
    BuiltIn,
    Imported,
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
    fn new(name: impl Into<String>, world: VoxelWorld) -> Self {
        let (center, radius) = asset_bounds(&world);
        let dimensions = asset_dimensions(&world);
        Self {
            name: name.into(),
            world,
            center,
            radius,
            voxel_size: 1.0,
            dimensions,
            source: AssetSource::BuiltIn,
        }
    }
}

#[derive(Clone, Debug)]
struct AssetViewerState {
    assets: Vec<PreviewAsset>,
    load_errors: Vec<String>,
    selected: usize,
    camera: Camera,
    distance: f32,
}

#[derive(Clone, Debug)]
struct PreviewMap {
    name: String,
    world: VoxelWorld,
    start_camera: Camera,
    center: Vec3,
    radius: f32,
    dimensions: [i32; 3],
    definition: String,
}

impl PreviewMap {
    fn new(
        name: impl Into<String>,
        world: VoxelWorld,
        start_camera: Camera,
        definition: impl Into<String>,
    ) -> Self {
        let (center, radius) = asset_bounds(&world);
        let dimensions = asset_dimensions(&world);
        Self {
            name: name.into(),
            world,
            start_camera,
            center,
            radius,
            dimensions,
            definition: definition.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapViewerView {
    FreeFlight,
    Orbit,
}

/// Map inspection should never lose finite map geometry to a render-distance
/// cutoff, regardless of how far the camera has flown from its start point.
fn map_viewer_camera(mut camera: Camera) -> Camera {
    camera.max_distance = f32::INFINITY;
    camera
}

#[derive(Clone, Debug)]
struct MapViewerState {
    maps: Vec<PreviewMap>,
    selected: usize,
    camera: Camera,
    target: Vec3,
    distance: f32,
    view: MapViewerView,
    ceilings_hidden: bool,
    ceilingless_world: Option<VoxelWorld>,
}

impl MapViewerState {
    fn new(maps: Vec<PreviewMap>) -> Self {
        assert!(!maps.is_empty(), "map viewer requires at least one map");
        let target = maps[0].center;
        let distance = map_viewer_default_distance(&maps[0]);
        let camera = map_viewer_camera(maps[0].start_camera);
        Self {
            maps,
            selected: 0,
            camera,
            target,
            distance,
            view: MapViewerView::FreeFlight,
            ceilings_hidden: false,
            ceilingless_world: None,
        }
    }

    fn selected_map(&self) -> &PreviewMap {
        &self.maps[self.selected]
    }

    fn select(&mut self, index: usize) {
        if index >= self.maps.len() {
            return;
        }
        self.selected = index;
        self.rebuild_ceilingless_world();
        self.reset_view();
    }

    fn select_next(&mut self) {
        self.select((self.selected + 1) % self.maps.len());
    }

    fn select_previous(&mut self) {
        self.select((self.selected + self.maps.len() - 1) % self.maps.len());
    }

    fn update(&mut self, input: &PlayerInput, dt: f32) {
        if self.view == MapViewerView::FreeFlight {
            update_map_viewer_free_camera(&mut self.camera, input, dt);
            return;
        }

        let speed = if input.boost { BOOST_MULTIPLIER } else { 1.0 };
        if input.roll_left {
            self.camera.roll_by(MAP_VIEWER_ROLL_SPEED * speed * dt);
        }
        if input.roll_right {
            self.camera.roll_by(-MAP_VIEWER_ROLL_SPEED * speed * dt);
        }
        if input.up {
            self.distance -= MAP_VIEWER_ZOOM_SPEED * speed * dt;
        }
        if input.down {
            self.distance += MAP_VIEWER_ZOOM_SPEED * speed * dt;
        }

        let pan_speed = (self.selected_map().radius * 0.75).max(18.0) * speed * dt;
        let pan_forward = horizontal(self.camera.forward()).normalized();
        let pan_forward = if pan_forward.length() <= f32::EPSILON {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            pan_forward
        };
        let pan_right = horizontal(self.camera.right()).normalized();
        let pan_right = if pan_right.length() <= f32::EPSILON {
            Vec3::new(pan_forward.z, 0.0, -pan_forward.x)
        } else {
            pan_right
        };
        if input.forward || input.pan_forward {
            self.target = self.target + pan_forward * pan_speed;
        }
        if input.backward || input.pan_backward {
            self.target = self.target - pan_forward * pan_speed;
        }
        if input.right || input.pan_right {
            self.target = self.target + pan_right * pan_speed;
        }
        if input.left || input.pan_left {
            self.target = self.target - pan_right * pan_speed;
        }

        self.clamp_view();
        self.sync_camera_position();
    }

    fn rotate_with_mouse(&mut self, delta_x: f32, delta_y: f32) {
        match self.view {
            MapViewerView::FreeFlight => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Clamped);
            }
            MapViewerView::Orbit => {
                self.camera.rotate_local_yaw_pitch(
                    delta_x * MAP_VIEWER_MOUSE_SENSITIVITY,
                    delta_y * MAP_VIEWER_MOUSE_SENSITIVITY,
                );
                self.sync_camera_position();
            }
        }
    }

    fn camera(&self) -> Camera {
        self.camera
    }

    fn reset_view(&mut self) {
        self.target = self.selected_map().center;
        self.distance = map_viewer_default_distance(self.selected_map());
        self.camera = map_viewer_camera(match self.view {
            MapViewerView::FreeFlight => map_viewer_camera(self.selected_map().start_camera),
            MapViewerView::Orbit => {
                map_viewer_start_camera(self.target, self.selected_map().radius, self.distance)
            }
        });
    }

    fn toggle_view(&mut self) {
        self.view = match self.view {
            MapViewerView::FreeFlight => {
                self.reset_view();
                MapViewerView::Orbit
            }
            MapViewerView::Orbit => MapViewerView::FreeFlight,
        };
    }

    fn toggle_ceilings(&mut self) {
        self.ceilings_hidden = !self.ceilings_hidden;
        self.rebuild_ceilingless_world();
    }

    fn render_world(&self) -> &VoxelWorld {
        self.ceilingless_world
            .as_ref()
            .unwrap_or(&self.selected_map().world)
    }

    fn rebuild_ceilingless_world(&mut self) {
        self.ceilingless_world = self
            .ceilings_hidden
            .then(|| without_map_ceilings(&self.selected_map().world));
    }

    fn clamp_view(&mut self) {
        let radius = self.selected_map().radius;
        let min_distance = (radius * 0.12).max(6.0);
        let max_distance = (radius * 8.0).max(120.0);
        self.distance = self.distance.clamp(min_distance, max_distance);
        self.camera.max_distance = f32::INFINITY;
    }

    fn sync_camera_position(&mut self) {
        self.camera.position = self.target - self.camera.forward() * self.distance;
    }
}

impl AssetViewerState {
    fn new() -> Self {
        let (assets, load_errors) = build_asset_catalog();
        let camera = asset_viewer_start_camera(&assets[0], ASSET_VIEWER_DEFAULT_DISTANCE);
        let mut viewer = Self {
            assets,
            load_errors,
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

            let profile = enemy.profile();
            enemy.attack_cooldown = (enemy.attack_cooldown - dt).max(0.0);
            let to_player = horizontal(player_position - enemy.position);
            let distance = horizontal_distance(enemy.position, player_position);

            if distance <= profile.attack_range {
                if enemy.attack_cooldown <= 0.0 {
                    self.health = (self.health - profile.attack_damage).max(0);
                    enemy.attack_cooldown = profile.attack_cooldown;
                    player_hurt = true;
                }
            } else if distance < 80.0 && to_player.length() > f32::EPSILON {
                let candidate = enemy.position + to_player * profile.speed * dt;
                let candidate = Vec3::new(candidate.x, profile.eye_height, candidate.z);
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
        if enemy.take_damage(WEAPON_DAMAGE) {
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
    enemy_type: EnemyType,
    position: Vec3,
    health: i32,
    max_health: i32,
    attack_cooldown: f32,
}

impl Enemy {
    fn new(enemy_type: EnemyType, position: Vec3, round: u32) -> Self {
        let profile = enemy_type.profile();
        let max_health =
            profile.base_health + profile.health_per_round * round.saturating_sub(1) as i32;
        Self {
            enemy_type,
            position,
            health: max_health,
            max_health,
            attack_cooldown: 0.0,
        }
    }

    fn profile(self) -> EnemyProfile {
        self.enemy_type.profile()
    }

    fn take_damage(&mut self, amount: i32) -> bool {
        let was_alive = self.is_alive();
        self.health = (self.health - amount.max(0)).max(0);
        was_alive && !self.is_alive()
    }

    fn is_alive(self) -> bool {
        self.health > 0
    }

    fn contains_voxel(self, coord: VoxelCoord) -> bool {
        enemy_body_contains_voxel(self.enemy_type, self.position, coord)
    }
}

fn spawn_enemies() -> Vec<Enemy> {
    vec![
        Enemy::new(EnemyType::Clown, Vec3::new(0.5, 0.0, -34.5), 1),
        Enemy::new(EnemyType::Clown, Vec3::new(-42.5, 0.0, -22.5), 1),
        Enemy::new(EnemyType::Clown, Vec3::new(39.5, 0.0, -5.5), 1),
        Enemy::new(EnemyType::Clown, Vec3::new(-18.5, 0.0, 39.5), 1),
        Enemy::new(EnemyType::Clown, Vec3::new(22.5, 0.0, 42.5), 1),
        Enemy::new(EnemyType::Clown, Vec3::new(0.5, 0.0, 53.5), 1),
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

#[derive(Clone, Debug)]
struct ZombiesState {
    zombies: Vec<Enemy>,
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
        input: &mut PlayerInput,
        motion: &mut WalkMotion,
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

        // Sprint has already been folded into `speed`; do not apply the
        // generic walk boost a second time.
        let boost = input.boost;
        input.boost = false;
        update_jumping_walking_camera(
            camera,
            input,
            motion,
            world,
            WalkProfile {
                eye_height: ZOMBIE_EYE_HEIGHT,
                speed,
                collision_radius: ZOMBIE_COLLISION_RADIUS,
            },
            dt,
        );
        input.boost = boost;
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

        let navigation = NavigationField::build(world, player_position, zombie_walk_profile());
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
            let profile = zombie.profile();
            let distance = horizontal_distance(zombie.position, player_position);
            if distance <= profile.attack_range {
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
                    zombie.position + horizontal(step - zombie.position) * profile.speed * dt;
                let candidate = Vec3::new(candidate.x, profile.eye_height, candidate.z);
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
        let killed = zombie.take_damage(self.weapon.damage());
        self.points += ZOMBIE_HIT_POINTS;
        self.total_points += ZOMBIE_HIT_POINTS;
        if killed {
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
        self.zombies
            .push(Enemy::new(EnemyType::Zombie, spawns[index], self.round));
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
    let profile = EnemyType::Zombie.profile();
    WalkProfile {
        eye_height: profile.eye_height,
        speed: profile.speed,
        collision_radius: profile.collision_radius,
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

/// Select the farthest cell in the player's connected walkable component.
/// This makes the invisible pursuer deterministic and prevents impossible
/// starts across a sealed wall.
fn echo_pursuer_spawn_position(world: &VoxelWorld, start: Vec3) -> Option<Vec3> {
    let navigation = NavigationField::build(world, start, ECHOLOCATION_WALK_PROFILE)?;
    let mut farthest = None;
    for z in navigation.min_z..navigation.min_z + navigation.height as i32 {
        for x in navigation.min_x..navigation.min_x + navigation.width as i32 {
            let Some(distance) = navigation.distance(x, z) else {
                continue;
            };
            if distance == u16::MAX {
                continue;
            }
            if farthest.map(|(best, _, _)| distance > best).unwrap_or(true) {
                farthest = Some((distance, x, z));
            }
        }
    }
    farthest.map(|(_, x, z)| {
        Vec3::new(
            x as f32 + 0.5,
            ECHOLOCATION_WALK_PROFILE.eye_height,
            z as f32 + 0.5,
        )
    })
}

struct NavigationField {
    min_x: i32,
    min_z: i32,
    width: usize,
    height: usize,
    distances: Vec<u16>,
    eye_height: f32,
}

impl NavigationField {
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

        if !navigation_cell_is_walkable(world, start_x, start_z, profile) {
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
                if !navigation_cell_is_walkable(world, nx, nz, profile) {
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
            eye_height: profile.eye_height,
        })
    }

    fn next_step(&self, position: Vec3) -> Option<Vec3> {
        let x = position.x.floor() as i32;
        let z = position.z.floor() as i32;
        let current = self.distance(x, z)?;
        if current == 0 {
            return Some(Vec3::new(x as f32 + 0.5, self.eye_height, z as f32 + 0.5));
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

        best.map(|(_, nx, nz)| Vec3::new(nx as f32 + 0.5, self.eye_height, nz as f32 + 0.5))
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

fn navigation_cell_is_walkable(world: &VoxelWorld, x: i32, z: i32, profile: WalkProfile) -> bool {
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
        PhysicalKey::Code(KeyCode::Space) => {
            input.up = pressed;
            if pressed {
                input.jump_requested = true;
            }
        }
        PhysicalKey::Code(KeyCode::ControlLeft) | PhysicalKey::Code(KeyCode::ControlRight) => {
            input.down = pressed
        }
        PhysicalKey::Code(KeyCode::KeyQ) => input.roll_left = pressed,
        PhysicalKey::Code(KeyCode::KeyE) => input.roll_right = pressed,
        PhysicalKey::Code(KeyCode::ArrowUp) => input.pan_forward = pressed,
        PhysicalKey::Code(KeyCode::ArrowDown) => input.pan_backward = pressed,
        PhysicalKey::Code(KeyCode::ArrowLeft) => input.pan_left = pressed,
        PhysicalKey::Code(KeyCode::ArrowRight) => input.pan_right = pressed,
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
    update_flight_camera_with_speed(camera, input, dt, FLIGHT_SPEED);
}

fn update_flight_camera_with_speed(
    camera: &mut Camera,
    input: &PlayerInput,
    dt: f32,
    base_speed: f32,
) {
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
    apply_flight_roll(camera, input, dt);

    if movement.length() > f32::EPSILON {
        let speed = if input.boost {
            base_speed * BOOST_MULTIPLIER
        } else {
            base_speed
        };
        camera.position = camera.position + movement.normalized() * speed * dt;
    }
}

fn apply_flight_roll(camera: &mut Camera, input: &PlayerInput, dt: f32) {
    if input.roll_left {
        camera.roll_by(ROLL_SPEED * dt);
    }
    if input.roll_right {
        camera.roll_by(-ROLL_SPEED * dt);
    }
}

fn approach_vec3(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let delta = target - current;
    let distance = delta.length();
    if distance <= max_delta || distance <= f32::EPSILON {
        target
    } else {
        current + delta.normalized() * max_delta
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

fn update_jumping_walking_camera(
    camera: &mut Camera,
    input: &mut PlayerInput,
    motion: &mut WalkMotion,
    world: &VoxelWorld,
    profile: WalkProfile,
    dt: f32,
) {
    let mut horizontal_input = *input;
    horizontal_input.up = false;
    update_walking_camera_with_profile(camera, &horizontal_input, world, profile, dt);

    let grounded = walking_ground_y(world, camera.position, profile).is_some();
    if grounded {
        motion.airborne = false;
        motion.vertical_velocity = 0.0;
    }

    if input.jump_requested && grounded {
        motion.vertical_velocity = WALK_JUMP_SPEED;
        motion.airborne = true;
    }
    input.jump_requested = false;

    // A player who walks off a ledge begins falling; empty test worlds retain
    // their legacy flat-camera behavior until a jump-capable surface exists.
    if !motion.airborne && !grounded {
        return;
    }
    if !grounded {
        motion.airborne = true;
    }

    motion.vertical_velocity -= WALK_GRAVITY * dt;
    let vertical_step = motion.vertical_velocity * dt;
    if vertical_step > 0.0 {
        let candidate = Vec3::new(
            camera.position.x,
            camera.position.y + vertical_step,
            camera.position.z,
        );
        if can_walk_to_with_profile(world, candidate, profile) {
            camera.position = candidate;
        } else {
            motion.vertical_velocity = 0.0;
        }
    } else if vertical_step < 0.0 {
        let candidate = Vec3::new(
            camera.position.x,
            camera.position.y + vertical_step,
            camera.position.z,
        );
        if let Some(ground_y) = walking_landing_y(world, camera.position, candidate, profile) {
            camera.position.y = ground_y as f32 + profile.eye_height;
            motion.vertical_velocity = 0.0;
            motion.airborne = false;
        } else if can_walk_to_with_profile(world, candidate, profile) {
            camera.position = candidate;
        } else {
            motion.vertical_velocity = 0.0;
        }
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

/// The map viewer follows the sandbox's first-person controls, but deliberately
/// skips collision so every enclosed room, roof, and exterior can be inspected.
fn update_map_viewer_free_camera(camera: &mut Camera, input: &PlayerInput, dt: f32) {
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
        SANDBOX_SPEED * MAP_VIEWER_FREE_SPEED_MULTIPLIER * WALK_BOOST_MULTIPLIER
    } else {
        SANDBOX_SPEED * MAP_VIEWER_FREE_SPEED_MULTIPLIER
    };
    camera.position = camera.position + movement.normalized() * speed * dt;
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
    let full = Vec3::new(position.x + step.x, position.y, position.z + step.z);
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
        Vec3::new(resolved.x + primary, resolved.y, resolved.z)
    } else {
        Vec3::new(resolved.x, resolved.y, resolved.z + primary)
    };
    if can_walk_to_with_profile(city, primary_candidate, profile) {
        resolved = primary_candidate;
    }

    let secondary_candidate = if primary_is_x {
        Vec3::new(resolved.x, resolved.y, resolved.z + secondary)
    } else {
        Vec3::new(resolved.x + secondary, resolved.y, resolved.z)
    };
    if can_walk_to_with_profile(city, secondary_candidate, profile) {
        resolved = secondary_candidate;
    }

    resolved
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
    let foot_y = (position.y - profile.eye_height).floor() as i32;
    for y in (foot_y + 1)..=position.y.ceil() as i32 {
        if city.get(VoxelCoord::new(x, y, z)).is_some() {
            return false;
        }
    }
    true
}

fn walking_ground_y(world: &VoxelWorld, position: Vec3, profile: WalkProfile) -> Option<i32> {
    let x = position.x.floor() as i32;
    let z = position.z.floor() as i32;
    let min_y = (position.y - profile.eye_height).floor() as i32;
    let max_y = position.y.floor() as i32;
    (min_y..=max_y)
        .rev()
        .find(|&y| world.get(VoxelCoord::new(x, y, z)).is_some())
}

fn walking_landing_y(
    world: &VoxelWorld,
    current: Vec3,
    candidate: Vec3,
    profile: WalkProfile,
) -> Option<i32> {
    let current_foot = current.y - profile.eye_height;
    let candidate_foot = candidate.y - profile.eye_height;
    let top = current_foot.floor() as i32;
    let bottom = candidate_foot.ceil() as i32;
    (bottom..=top).rev().find(|&ground_y| {
        candidate_foot <= ground_y as f32
            && walking_ground_y(
                world,
                Vec3::new(
                    candidate.x,
                    ground_y as f32 + profile.eye_height,
                    candidate.z,
                ),
                profile,
            )
            .is_some()
    })
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

fn echo_pursuer_foot_source(position: Vec3) -> VoxelCoord {
    // Floor is y=0 in the mode; y=1 is the walkable air cell directly above it.
    VoxelCoord::new(position.x.floor() as i32, 1, position.z.floor() as i32)
}

fn invisible_footstep_effect(source: Vec3, listener: Vec3, listener_right: Vec3) -> SoundEffect {
    let (pan, gain) = spatial_sound_parameters(source, listener, listener_right, 0.18, 0.70, 42.0);
    SoundEffect::InvisibleFootstep { pan, gain }
}

#[derive(Clone, Copy)]
enum PuzzleSoundEffect {
    Receiver,
    DoorOpen,
    DoorClose,
}

fn spatial_puzzle_effect(
    kind: PuzzleSoundEffect,
    source: Vec3,
    listener: Vec3,
    listener_right: Vec3,
) -> SoundEffect {
    let (pan, gain) = spatial_sound_parameters(source, listener, listener_right, 0.08, 0.72, 52.0);
    match kind {
        PuzzleSoundEffect::Receiver => SoundEffect::ReceiverActivation { pan, gain },
        PuzzleSoundEffect::DoorOpen => SoundEffect::PuzzleDoorOpen { pan, gain },
        PuzzleSoundEffect::DoorClose => SoundEffect::PuzzleDoorClose { pan, gain },
    }
}

fn spatial_sound_parameters(
    source: Vec3,
    listener: Vec3,
    listener_right: Vec3,
    minimum_gain: f32,
    maximum_gain: f32,
    audible_distance: f32,
) -> (f32, f32) {
    let offset = horizontal(source - listener);
    let distance = Vec3::new(source.x - listener.x, 0.0, source.z - listener.z).length();
    let right = horizontal(listener_right);
    let pan = if distance > f32::EPSILON && right.length() > f32::EPSILON {
        offset.dot(right).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let gain = minimum_gain
        + (maximum_gain - minimum_gain) * (1.0 - distance / audible_distance).clamp(0.0, 1.0);
    (pan, gain.clamp(0.0, 1.0))
}

fn echo_receiver_sound_position(coord: VoxelCoord) -> Vec3 {
    Vec3::new(
        coord.x as f32 + 0.5,
        coord.y as f32 + 1.05,
        coord.z as f32 + 0.5,
    )
}

fn echo_door_sound_position() -> Vec3 {
    Vec3::new(ECHO_DOOR_X as f32 + 0.5, 2.5, 0.5)
}

fn echo_door_overlaps(door: &EchoPuzzleDoor, position: Vec3, radius: f32) -> bool {
    let plane_center = door.voxels[0].x as f32 + 0.5;
    let min_z = door.voxels.iter().map(|coord| coord.z).min().unwrap_or(0) as f32;
    let max_z = door.voxels.iter().map(|coord| coord.z).max().unwrap_or(0) as f32 + 1.0;
    position.x + radius > plane_center - 0.5
        && position.x - radius < plane_center + 0.5
        && position.z + radius > min_z
        && position.z - radius < max_z
}

fn echo_door_clear_position(
    world: &VoxelWorld,
    door: &EchoPuzzleDoor,
    position: Vec3,
    profile: WalkProfile,
) -> Vec3 {
    let start_distance = horizontal_distance(position, door.starting_side_anchor);
    let far_distance = horizontal_distance(position, door.far_side_anchor);
    let signed_side = (position - echo_door_sound_position()).dot(door.normal);
    let prefer_start = if (start_distance - far_distance).abs() <= 0.0001 {
        signed_side <= 0.0
    } else {
        start_distance < far_distance
    };
    let candidates = if prefer_start {
        [door.starting_side_anchor, door.far_side_anchor]
    } else {
        [door.far_side_anchor, door.starting_side_anchor]
    };
    candidates
        .into_iter()
        .find(|candidate| can_walk_to_with_profile(world, *candidate, profile))
        .unwrap_or(candidates[0])
}

fn has_line_of_sight(city: &VoxelWorld, origin: Vec3, target: Vec3) -> bool {
    let delta = target - origin;
    let distance = delta.length();
    if distance <= 0.5 {
        return true;
    }

    raycast(city, Ray::new(origin, delta), distance - 0.35).is_none()
}

fn has_line_of_sight_to_voxel(
    world: &VoxelWorld,
    origin: Vec3,
    target: Vec3,
    target_voxel: VoxelCoord,
) -> bool {
    let delta = target - origin;
    let distance = delta.length();
    raycast(world, Ray::new(origin, delta), distance + 0.01)
        .map(|hit| hit.coord == target_voxel)
        .unwrap_or(true)
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

fn build_map_catalog() -> Vec<PreviewMap> {
    let assets = AssetCatalog::discover(
        asset_directory().unwrap_or_else(|| Path::new("assets/voxel-assets").to_owned()),
    );
    let maps = MapCatalog::discover(
        map_directory().unwrap_or_else(|| Path::new("assets/voxel-maps").to_owned()),
        &assets,
    );
    build_map_catalog_from(&maps)
}

fn build_map_catalog_from(maps: &MapCatalog) -> Vec<PreviewMap> {
    let city = build_demo_city();
    let doom_map = build_doom_map();
    let corn_maze = CornMazeState::new();
    let corn_start_camera = corn_maze_start_camera(&corn_maze);
    let bar = maps
        .get("bar")
        .map(|map| map.fresh_session().world)
        .unwrap_or_else(build_bar_scene);
    let sandbox = build_voxel_sandbox_world();
    let zombies = ZombiesState::new();
    let zombies_map = maps
        .get("zombies")
        .map(|map| map.fresh_session().world)
        .unwrap_or_else(|| build_zombies_map(&zombies));
    let liminal = LiminalState::new_seeded(LIMINAL_SEED);
    let liminal_start_camera = liminal_start_camera(&liminal);
    let drone_runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
    let drone_start_camera = drone_gate_runner_start_camera(&drone_runner);
    let echolocation = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
    let echolocation_start_camera = echolocation_start_camera(&echolocation);

    vec![
        PreviewMap::new(
            "procedural city",
            city,
            city_start_camera(),
            "core generator",
        ),
        PreviewMap::new(
            "doomlike arena",
            maps.get("doom")
                .map(|m| m.world.clone())
                .unwrap_or(doom_map),
            maps.get("doom")
                .map(compiled_start_camera)
                .unwrap_or_else(doom_start_camera),
            if maps.get("doom").is_some() {
                "hbmap"
            } else {
                "core generator"
            },
        ),
        PreviewMap::new(
            "corn maze",
            corn_maze.world,
            corn_start_camera,
            "CLI generator",
        ),
        PreviewMap::new(
            "Starhusk bar",
            bar,
            maps.get("bar")
                .map(compiled_start_camera)
                .unwrap_or_else(bar_start_camera),
            if maps.get("bar").is_some() {
                "hbmap"
            } else {
                "CLI stamps"
            },
        ),
        PreviewMap::new(
            "voxel sandbox",
            sandbox.clone(),
            sandbox_start_camera(&sandbox),
            "CLI generator",
        ),
        PreviewMap::new(
            "Heliobound Zombies",
            zombies_map,
            maps.get("zombies")
                .map(compiled_start_camera)
                .unwrap_or_else(zombies_start_camera),
            if maps.get("zombies").is_some() {
                "hbmap"
            } else {
                "CLI stamps"
            },
        ),
        PreviewMap::new(
            "liminal office",
            maps.get("liminal-office")
                .map(|m| m.world.clone())
                .unwrap_or(liminal.world),
            maps.get("liminal-office")
                .map(compiled_start_camera)
                .unwrap_or(liminal_start_camera),
            if maps.get("liminal-office").is_some() {
                "hbmap"
            } else {
                "CLI generator"
            },
        ),
        PreviewMap::new(
            "drone gate course",
            drone_runner.render_world(),
            drone_start_camera,
            "CLI generator",
        ),
        PreviewMap::new(
            "echolocation",
            maps.get("echolocation")
                .map(|m| m.world.clone())
                .unwrap_or(echolocation.world),
            maps.get("echolocation")
                .map(compiled_start_camera)
                .unwrap_or(echolocation_start_camera),
            if maps.get("echolocation").is_some() {
                "hbmap"
            } else {
                "CLI generator"
            },
        ),
    ]
}

fn map_directory() -> Option<std::path::PathBuf> {
    [
        std::env::current_dir()
            .ok()
            .map(|path| path.join("assets/voxel-maps")),
        Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/voxel-maps")),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_dir())
}

fn compiled_start_camera(map: &heliobound_core::CompiledMap) -> Camera {
    match map.player_start {
        heliobound_core::MapMarker::PlayerSpawn {
            position,
            yaw_degrees,
            ..
        } => Camera::new(position).looking_at((yaw_degrees as f32).to_radians(), 0.0),
        _ => unreachable!("compiled maps always have a player spawn"),
    }
}

fn build_echolocation_map(seed: u64) -> (VoxelWorld, Vec3, EchoPuzzle) {
    let mut rng = LiminalRng::new(seed);
    let mut world = VoxelWorld::new();
    let rooms = [
        (-38, -8, 16, 16),
        (-8, -16, 20, 14),
        (18, -28, 18, 16),
        (-24, 16, 18, 14),
        (16, 14, 22, 18),
    ];

    for (x, z, width, depth) in rooms {
        stamp_echo_room(&mut world, x, z, width, depth);
    }

    stamp_echo_corridor(&mut world, -28, -3, -8, 3);
    stamp_echo_corridor(&mut world, -2, -14, 4, 18);
    stamp_echo_corridor(&mut world, 4, -14, 20, -14);
    stamp_echo_corridor(&mut world, -8, 18, 18, 22);
    seal_echolocation_hull(&mut world);

    for _ in 0..8 {
        let x = rng.range_i32(-30, 30);
        let z = rng.range_i32(-25, 25);
        if world.get(VoxelCoord::new(x, 1, z)).is_none()
            && world.get(VoxelCoord::new(x, 0, z)).is_some()
        {
            fill_cuboid(
                &mut world,
                VoxelCoord::new(x, 1, z),
                VoxelCoord::new(x, 3, z),
                VoxelMaterial::Stone,
            );
        }
    }

    fill_cuboid(
        &mut world,
        VoxelCoord::new(-5, 1, 24),
        VoxelCoord::new(5, 2, 25),
        VoxelMaterial::Glass,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(26, 1, 18),
        VoxelCoord::new(29, 4, 21),
        VoxelMaterial::Beacon,
    );
    fill_cuboid(
        &mut world,
        VoxelCoord::new(-27, 1, 21),
        VoxelCoord::new(-22, 1, 25),
        VoxelMaterial::Wood,
    );

    let puzzle = EchoPuzzle::new();
    // The bulkhead spans the complete east passage. Only the aperture is
    // mutable; its surrounding frame remains solid for the entire run.
    fill_cuboid(
        &mut world,
        VoxelCoord::new(ECHO_DOOR_X, 1, -5),
        VoxelCoord::new(ECHO_DOOR_X, 5, 5),
        VoxelMaterial::ShipHull,
    );
    for coord in &puzzle.door.voxels {
        world.set(*coord, VoxelCell::new(VoxelMaterial::PuzzleDoor));
    }
    for x in ECHO_RECEIVER_COORD.x..=ECHO_DOOR_X {
        world.set(
            VoxelCoord::new(x, 0, 0),
            VoxelCell::new(VoxelMaterial::SignalPipe),
        );
    }
    world.set(
        puzzle.receiver.coord,
        VoxelCell::new(VoxelMaterial::Receiver),
    );

    (world, Vec3::new(-28.5, WALK_EYE_HEIGHT, -0.5), puzzle)
}

fn stamp_echo_room(world: &mut VoxelWorld, x: i32, z: i32, width: i32, depth: i32) {
    fill_cuboid(
        world,
        VoxelCoord::new(x, 0, z),
        VoxelCoord::new(x + width, 0, z + depth),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(x, 6, z),
        VoxelCoord::new(x + width, 6, z + depth),
        VoxelMaterial::ShipHull,
    );
}

fn stamp_echo_corridor(world: &mut VoxelWorld, x1: i32, z1: i32, x2: i32, z2: i32) {
    let min_x = x1.min(x2) - 2;
    let max_x = x1.max(x2) + 2;
    let min_z = z1.min(z2) - 2;
    let max_z = z1.max(z2) + 2;
    fill_cuboid(
        world,
        VoxelCoord::new(min_x, 0, min_z),
        VoxelCoord::new(max_x, 0, max_z),
        VoxelMaterial::Basalt,
    );
    fill_cuboid(
        world,
        VoxelCoord::new(min_x, 6, min_z),
        VoxelCoord::new(max_x, 6, max_z),
        VoxelMaterial::ShipHull,
    );
}

/// Seal the perimeter of the combined room-and-corridor floor plan once all
/// of its pieces are stamped. This avoids gaps where overlapping pieces used
/// to overwrite one another's partial walls.
fn seal_echolocation_hull(world: &mut VoxelWorld) {
    let mut floor_tiles = Vec::new();
    for_each_voxel(world, |coord, cell| {
        if coord.y == 0 && cell.material == VoxelMaterial::Basalt {
            floor_tiles.push(coord);
        }
    });

    for floor in floor_tiles {
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let neighbor = VoxelCoord::new(floor.x + dx, 0, floor.z + dz);
            if world.get(neighbor).is_none() {
                fill_cuboid(
                    world,
                    VoxelCoord::new(neighbor.x, 1, neighbor.z),
                    VoxelCoord::new(neighbor.x, 5, neighbor.z),
                    VoxelMaterial::Stone,
                );
                world.set(
                    VoxelCoord::new(neighbor.x, 6, neighbor.z),
                    VoxelCell::new(VoxelMaterial::ShipHull),
                );
            }
        }
    }
}

fn runtime_seed_nonce() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

fn drone_course_seed(nonce: u64, run_index: u64) -> u64 {
    mix_seed(
        DRONE_GATE_SEED ^ nonce.rotate_left(17) ^ run_index.wrapping_mul(0x9E37_79B9_7F4A_7C15),
    )
}

fn mix_seed(seed: u64) -> u64 {
    let mut value = seed;
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

impl DroneGateCourseCursor {
    fn new(seed: u64, config: DroneGateRunnerConfig) -> Self {
        let mut rng = LiminalRng::new(seed);
        let min_altitude = config.gate_radius as f32 + 4.0;
        let mut position = Vec3::new(
            rng.range_f32(-config.course.lateral_jitter, config.course.lateral_jitter),
            config.course.base_altitude.max(min_altitude)
                + rng.range_f32(
                    -config.course.vertical_jitter_down,
                    config.course.vertical_jitter_up,
                ),
            0.0,
        );
        position.y = position.y.max(min_altitude);
        Self {
            rng,
            position,
            direction: Vec3::new(0.0, 0.0, 1.0),
            next_index: 0,
            min_altitude,
        }
    }

    fn next_position(&mut self, config: DroneGateRunnerConfig) -> Vec3 {
        let index = self.next_index;
        if index > 0 {
            let t = index as f32;
            let yaw = (t * config.course.lateral_wave_frequency).sin() * DRONE_GATE_COURSE_YAW_BEND
                + self.rng.range_f32(-0.16, 0.16);
            self.direction = self
                .direction
                .rotate_around(Vec3::new(0.0, 1.0, 0.0), yaw)
                .normalized();

            let right = drone_gate_basis(self.direction).0;
            let pitch = (t * config.course.vertical_wave_frequency).cos()
                * DRONE_GATE_COURSE_PITCH_BEND
                + self.rng.range_f32(-0.08, 0.08);
            self.direction = self.direction.rotate_around(right, -pitch).normalized();

            let spacing = config.spacing
                * self.rng.range_f32(
                    1.0 - DRONE_GATE_COURSE_SPACING_JITTER,
                    1.0 + DRONE_GATE_COURSE_SPACING_JITTER,
                );
            self.position = self.position + self.direction * spacing;
            self.position.y = self.position.y.max(self.min_altitude);
        }

        let t = index as f32;
        let right = drone_gate_basis(self.direction).0;
        let vertical = Vec3::new(0.0, 1.0, 0.0);
        let lateral_offset = (t * config.course.lateral_wave_frequency).sin()
            * config.course.lateral_amplitude
            * 0.12
            + self.rng.range_f32(
                -config.course.lateral_jitter * 0.35,
                config.course.lateral_jitter * 0.35,
            );
        let vertical_offset = (t * config.course.vertical_wave_frequency).cos()
            * config.course.vertical_amplitude
            * 0.10
            + self.rng.range_f32(
                -config.course.vertical_jitter_down * 0.35,
                config.course.vertical_jitter_up * 0.35,
            );
        let target_position = self.position + right * lateral_offset + vertical * vertical_offset;
        self.next_index += 1;
        Vec3::new(
            target_position.x,
            target_position.y.max(self.min_altitude),
            target_position.z,
        )
    }
}

fn generate_drone_gate_course(seed: u64, config: DroneGateRunnerConfig) -> DroneGateCourse {
    let mut cursor = DroneGateCourseCursor::new(seed, config);
    let positions: Vec<Vec3> = (0..config.course.gate_count)
        .map(|_| cursor.next_position(config))
        .collect();
    DroneGateCourse {
        seed,
        name: "Relay Spine",
        gates: drone_gate_targets_from_positions(&positions),
    }
}

fn generate_drone_gate_course_with_cursor(
    seed: u64,
    config: DroneGateRunnerConfig,
) -> (DroneGateCourse, DroneGateCourseCursor) {
    let course = generate_drone_gate_course(seed, config);
    let mut cursor = DroneGateCourseCursor::new(seed, config);
    for _ in 0..course.gates.len() {
        cursor.next_position(config);
    }
    (course, cursor)
}

fn drone_gate_targets_from_positions(positions: &[Vec3]) -> Vec<DroneGateTarget> {
    positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let previous = index
                .checked_sub(1)
                .and_then(|previous| positions.get(previous))
                .copied();
            let next = positions.get(index + 1).copied();
            let normal = match (previous, next) {
                (Some(previous), Some(next)) => (next - previous).normalized(),
                (None, Some(next)) => (next - *position).normalized(),
                (Some(previous), None) => (*position - previous).normalized(),
                (None, None) => Vec3::new(0.0, 0.0, 1.0),
            };
            DroneGateTarget {
                position: *position,
                normal,
            }
        })
        .collect()
}

fn build_drone_gate_runner_world(state: &DroneGateRunnerState) -> VoxelWorld {
    let mut world = VoxelWorld::new();
    let (start, end) = state.visible_gate_range();
    stamp_drone_course_grid(&mut world, state, start, end);
    for index in start..end {
        let gate = state.course.gates[index];
        stamp_drone_gate(
            &mut world,
            gate,
            state.config,
            index == state.active_gate,
            index < state.active_gate,
        );
    }
    stamp_drone_start_marker(&mut world, state.start_position);
    world
}

fn stamp_drone_course_grid(
    world: &mut VoxelWorld,
    state: &DroneGateRunnerState,
    start: usize,
    end: usize,
) {
    let Some(first) = state.course.gates.get(start).copied() else {
        return;
    };
    let Some(last) = state.course.gates.get(end.saturating_sub(1)).copied() else {
        return;
    };
    let z_min = first.position.z.floor() as i32 - 56;
    let z_max = last.position.z.ceil() as i32 + 80;

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
            VoxelCoord::new(
                gate.position.x.floor() as i32 - 1,
                -16,
                gate.position.z.floor() as i32 - 1,
            ),
            VoxelCoord::new(
                gate.position.x.floor() as i32 + 1,
                -10,
                gate.position.z.floor() as i32 + 1,
            ),
            VoxelMaterial::ShipHull,
        );
    }
}

fn stamp_drone_gate(
    world: &mut VoxelWorld,
    target: DroneGateTarget,
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
    let center = target.position;
    let (right, up) = drone_gate_basis(target.normal);

    for segment in 0..DRONE_GATE_RING_SEGMENTS {
        let radians = segment as f32 / DRONE_GATE_RING_SEGMENTS as f32 * std::f32::consts::TAU;
        let point = center + right * (radians.cos() * radius) + up * (radians.sin() * radius);
        fill_cuboid(
            world,
            VoxelCoord::new(
                point.x.round() as i32 - config.tube_radius,
                point.y.round() as i32 - config.tube_radius,
                point.z.round() as i32 - DRONE_GATE_DEPTH,
            ),
            VoxelCoord::new(
                point.x.round() as i32 + config.tube_radius,
                point.y.round() as i32 + config.tube_radius,
                point.z.round() as i32 + DRONE_GATE_DEPTH,
            ),
            material,
        );
    }

    let crown = center + up * (config.gate_radius as f32 + 3.5);
    fill_cuboid(
        world,
        VoxelCoord::new(
            crown.x.round() as i32 - 1,
            crown.y.round() as i32 - 1,
            crown.z.round() as i32 - 1,
        ),
        VoxelCoord::new(
            crown.x.round() as i32 + 1,
            crown.y.round() as i32 + 2,
            crown.z.round() as i32 + 1,
        ),
        material,
    );
}

fn drone_gate_basis(normal: Vec3) -> (Vec3, Vec3) {
    let normal = normal.normalized();
    let world_up = Vec3::new(0.0, 1.0, 0.0);
    let mut right = world_up.cross(normal).normalized();
    if right.length() <= f32::EPSILON {
        right = Vec3::new(1.0, 0.0, 0.0);
    }
    let up = normal.cross(right).normalized();
    (right, up)
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

fn build_asset_catalog() -> (Vec<PreviewAsset>, Vec<String>) {
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
        ("receiver block", VoxelMaterial::Receiver),
        ("signal pipe block", VoxelMaterial::SignalPipe),
        ("puzzle door block", VoxelMaterial::PuzzleDoor),
    ] {
        assets.push(PreviewAsset::new(name, build_block_asset(material)));
    }

    assets.push(PreviewAsset::new("gun", build_weapon_asset()));
    assets.push(PreviewAsset::new(
        "drone race gate",
        build_drone_gate_asset(),
    ));

    let (imported, errors) = load_imported_assets();
    assets.extend(imported);
    (assets, errors)
}

#[derive(Debug, Deserialize)]
struct VoxelAssetFile {
    format_version: u32,
    id: String,
    name: String,
    voxel_size: f32,
    dimensions: [i32; 3],
    #[serde(default)]
    pivot: Option<[f32; 3]>,
    palette: HashMap<String, String>,
    layers: Vec<Vec<String>>,
}

fn load_imported_assets() -> (Vec<PreviewAsset>, Vec<String>) {
    let Some(directory) = asset_directory() else {
        return (Vec::new(), Vec::new());
    };
    let mut files = match fs::read_dir(&directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>(),
        Err(error) => {
            return (
                Vec::new(),
                vec![format!("{}: {}", directory.display(), error)],
            )
        }
    };
    files.sort();
    let mut assets = Vec::new();
    let mut errors = Vec::new();
    let mut ids = HashMap::new();
    for path in files {
        match load_asset_file(&path) {
            Ok((id, asset)) => {
                if ids.insert(id.clone(), path.clone()).is_some() {
                    errors.push(format!("{}: duplicate asset id '{}'", path.display(), id));
                } else {
                    assets.push(asset);
                }
            }
            Err(error) => errors.push(format!("{}: {}", path.display(), error)),
        }
    }
    (assets, errors)
}

fn asset_directory() -> Option<std::path::PathBuf> {
    let candidates = [
        std::env::current_dir()
            .ok()
            .map(|path| path.join("assets/voxel-assets")),
        Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/voxel-assets")),
    ];
    candidates.into_iter().flatten().find(|path| path.is_dir())
}

fn load_asset_file(path: &Path) -> Result<(String, PreviewAsset), String> {
    const MAX_DIMENSION: i32 = 128;
    const MAX_VOXELS: usize = 262_144;
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    if text.len() > 8 * 1024 * 1024 {
        return Err("file exceeds 8 MiB limit".into());
    }
    let file: VoxelAssetFile = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if file.format_version != 1 {
        return Err("format_version must be 1".into());
    }
    if file.id.trim().is_empty() || file.name.trim().is_empty() {
        return Err("id and name are required".into());
    }
    if ![1.0, 0.5, 0.25, 0.125].contains(&file.voxel_size) {
        return Err("voxel_size must be 1, 0.5, 0.25, or 0.125".into());
    }
    if file
        .dimensions
        .iter()
        .any(|&dimension| !(1..=MAX_DIMENSION).contains(&dimension))
    {
        return Err("dimensions must be between 1 and 128".into());
    }
    let pivot = file.pivot.unwrap_or([
        file.dimensions[0] as f32 / 2.0,
        0.0,
        file.dimensions[2] as f32 / 2.0,
    ]);
    if pivot.iter().any(|value| !value.is_finite()) {
        return Err("pivot must contain finite numbers".into());
    }
    let voxel_count =
        file.dimensions[0] as usize * file.dimensions[1] as usize * file.dimensions[2] as usize;
    if voxel_count > MAX_VOXELS {
        return Err("asset exceeds 262144 voxels".into());
    }
    if file.layers.len() != file.dimensions[1] as usize {
        return Err("layers count must equal dimensions[1]".into());
    }
    if file.palette.is_empty() || file.palette.len() > 64 {
        return Err("palette must contain 1 to 64 entries".into());
    }
    let mut palette = HashMap::new();
    for (symbol, color) in &file.palette {
        let mut chars = symbol.chars();
        let Some(symbol) = chars.next() else {
            return Err("palette symbols must be one alphanumeric character".into());
        };
        if chars.next().is_some() || !symbol.is_ascii_alphanumeric() {
            return Err("palette symbols must be one alphanumeric character".into());
        }
        palette.insert(symbol, parse_asset_hex_color(color)?);
    }
    let mut world = VoxelWorld::new();
    let mut filled = 0;
    for (y, layer) in file.layers.iter().enumerate() {
        if layer.len() != file.dimensions[2] as usize {
            return Err(format!("layer {} row count is incorrect", y));
        }
        for (z, row) in layer.iter().enumerate() {
            if row.chars().count() != file.dimensions[0] as usize {
                return Err(format!("layer {} row {} width is incorrect", y, z));
            }
            for (x, symbol) in row.chars().enumerate() {
                if symbol == '.' {
                    continue;
                }
                let color = *palette
                    .get(&symbol)
                    .ok_or_else(|| format!("undefined palette symbol '{}'", symbol))?;
                world.set(
                    VoxelCoord::new(
                        (x as f32 - pivot[0]).round() as i32,
                        (y as f32 - pivot[1]).round() as i32,
                        (z as f32 - pivot[2]).round() as i32,
                    ),
                    VoxelCell::new(VoxelMaterial::Custom(color)),
                );
                filled += 1;
            }
        }
    }
    if filled == 0 {
        return Err("asset must contain at least one voxel".into());
    }
    let (center, radius) = asset_bounds(&world);
    let asset = PreviewAsset {
        name: file.name,
        world,
        center,
        radius,
        voxel_size: file.voxel_size,
        dimensions: file.dimensions,
        source: AssetSource::Imported,
    };
    Ok((file.id, asset))
}

fn parse_asset_hex_color(value: &str) -> Result<[u8; 3], String> {
    if value.len() != 7 || !value.starts_with('#') {
        return Err(format!("invalid color '{}'", value));
    }
    Ok([
        u8::from_str_radix(&value[1..3], 16).map_err(|_| format!("invalid color '{}'", value))?,
        u8::from_str_radix(&value[3..5], 16).map_err(|_| format!("invalid color '{}'", value))?,
        u8::from_str_radix(&value[5..7], 16).map_err(|_| format!("invalid color '{}'", value))?,
    ])
}

fn asset_dimensions(world: &VoxelWorld) -> [i32; 3] {
    world
        .bounds()
        .map(|bounds| {
            [
                bounds.max.x - bounds.min.x + 1,
                bounds.max.y - bounds.min.y + 1,
                bounds.max.z - bounds.min.z + 1,
            ]
        })
        .unwrap_or([0; 3])
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

fn map_viewer_default_distance(map: &PreviewMap) -> f32 {
    (map.radius * 2.55).max(32.0)
}

fn map_viewer_start_camera(target: Vec3, radius: f32, distance: f32) -> Camera {
    let pitch = 0.62_f32;
    look_at(
        target + Vec3::new(0.0, pitch.sin() * distance, -pitch.cos() * distance),
        target,
    )
    .with_fov_y(48.0_f32.to_radians())
    .with_max_distance((distance + radius * 3.0).max(80.0))
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
        DroneGateTarget {
            position: Vec3::new(0.0, DRONE_GATE_FRAME_RADIUS as f32 + 2.0, 0.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
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
            stamp_enemy(&mut world, enemy);
        }
    }
    world
}

fn zombies_world_with_zombies(base: &VoxelWorld, zombies: &ZombiesState) -> VoxelWorld {
    let mut world = base.clone();
    for zombie in &zombies.zombies {
        if zombie.is_alive() {
            stamp_enemy(&mut world, zombie);
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

fn stamp_enemy(world: &mut VoxelWorld, enemy: &Enemy) {
    match enemy.enemy_type {
        EnemyType::Clown => stamp_npc_body(
            world,
            enemy.position,
            VoxelMaterial::SiliconLife,
            VoxelMaterial::Beacon,
        ),
        EnemyType::Zombie => {
            let wounded = enemy.health * 2 < enemy.max_health;
            let accent = if wounded {
                VoxelMaterial::Beacon
            } else {
                VoxelMaterial::CarbonLife
            };
            stamp_zombie_body(world, enemy.position, accent);
        }
    }
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

fn enemy_body_contains_voxel(enemy_type: EnemyType, position: Vec3, coord: VoxelCoord) -> bool {
    match enemy_type {
        EnemyType::Clown => npc_body_contains_voxel(position, coord),
        EnemyType::Zombie => zombie_body_contains_voxel(position, coord),
    }
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
        y: 77,
        z: 10,
        text: "E  ECHOLOCATION".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 81,
        z: 10,
        text: "V  MAP VIEWER".to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 48,
        y: 86,
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
            "ASSET VIEWER  {} / {}  {}  {:.3}m voxels  {:.1}x{:.1}x{:.1}m  {}  zoom {:.1}  mouse {}  M menu",
            viewer.selected + 1,
            viewer.assets.len(),
            asset.name,
            asset.voxel_size,
            asset.dimensions[0] as f32 * asset.voxel_size,
            asset.dimensions[1] as f32 * asset.voxel_size,
            asset.dimensions[2] as f32 * asset.voxel_size,
            match asset.source { AssetSource::BuiltIn => "built-in", AssetSource::Imported => "imported" },
            viewer.distance,
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
    for (index, error) in viewer.load_errors.iter().take(3).enumerate() {
        scene.overlays.push(Overlay {
            x: 2,
            y: 6 + index as i32,
            z: 120,
            text: format!("ASSET ERROR: {}", error),
            style: TextStyle::default(),
        });
    }
    scene.overlays.push(Overlay {
        x: 2,
        y: 10 + viewer.load_errors.len().min(3) as i32,
        z: 120,
        text: "1-9 select  N/P cycle  A/D yaw  W/S pitch  Q/E roll  Space/Ctrl zoom".to_string(),
        style: TextStyle::default(),
    });
    for (index, preview) in viewer.assets.iter().enumerate() {
        let marker = if index == viewer.selected { '>' } else { ' ' };
        scene.overlays.push(Overlay {
            x: 2,
            y: 14 + viewer.load_errors.len().min(3) as i32 + index as i32,
            z: 120,
            text: format!("{}{} {}", marker, index + 1, preview.name),
            style: TextStyle::default(),
        });
    }
}

fn render_map_viewer_scene(scene: &mut Scene, viewer: &MapViewerState, mouse_captured: bool) {
    let map = viewer.selected_map();
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "MAP VIEWER  {} / {}  {}  {}x{}x{} voxels  {} filled  {}  zoom {:.1}  mouse {}  M menu",
            viewer.selected + 1,
            viewer.maps.len(),
            map.name,
            map.dimensions[0],
            map.dimensions[1],
            map.dimensions[2],
            map.world.voxel_count(),
            map.definition,
            viewer.distance,
            if mouse_captured { "locked" } else { "free" }
        ),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: match viewer.view {
            MapViewerView::FreeFlight => {
                "FREE FLIGHT  WASD move  Space/Ctrl rise/drop  Shift boost  mouse look  O orbit"
            }
            MapViewerView::Orbit => {
                "ORBIT  WASD/arrows pan  mouse orbit  Q/E roll  Space/Ctrl zoom  O free flight"
            }
        }
        .to_string(),
        style: TextStyle::default(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 7,
        z: 120,
        text: format!(
            "C ceilings {}  R reset  planet flight omitted: analytic terrain, not a finite voxel map",
            if viewer.ceilings_hidden { "hidden" } else { "shown" },
        ),
        style: TextStyle::default(),
    });
    for (index, preview) in viewer.maps.iter().enumerate() {
        let marker = if index == viewer.selected { '>' } else { ' ' };
        scene.overlays.push(Overlay {
            x: 2,
            y: 11 + index as i32,
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
        VoxelMaterial::Receiver => "receiver",
        VoxelMaterial::SignalPipe => "signal pipe",
        VoxelMaterial::PuzzleDoor => "puzzle door",
        VoxelMaterial::Custom(_) => "asset",
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
            "DRONE GATE RUNNER  map {}  seed {:016x}  next gate {}  mouse {}",
            runner.course.name,
            runner.course.seed,
            runner.active_gate + 1,
            if mouse_captured { "locked" } else { "free" }
        ),
        style: hud_style(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: format!(
            "passed {}  generated {}  time {:05.1}  speed {:03.0}  target {:.0},{:.0},{:.0}",
            runner.passed_gates,
            runner.course.gates.len(),
            runner.elapsed,
            runner.velocity.length(),
            active.x,
            active.y,
            active.z
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

fn render_echolocation_scene(
    scene: &mut Scene,
    echo: &EchoLocationState,
    camera: &Camera,
    mouse_captured: bool,
) {
    let search_effect = echo_search_effect(echo, camera.position);
    let ping_status = if echo.ping_cooldown_remaining > 0.0 {
        format!("{:.1}s", echo.ping_cooldown_remaining)
    } else {
        "READY".to_string()
    };
    let charge_status = echo
        .pulse_charge_seconds
        .map(|charge| {
            format!(
                "{:>3}%",
                (charge / ECHO_CHARGED_PULSE_SECONDS * 100.0) as u8
            )
        })
        .unwrap_or_else(|| "idle".to_string());
    scene.layers.push(Layer {
        name: "reticle".to_string(),
        z: 50,
        cells: reticle_cells_offset(scene.viewport, search_effect.reticle_offset),
    });
    if search_effect.intensity > 0.0 {
        scene.layers.push(Layer {
            name: "search-twitch".to_string(),
            z: 49,
            cells: echo_search_twitch_cells(scene.viewport, search_effect),
        });
        scene.layers.push(Layer {
            name: "search-static".to_string(),
            z: 85,
            cells: echo_search_static_cells(scene.viewport, search_effect),
        });
    }
    scene.layers.push(Layer {
        name: "footprints".to_string(),
        z: 70,
        cells: echo_footprint_cells(scene.viewport, camera, &echo.world, &echo.footprints),
    });
    scene.layers.push(Layer {
        name: "step-waves".to_string(),
        z: 71,
        cells: echo_step_wave_cells(scene.viewport, camera, &echo.world, &echo.step_waves),
    });
    scene.layers.push(Layer {
        name: "receiver-signal".to_string(),
        z: 72,
        cells: echo_receiver_signal_cells(scene.viewport, camera, echo),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 2,
        z: 120,
        text: format!(
            "ECHOLOCATION  seed {:016x}  pulses {} ({} reflected)  mouse {}  M menu",
            echo.seed,
            echo.waves.len(),
            echo.reflected_pulse_count(),
            if mouse_captured { "locked" } else { "free" }
        ),
        style: hud_style(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 5,
        z: 120,
        text: format!(
            "Space jump  hold/click mouse pulse [{}; charge {}]  V full map [{}]  speed {:.0}  range {:.0}  TAB tuning [{}]",
            ping_status,
            charge_status,
            if echo.show_full_map { "ON" } else { "off" },
            echo.config.ping_speed,
            echo.config.max_range,
            if echo.tuning_open { "open" } else { "closed" },
        ),
        style: hud_style(),
    });
    scene.overlays.push(Overlay {
        x: 2,
        y: 8,
        z: 120,
        text: format!(
            "Your steps make tiny echoes. Something follows: fading prints and footsteps reveal it.{}",
            if search_effect.intensity > 0.0 { "  IT HEARD YOU — SEARCHING" } else { "" }
        ),
        style: hud_style(),
    });
    if search_effect.intensity > 0.0 {
        scene.overlays.push(Overlay {
            x: 2 + search_effect.hud_offset,
            y: 11,
            z: 121,
            text: if search_effect.corrupt_glyphs {
                "IT H?ARD YOU — S?ARCHING".to_string()
            } else {
                "IT HEARD YOU — SEARCHING".to_string()
            },
            style: TextStyle {
                fg: Some("#b8b8b8".to_string()),
                ..hud_style()
            },
        });
    }
    if echo.tuning_open {
        scene.overlays.push(Overlay {
            x: 2,
            y: 11,
            z: 120,
            text: format!(
                "[ ] range {:.0}   - = speed {:.0}   , . echo strength {:.2}",
                echo.config.max_range, echo.config.ping_speed, echo.config.echo_strength,
            ),
            style: hud_style(),
        });
        scene.overlays.push(Overlay {
            x: 2,
            y: 14,
            z: 120,
            text: format!(
                "all surfaces return equally; bounce depth is derived from strength  R defaults"
            ),
            style: hud_style(),
        });
    }
    if echo.run_status == EchoRunStatus::Dead {
        let style = TextStyle {
            fg: Some("#ff4d5a".to_string()),
            bg: Some("#080b10".to_string()),
            ..TextStyle::default()
        };
        scene.overlays.push(Overlay {
            x: scene.viewport.width as i32 / 2 - 8,
            y: scene.viewport.height as i32 / 2 - 2,
            z: 250,
            text: "YOU WERE FOUND".to_string(),
            style: style.clone(),
        });
        scene.overlays.push(Overlay {
            x: scene.viewport.width as i32 / 2 - 9,
            y: scene.viewport.height as i32 / 2 + 2,
            z: 250,
            text: "R restart   M menu".to_string(),
            style,
        });
    }
}

fn echo_receiver_signal_cells(
    viewport: Viewport,
    camera: &Camera,
    echo: &EchoLocationState,
) -> Vec<SceneCell> {
    let mut cells = Vec::new();
    let active: Vec<_> = echo
        .puzzle
        .pipe
        .iter()
        .map(|point| {
            echo.puzzle
                .signal_active_at(point.distance, echo.puzzle.time)
        })
        .collect();
    for (index, point) in echo.puzzle.pipe.iter().enumerate().skip(1) {
        if !active[index]
            || !has_line_of_sight(
                &echo.world,
                camera.position,
                point.position + Vec3::new(0.0, 0.04, 0.0),
            )
        {
            continue;
        }
        let trailing_edge = !active[index.saturating_sub(1)];
        let leading_edge = !active.get(index + 1).copied().unwrap_or(false);
        let edge = trailing_edge || leading_edge;
        let glyph = match (trailing_edge, leading_edge) {
            (true, true) => '*',
            (true, false) => '>',
            (false, true) => '>',
            (false, false) => '=',
        };
        if let Some(projection) = project_world_point(camera, point.position, viewport) {
            let progress = point.distance / echo.puzzle.endpoint_distance().max(1.0);
            cells.push(SceneCell {
                x: projection.x,
                y: projection.y,
                glyph,
                style: TextStyle {
                    fg: Some(if edge {
                        "#7deaff".to_string()
                    } else {
                        format!(
                            "#{:02x}{:02x}{:02x}",
                            lerp_channel(0x18, 0x28, progress),
                            lerp_channel(0x8f, 0xc8, progress),
                            lerp_channel(0xd8, 0xff, progress),
                        )
                    }),
                    bold: edge,
                    ..TextStyle::default()
                },
            });
        }
    }

    if echo.puzzle.receiver_active() {
        let position = echo_receiver_sound_position(echo.puzzle.receiver.coord);
        if has_line_of_sight(&echo.world, camera.position, position) {
            if let Some(projection) = project_world_point(camera, position, viewport) {
                cells.push(SceneCell {
                    x: projection.x,
                    y: projection.y,
                    glyph: '@',
                    style: TextStyle {
                        fg: Some("#79efff".to_string()),
                        bold: true,
                        ..TextStyle::default()
                    },
                });
            }
        }
    }
    if echo.puzzle.door_powered() {
        let position = echo
            .puzzle
            .pipe
            .last()
            .map(|point| point.position)
            .unwrap_or_else(echo_door_sound_position);
        if has_line_of_sight(&echo.world, camera.position, position) {
            if let Some(projection) = project_world_point(camera, position, viewport) {
                cells.push(SceneCell {
                    x: projection.x,
                    y: projection.y,
                    glyph: '#',
                    style: TextStyle {
                        fg: Some("#93f5ff".to_string()),
                        bold: true,
                        ..TextStyle::default()
                    },
                });
            }
        }
    }
    cells
}

fn echo_footprint_cells(
    viewport: Viewport,
    camera: &Camera,
    world: &VoxelWorld,
    footprints: &[EchoFootprint],
) -> Vec<SceneCell> {
    footprints
        .iter()
        .filter_map(|print| {
            // Decals sit just above the floor's top surface; aim slightly above
            // that point so the supporting floor voxel is not self-occluding.
            let sight_target = print.position + Vec3::new(0.0, 0.04, 0.0);
            if !has_line_of_sight(world, camera.position, sight_target) {
                return None;
            }
            let (glyph, color) = echo_footprint_visual(*print);
            project_world_point(camera, print.position, viewport).map(|projection| SceneCell {
                x: projection.x,
                y: projection.y,
                // Pair spacing carries the travel direction; these larger glyphs
                // remain visible at the deliberately low terminal resolution.
                glyph,
                style: TextStyle {
                    fg: Some(color),
                    ..TextStyle::default()
                },
            })
        })
        .collect()
}

fn echo_step_wave_cells(
    viewport: Viewport,
    camera: &Camera,
    world: &VoxelWorld,
    waves: &[EchoStepWave],
) -> Vec<SceneCell> {
    let mut cells = Vec::new();
    for wave in waves {
        let radius = wave.age * ECHO_STEP_WAVE_SPEED;
        let strength = (1.0 - radius / ECHO_STEP_WAVE_MAX_RADIUS).clamp(0.0, 1.0);
        let color = format!(
            "#{:02x}{:02x}{:02x}",
            lerp_channel(0x3c, 0x9f, strength),
            lerp_channel(0x58, 0xe9, strength),
            lerp_channel(0x65, 0xff, strength),
        );
        if radius < 0.45
            && has_line_of_sight(
                world,
                camera.position,
                wave.origin + Vec3::new(0.0, 0.04, 0.0),
            )
        {
            if let Some(projection) = project_world_point(camera, wave.origin, viewport) {
                cells.push(SceneCell {
                    x: projection.x,
                    y: projection.y,
                    glyph: '*',
                    style: TextStyle {
                        fg: Some(color.clone()),
                        ..TextStyle::default()
                    },
                });
            }
        }
        for impact in &wave.impacts {
            let arrival = impact.arrival_distance_milli as f32 / 1000.0;
            if (arrival - radius).abs() > 0.34 {
                continue;
            }
            let point = Vec3::new(
                impact.solid_voxel.x as f32 + 0.5,
                impact.solid_voxel.y as f32 + 0.5,
                impact.solid_voxel.z as f32 + 0.5,
            );
            if !has_line_of_sight_to_voxel(world, camera.position, point, impact.solid_voxel) {
                continue;
            }
            if let Some(projection) = project_world_point(camera, point, viewport) {
                cells.push(SceneCell {
                    x: projection.x,
                    y: projection.y,
                    glyph: '~',
                    style: TextStyle {
                        fg: Some(color.clone()),
                        ..TextStyle::default()
                    },
                });
            }
        }
    }
    cells
}

fn echo_footprint_visual(print: EchoFootprint) -> (char, String) {
    let strength = (print.remaining_seconds / ECHO_FOOTPRINT_LIFETIME).clamp(0.0, 1.0);
    let (r, g, b) = (
        lerp_channel(0x50, 0xd5, strength),
        lerp_channel(0x3d, 0xa7, strength),
        lerp_channel(0x2b, 0x5a, strength),
    );
    let glyph = if strength < 0.22 {
        '.'
    } else if print.left && strength > 0.60 {
        'O'
    } else {
        'o'
    };
    (glyph, format!("#{r:02x}{g:02x}{b:02x}"))
}

fn lerp_channel(dim: u8, bright: u8, strength: f32) -> u8 {
    (dim as f32 + (bright as f32 - dim as f32) * strength).round() as u8
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
    reticle_cells_offset(viewport, (0, 0))
}

fn reticle_cells_offset(viewport: Viewport, offset: (i32, i32)) -> Vec<SceneCell> {
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
        x: center_x + x + offset.0,
        y: center_y + y + offset.1,
        glyph,
        style: TextStyle::default(),
    })
    .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoSearchEffect {
    intensity: f32,
    corruption_level: u8,
    reticle_offset: (i32, i32),
    hud_offset: i32,
    corrupt_glyphs: bool,
    phase: u32,
}

fn echo_search_effect(echo: &EchoLocationState, listener_position: Vec3) -> EchoSearchEffect {
    let EchoPursuerMode::Investigate {
        remaining_seconds, ..
    } = echo.pursuer.mode
    else {
        return EchoSearchEffect {
            intensity: 0.0,
            corruption_level: 0,
            reticle_offset: (0, 0),
            hud_offset: 0,
            corrupt_glyphs: false,
            phase: 0,
        };
    };
    let distance = horizontal_distance(echo.pursuer.position, listener_position);
    let proximity = (1.0 - distance / 36.0).clamp(0.0, 1.0);
    // Searching is the on/off gate. Distance chooses a deliberately steep
    // corruption scale so a nearby pursuer is unmistakably invasive.
    let corruption_level = match distance {
        d if d <= 4.0 => 5,
        d if d <= 9.0 => 4,
        d if d <= 15.0 => 3,
        d if d <= 30.0 => 2,
        _ => 1,
    };
    let intensity = 0.12 + proximity * 0.88;
    let phase = ((ECHO_PURSUER_INVESTIGATE_SECONDS - remaining_seconds).max(0.0) * 17.0) as u32
        ^ (echo.seed as u32);
    let twitch = if corruption_level >= 2
        && phase % (12_u32.saturating_sub((intensity * 5.0) as u32).max(3)) == 0
    {
        1
    } else {
        0
    };
    EchoSearchEffect {
        intensity,
        corruption_level,
        reticle_offset: (
            twitch * ((phase >> 1 & 1) as i32 * 2 - 1),
            twitch * ((phase >> 2 & 1) as i32 * 2 - 1),
        ),
        hud_offset: twitch * ((phase & 1) as i32 * 2 - 1),
        corrupt_glyphs: corruption_level >= 3
            && phase % (22 - corruption_level as u32 * 3).max(4) == 0
            && intensity > 0.32,
        phase,
    }
}

fn echo_search_twitch_cells(viewport: Viewport, effect: EchoSearchEffect) -> Vec<SceneCell> {
    let count = 2 + effect.corruption_level as i32 * 7;
    (0..count)
        .filter_map(|index| {
            let hash = effect
                .phase
                .wrapping_mul(1_103_515_245)
                .wrapping_add(index as u32 * 12_345);
            let edge = hash % 4;
            let (x, y) = match edge {
                0 => ((hash % viewport.width as u32) as i32, 0),
                1 => (
                    (hash % viewport.width as u32) as i32,
                    viewport.height as i32 - 1,
                ),
                2 => (0, (hash % viewport.height as u32) as i32),
                _ => (
                    viewport.width as i32 - 1,
                    (hash % viewport.height as u32) as i32,
                ),
            };
            (hash % 4 == 0 || effect.corruption_level >= 4 && effect.intensity > 0.72).then(|| {
                SceneCell {
                    x,
                    y,
                    glyph: if hash & 1 == 0 { ':' } else { '.' },
                    style: TextStyle {
                        fg: Some(
                            if effect.corruption_level == 1 {
                                "#25282b"
                            } else {
                                "#62686d"
                            }
                            .to_string(),
                        ),
                        ..TextStyle::default()
                    },
                }
            })
        })
        .collect()
}

/// Render-only glyph replacement. This intentionally sits above world layers:
/// the closer the searching pursuer is, the more of the ASCII image becomes
/// unreliable, without changing the simulation camera or player controls.
fn echo_search_static_cells(viewport: Viewport, effect: EchoSearchEffect) -> Vec<SceneCell> {
    if effect.corruption_level <= 1 {
        return Vec::new();
    }
    let coverage = match effect.corruption_level {
        2 => 0.003,
        3 => 0.016,
        4 => 0.075,
        _ => 0.16,
    };
    let cell_count = (viewport.width * viewport.height) as f32;
    let noise_count = (cell_count * coverage) as u32;
    let glyphs = [
        '.', ':', ';', '*', '#', '%', '@', '?', '/', '\\', '|', '+', '=',
    ];
    let mut cells = Vec::with_capacity(noise_count as usize + effect.corruption_level as usize * 8);
    for index in 0..noise_count {
        let hash = echo_static_hash(effect.phase, index);
        cells.push(SceneCell {
            x: (hash % viewport.width as u32) as i32,
            y: ((hash >> 9) % viewport.height as u32) as i32,
            glyph: glyphs[(hash >> 18) as usize % glyphs.len()],
            style: TextStyle {
                fg: Some(
                    match ((hash >> 3) & 3, effect.corruption_level) {
                        (_, 2) => {
                            if hash & 1 == 0 {
                                "#17191b"
                            } else {
                                "#303438"
                            }
                        }
                        (0, _) => "#17191b",
                        (1, _) => "#303438",
                        (2, _) => "#5e6469",
                        _ => "#aeb3b5",
                    }
                    .to_string(),
                ),
                ..TextStyle::default()
            },
        });
    }
    // At the upper tiers, intermittent horizontal tears make the image feel
    // broken rather than merely speckled.
    for tear in 0..effect.corruption_level.saturating_sub(2) as u32 {
        let hash = echo_static_hash(effect.phase ^ 0xD15E_A5E5, tear);
        let y = (hash % viewport.height as u32) as i32;
        let start = ((hash >> 10) % viewport.width as u32) as i32;
        let width = 6 + ((hash >> 19) % (effect.corruption_level as u32 * 7)) as i32;
        for x in start..(start + width).min(viewport.width as i32) {
            cells.push(SceneCell {
                x,
                y,
                glyph: if (x + y) & 1 == 0 { '=' } else { '-' },
                style: TextStyle {
                    fg: Some("#8b9093".to_string()),
                    ..TextStyle::default()
                },
            });
        }
    }
    cells
}

fn echo_static_hash(phase: u32, index: u32) -> u32 {
    let mut value = phase
        .wrapping_add(index.wrapping_mul(0x9E37_79B9))
        .wrapping_add(0x7F4A_7C15);
    value ^= value >> 16;
    value = value.wrapping_mul(0x85EB_CA6B);
    value ^= value >> 13;
    value.wrapping_mul(0xC2B2_AE35) ^ (value >> 16)
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
            if opaque_cells {
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
    fn shared_jump_rises_from_and_lands_on_voxel_ground() {
        let mut world = VoxelWorld::new();
        fill_cuboid(
            &mut world,
            VoxelCoord::new(-2, 0, -2),
            VoxelCoord::new(2, 0, 2),
            VoxelMaterial::Stone,
        );
        let mut camera = Camera::new(Vec3::new(0.5, WALK_EYE_HEIGHT, 0.5));
        let mut input = PlayerInput {
            jump_requested: true,
            ..PlayerInput::default()
        };
        let mut motion = WalkMotion::default();

        update_jumping_walking_camera(
            &mut camera,
            &mut input,
            &mut motion,
            &world,
            STANDARD_WALK_PROFILE,
            0.05,
        );
        assert!(camera.position.y > WALK_EYE_HEIGHT);

        for _ in 0..40 {
            update_jumping_walking_camera(
                &mut camera,
                &mut input,
                &mut motion,
                &world,
                STANDARD_WALK_PROFILE,
                0.05,
            );
        }
        assert_eq!(camera.position.y, WALK_EYE_HEIGHT);
        assert!(!motion.airborne);
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
    fn enemy_types_supply_distinct_extensible_profiles() {
        let clown = EnemyType::Clown.profile();
        let zombie = EnemyType::Zombie.profile();

        assert_eq!(EnemyType::Clown.label(), "clown");
        assert_eq!(EnemyType::Zombie.label(), "zombie");
        assert_ne!(clown.eye_height, zombie.eye_height);
        assert_ne!(clown.speed, zombie.speed);
        assert_ne!(clown.base_health, zombie.base_health);
    }

    #[test]
    fn enemy_instances_share_health_and_type_specific_hitboxes() {
        let clown = Enemy::new(EnemyType::Clown, Vec3::ZERO, 1);
        let zombie = Enemy::new(EnemyType::Zombie, Vec3::ZERO, 2);

        assert!(clown.is_alive());
        assert!(zombie.is_alive());
        assert!(clown.contains_voxel(VoxelCoord::new(0, 2, 0)));
        assert!(zombie.contains_voxel(VoxelCoord::new(0, 5, 0)));
        assert!(!clown.contains_voxel(VoxelCoord::new(0, 5, 0)));
        assert!(zombie.max_health > Enemy::new(EnemyType::Zombie, Vec3::ZERO, 1).max_health);
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
    fn map_catalog_covers_every_finite_voxel_gameplay_map() {
        let maps = build_map_catalog();

        assert_eq!(maps.len(), 9);
        assert_eq!(
            maps.iter().map(|map| map.name.as_str()).collect::<Vec<_>>(),
            vec![
                "procedural city",
                "doomlike arena",
                "corn maze",
                "Starhusk bar",
                "voxel sandbox",
                "Heliobound Zombies",
                "liminal office",
                "drone gate course",
                "echolocation",
            ]
        );
        assert!(maps.iter().all(|map| map.world.voxel_count() > 0));
        assert!(maps
            .iter()
            .all(|map| map.dimensions.iter().all(|size| *size > 0)));
    }

    #[test]
    fn map_viewer_defaults_to_collision_free_sandbox_style_flight() {
        let map = PreviewMap::new(
            "test map",
            build_block_asset(VoxelMaterial::Stone),
            Camera::new(Vec3::new(1.0, 2.0, 3.0)),
            "test",
        );
        let center = map.center;
        let mut viewer = MapViewerState::new(vec![map]);
        let before = viewer.camera.position;
        let input = PlayerInput {
            right: true,
            ..PlayerInput::default()
        };

        viewer.update(&input, 1.0);

        assert_eq!(viewer.view, MapViewerView::FreeFlight);
        assert!(viewer.camera.max_distance.is_infinite());
        assert!(
            (horizontal_distance(viewer.camera.position, before)
                - SANDBOX_SPEED * MAP_VIEWER_FREE_SPEED_MULTIPLIER)
                .abs()
                < 0.001
        );
        assert_eq!(viewer.target, center);

        viewer.reset_view();
        assert_eq!(viewer.target, center);
    }

    #[test]
    fn map_viewer_uses_each_mode_start_camera_when_entering_or_resetting() {
        let maps = build_map_catalog();
        let city_start = map_viewer_camera(maps[0].start_camera);
        let echolocation_start = map_viewer_camera(maps[8].start_camera);
        let mut viewer = MapViewerState::new(maps);

        assert_eq!(viewer.camera(), city_start);

        viewer.select(8);
        assert_eq!(viewer.camera(), echolocation_start);
        viewer.camera.position = Vec3::ZERO;
        viewer.reset_view();
        assert_eq!(viewer.camera(), echolocation_start);
    }

    #[test]
    fn map_viewer_pan_tracks_the_current_camera_basis() {
        let map = PreviewMap::new(
            "test map",
            build_block_asset(VoxelMaterial::Stone),
            Camera::new(Vec3::new(1.0, 2.0, 3.0)),
            "test",
        );
        let mut viewer = MapViewerState::new(vec![map]);
        viewer.toggle_view();
        viewer
            .camera
            .rotate_local_yaw_pitch(std::f32::consts::PI, 0.0);
        viewer.sync_camera_position();
        let expected_left = horizontal(viewer.camera.right()).normalized() * -1.0;
        let before = viewer.target;
        let input = PlayerInput {
            pan_left: true,
            ..PlayerInput::default()
        };

        viewer.update(&input, 1.0);

        assert!(
            horizontal(viewer.target - before)
                .normalized()
                .dot(expected_left)
                > 0.999
        );
    }

    #[test]
    fn map_viewer_ceiling_toggle_removes_dense_overhead_layers_only() {
        let mut world = VoxelWorld::new();
        fill_cuboid(
            &mut world,
            VoxelCoord::new(0, 0, 0),
            VoxelCoord::new(2, 0, 2),
            VoxelMaterial::Basalt,
        );
        fill_cuboid(
            &mut world,
            VoxelCoord::new(0, 4, 0),
            VoxelCoord::new(2, 4, 2),
            VoxelMaterial::ShipHull,
        );
        world.set(
            VoxelCoord::new(1, 5, 1),
            VoxelCell::new(VoxelMaterial::Beacon),
        );

        let ceilingless = without_map_ceilings(&world);

        assert_eq!(ceilingless.get(VoxelCoord::new(1, 4, 1)), None);
        assert!(ceilingless.get(VoxelCoord::new(1, 0, 1)).is_some());
        assert!(ceilingless.get(VoxelCoord::new(1, 5, 1)).is_some());
    }

    #[test]
    fn menu_can_start_and_leave_map_viewer_mode() {
        let mut app = AppState::new();

        let action = app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyV), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::MapViewer);
        assert_eq!(app.map_viewer.as_ref().unwrap().maps.len(), 9);

        app.enter_menu();
        assert_eq!(app.mode, AppMode::Menu);
        assert!(app.map_viewer.is_none());
    }

    #[test]
    fn map_viewer_scene_draws_selected_map_and_controls() {
        let mut app = AppState::new();
        app.start_map_viewer();

        let scene = app.frame(0.0, true);

        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "voxels" && !layer.cells.is_empty()));
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("MAP VIEWER")));
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("FREE FLIGHT")));
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
    fn voxel_sandbox_m_returns_to_menu() {
        let mut app = AppState::new();
        app.start_voxel_sandbox();

        let action = app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyM), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::EnterMenu);
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
        let mut app = AppState::new_with_drone_course_nonce(0xD00D, false);

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit0), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::DroneGateRunner);
        assert_eq!(
            app.drone_gate_runner.course.seed,
            drone_course_seed(0xD00D, 1)
        );
        assert!(app.camera.max_distance >= DRONE_GATE_VIEW_DISTANCE);
    }

    #[test]
    fn drone_gate_runner_menu_selection_reaches_the_render_path() {
        let mut app = AppState::new_with_drone_course_nonce(0xD00D, false);

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit0), ElementState::Pressed);
        let scene = app.frame(1.0 / 60.0, false);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::DroneGateRunner);
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| { overlay.text.starts_with("DRONE GATE RUNNER") }));
    }

    #[test]
    fn drone_gate_runner_restarts_with_new_procedural_course() {
        let mut app = AppState::new_with_drone_course_nonce(0xD00D, false);

        app.start_drone_gate_runner();
        let first = app.drone_gate_runner.course.clone();
        app.enter_menu();
        app.start_drone_gate_runner();
        let second = app.drone_gate_runner.course.clone();

        assert_ne!(first.seed, second.seed);
        assert_ne!(first.gates, second.gates);
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
    fn drone_gate_course_uses_tunable_generation_baseline() {
        let config = DroneGateRunnerConfig::default();
        let course = generate_drone_gate_course(1234, config);

        assert_eq!(course.gates.len(), config.course.gate_count);
        for pair in course.gates.windows(2) {
            let spacing = (pair[1].position - pair[0].position).length();
            assert!(spacing > config.spacing * 0.70);
            assert!(spacing < config.spacing * 2.50);
            assert!((pair[0].normal.length() - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn drone_gate_course_does_not_use_fixed_z_spine() {
        let config = DroneGateRunnerConfig::default();
        let course = generate_drone_gate_course(1234, config);

        assert!(course.gates.windows(2).any(|pair| {
            let step = pair[1].position - pair[0].position;
            (step.z - config.spacing).abs() > 1.0 || step.x.abs() > 1.0
        }));
    }

    #[test]
    fn drone_runner_advances_when_player_flies_through_active_gate() {
        let mut runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let gate = runner.course.gates[runner.active_gate];
        runner.previous_position = gate.position - gate.normal * 12.0;

        runner.update(gate.position + gate.normal * 12.0, 0.016);

        assert_eq!(runner.active_gate, 1);
        assert_eq!(runner.passed_gates, 1);
    }

    #[test]
    fn drone_runner_extends_course_instead_of_lapping() {
        let mut runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let original_len = runner.course.gates.len();
        runner.active_gate = original_len - 1;

        runner.advance_gate();

        assert_eq!(runner.active_gate, original_len);
        assert!(runner.course.gates.len() > original_len);
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
    fn drone_runner_accelerates_smoothly_toward_speed_cap() {
        let mut runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let mut camera = drone_gate_runner_start_camera(&runner);
        let input = PlayerInput {
            forward: true,
            ..PlayerInput::default()
        };

        runner.update_camera(&mut camera, &input, 0.1);
        let first_speed = runner.velocity.length();
        runner.update_camera(&mut camera, &input, 0.1);
        let second_speed = runner.velocity.length();

        assert!(first_speed > 0.0);
        assert!(second_speed > first_speed);
        assert!(second_speed < runner.config.flight.max_speed);
    }

    #[test]
    fn drone_runner_coasts_down_when_thrust_stops() {
        let mut runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let mut camera = drone_gate_runner_start_camera(&runner);
        let thrust = PlayerInput {
            forward: true,
            ..PlayerInput::default()
        };

        runner.update_camera(&mut camera, &thrust, 0.5);
        let powered_speed = runner.velocity.length();
        runner.update_camera(&mut camera, &PlayerInput::default(), 0.25);
        let coasting_speed = runner.velocity.length();

        assert!(powered_speed > 0.0);
        assert!(coasting_speed > 0.0);
        assert!(coasting_speed < powered_speed);
    }

    #[test]
    fn drone_runner_world_lights_active_gate() {
        let runner = DroneGateRunnerState::new_seeded(DRONE_GATE_SEED);
        let gate = runner.course.gates[runner.active_gate];
        let (right, _) = drone_gate_basis(gate.normal);
        let sample = gate.position + right * runner.config.gate_radius as f32;
        let world = runner.render_world();

        assert_eq!(
            world.get(VoxelCoord::new(
                sample.x.round() as i32,
                sample.y.round() as i32,
                sample.z.round() as i32
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
        app.zombies.zombies = vec![Enemy::new(EnemyType::Zombie, Vec3::new(0.5, 0.0, -38.5), 1)];
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
        assert!(
            zombies.zombies[0].max_health > Enemy::new(EnemyType::Zombie, Vec3::ZERO, 1).max_health
        );
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
        zombies.zombies = vec![Enemy::new(EnemyType::Zombie, Vec3::new(0.7, 0.0, -66.5), 1)];
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
        let nav = NavigationField::build(&world, player, zombie_walk_profile())
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
        let mut input = PlayerInput {
            forward: true,
            boost: true,
            ..Default::default()
        };

        zombies.update_player(
            &mut camera,
            &mut input,
            &mut WalkMotion::default(),
            &world,
            1.0,
        );

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
        let mut input = PlayerInput {
            forward: true,
            boost: true,
            ..Default::default()
        };

        zombies.update_player(
            &mut camera,
            &mut input,
            &mut WalkMotion::default(),
            &world,
            4.0,
        );
        assert!(zombies.sprint_locked);
        assert!(zombies.sprint < 1.0);

        zombies.update_player(
            &mut camera,
            &mut input,
            &mut WalkMotion::default(),
            &world,
            1.0,
        );

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
        app.zombies.zombies = vec![Enemy::new(EnemyType::Zombie, Vec3::new(0.5, 0.0, -38.5), 1)];
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

    #[test]
    fn echolocation_tuning_panel_changes_live_config_and_is_rendered() {
        let mut app = AppState::new();
        app.start_echolocation();
        let initial_range = app.echolocation.config.max_range;

        // Tuning keys are global to the mode, like M: opening the drawer is
        // only needed to inspect the values, not to change them.
        app.handle_keyboard(
            &PhysicalKey::Code(KeyCode::BracketRight),
            ElementState::Pressed,
        );
        assert!(app.echolocation.config.max_range > initial_range);
        assert_eq!(
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Tab), ElementState::Pressed),
            KeyboardAction::None
        );
        assert!(app.echolocation.tuning_open);

        let scene = app.frame(0.0, false);
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("echo strength")));

        app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyV), ElementState::Pressed);
        assert!(app.echolocation.show_full_map);
        let scene = app.frame(0.0, false);
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("V full map [ON]")));
    }

    #[test]
    fn echolocation_global_tuning_does_not_consume_movement_keys() {
        let mut app = AppState::new();
        app.start_echolocation();

        app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyW), ElementState::Pressed);
        assert!(app.input.forward);
        app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyW), ElementState::Released);
        assert!(!app.input.forward);
    }

    #[test]
    fn echolocation_ping_audio_respects_one_second_cooldown() {
        let mut app = AppState::new();
        app.start_echolocation();

        app.fire_weapon();
        assert_eq!(app.drain_audio_events(), vec![SoundEffect::EchoPing]);

        app.fire_weapon();
        assert!(app.drain_audio_events().is_empty());

        app.echolocation.update(0.99);
        app.fire_weapon();
        assert!(app.drain_audio_events().is_empty());

        app.echolocation.update(0.01);
        app.fire_weapon();
        assert_eq!(app.drain_audio_events(), vec![SoundEffect::EchoPing]);
    }

    #[test]
    fn echolocation_charged_pulse_reaches_a_pursuer_outside_normal_range() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(200, 4, 4));
        echo.start_position = Vec3::new(2.5, WALK_EYE_HEIGHT, 2.5);
        echo.pursuer.position = Vec3::new(112.5, WALK_EYE_HEIGHT, 2.5);
        assert!(echo.emit_ping(echo.start_position));
        for _ in 0..10 {
            echo.update_with_pursuer(1.0, Vec3::new(190.5, WALK_EYE_HEIGHT, 2.5));
        }
        assert_eq!(echo.pursuer.mode, EchoPursuerMode::Wander);
        echo.ping_cooldown_remaining = 0.0;
        echo.begin_pulse_charge();
        echo.update_with_pursuer(
            ECHO_CHARGED_PULSE_SECONDS,
            Vec3::new(190.5, WALK_EYE_HEIGHT, 2.5),
        );
        assert!(echo.release_pulse_charge(echo.start_position));
        let max_impact = echo.waves[0]
            .impacts
            .iter()
            .map(|impact| impact.arrival_distance_milli)
            .max()
            .unwrap_or_default() as f32
            / 1000.0;
        assert!(max_impact > echo.config.max_range);
        echo.update_with_pursuer(12.0, Vec3::new(190.5, WALK_EYE_HEIGHT, 2.5));
        assert!(matches!(
            echo.pursuer.mode,
            EchoPursuerMode::Investigate { .. }
        ));
    }

    #[test]
    fn echolocation_player_walking_emits_tiny_footstep_echoes() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let position = Vec3::new(4.5, WALK_EYE_HEIGHT, 4.5);
        let effects = echo.update_player_footsteps(ECHO_PLAYER_STEP_DISTANCE, position);
        assert_eq!(effects, vec![SoundEffect::PlayerFootstep]);
        assert_eq!(echo.step_waves.len(), 1);
        assert_eq!(
            echo.step_waves[0].origin,
            Vec3::new(position.x, ECHO_FOOTPRINT_SURFACE_Y, position.z)
        );
        assert!(echo.step_waves[0]
            .impacts
            .iter()
            .all(
                |impact| impact.arrival_distance_milli as f32 / 1000.0 <= ECHO_STEP_WAVE_MAX_RADIUS
            ));
    }

    #[test]
    fn echolocation_footsteps_require_range_and_line_of_sight_to_alert_pursuer() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(20, 4, 6));
        let step = Vec3::new(2.5, WALK_EYE_HEIGHT, 2.5);
        echo.pursuer.position = Vec3::new(8.5, WALK_EYE_HEIGHT, 2.5);
        echo.update_player_footsteps(ECHO_PLAYER_STEP_DISTANCE, step);
        assert!(matches!(
            echo.pursuer.mode,
            EchoPursuerMode::Investigate { .. }
        ));

        echo.pursuer.mode = EchoPursuerMode::Wander;
        echo.pursuer.target_position = None;
        echo.pursuer.position = Vec3::new(18.5, WALK_EYE_HEIGHT, 2.5);
        echo.update_player_footsteps(ECHO_PLAYER_STEP_DISTANCE, step);
        assert_eq!(echo.pursuer.mode, EchoPursuerMode::Wander);

        echo.pursuer.position = Vec3::new(8.5, WALK_EYE_HEIGHT, 2.5);
        for y in 1..=3 {
            echo.world.set(
                VoxelCoord::new(5, y, 2),
                VoxelCell::new(VoxelMaterial::ShipHull),
            );
        }
        echo.update_player_footsteps(ECHO_PLAYER_STEP_DISTANCE, step);
        assert_eq!(echo.pursuer.mode, EchoPursuerMode::Wander);
    }

    #[test]
    fn echolocation_investigation_refreshes_searches_and_expires() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(20, 4, 8));
        echo.pursuer.position = Vec3::new(12.5, WALK_EYE_HEIGHT, 4.5);
        let first_noise = Vec3::new(3.5, WALK_EYE_HEIGHT, 4.5);
        echo.notify_pursuer_of_noise(first_noise);
        echo.update_with_pursuer(1.0, Vec3::new(19.5, WALK_EYE_HEIGHT, 6.5));
        assert!(
            matches!(echo.pursuer.mode, EchoPursuerMode::Investigate { last_heard_position, remaining_seconds } if last_heard_position == first_noise && remaining_seconds < ECHO_PURSUER_INVESTIGATE_SECONDS)
        );
        let newer_noise = Vec3::new(6.5, WALK_EYE_HEIGHT, 4.5);
        echo.notify_pursuer_of_noise(newer_noise);
        assert!(
            matches!(echo.pursuer.mode, EchoPursuerMode::Investigate { last_heard_position, remaining_seconds } if last_heard_position == newer_noise && remaining_seconds == ECHO_PURSUER_INVESTIGATE_SECONDS)
        );
        echo.update_with_pursuer(
            ECHO_PURSUER_INVESTIGATE_SECONDS + 0.1,
            Vec3::new(19.5, WALK_EYE_HEIGHT, 6.5),
        );
        assert_eq!(echo.pursuer.mode, EchoPursuerMode::Wander);
        assert!(echo.pursuer.target_position.is_none());
    }

    #[test]
    fn echolocation_search_hud_and_twitch_only_render_during_investigation() {
        let mut app = AppState::new();
        app.start_echolocation();
        let calm = app.frame(0.0, false);
        assert!(!calm
            .layers
            .iter()
            .any(|layer| layer.name == "search-twitch"));
        assert!(!calm
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("IT HEARD YOU")));
        app.echolocation
            .notify_pursuer_of_noise(app.camera.position);
        let searching = app.frame(0.0, false);
        assert!(searching
            .layers
            .iter()
            .any(|layer| layer.name == "search-twitch"));
        assert!(searching
            .overlays
            .iter()
            .any(|overlay| overlay.text.contains("IT HEARD YOU")));
    }

    #[test]
    fn echolocation_search_static_escalates_with_pursuer_proximity() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let listener = echo.start_position;
        echo.notify_pursuer_of_noise(listener);
        echo.pursuer.position = listener + Vec3::new(30.0, 0.0, 0.0);
        let distant = echo_search_effect(&echo, listener);
        echo.pursuer.position = listener + Vec3::new(2.0, 0.0, 0.0);
        let close = echo_search_effect(&echo, listener);

        assert!(close.corruption_level > distant.corruption_level);
        assert!(
            echo_search_static_cells(VIEWPORT, close).len()
                > echo_search_static_cells(VIEWPORT, distant).len()
        );
    }

    #[test]
    fn echolocation_static_is_grayscale_and_stays_below_hud() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let listener = echo.start_position;
        echo.notify_pursuer_of_noise(listener);
        echo.pursuer.position = listener + Vec3::new(3.0, 0.0, 0.0);
        let effect = echo_search_effect(&echo, listener);
        let cells = echo_search_static_cells(VIEWPORT, effect);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|cell| matches!(
            cell.style.fg.as_deref(),
            Some("#17191b" | "#303438" | "#5e6469" | "#aeb3b5" | "#8b9093")
        )));

        let mut app = AppState::new();
        app.start_echolocation();
        app.echolocation
            .notify_pursuer_of_noise(app.camera.position);
        let scene = app.frame(0.0, false);
        let static_z = scene
            .layers
            .iter()
            .find(|layer| layer.name == "search-static")
            .unwrap()
            .z;
        assert!(scene.overlays.iter().all(|overlay| overlay.z > static_z));
    }

    #[test]
    fn echolocation_static_bursts_are_seeded_and_stop_when_search_ends() {
        let listener = Vec3::new(6.5, WALK_EYE_HEIGHT, 6.5);
        let mut a = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let mut b = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        for echo in [&mut a, &mut b] {
            echo.notify_pursuer_of_noise(listener);
            echo.pursuer.position = listener + Vec3::new(8.0, 0.0, 0.0);
        }
        let events_a = a.update_with_pursuer(0.0, listener);
        let events_b = b.update_with_pursuer(0.0, listener);
        assert_eq!(events_a, events_b);
        assert!(events_a
            .iter()
            .any(|effect| matches!(effect, SoundEffect::EcholocationStaticBurst { .. })));
        a.pursuer.mode = EchoPursuerMode::Wander;
        assert!(!a
            .update_with_pursuer(5.0, listener)
            .iter()
            .any(|effect| matches!(effect, SoundEffect::EcholocationStaticBurst { .. })));
    }

    #[test]
    fn echolocation_preserves_global_menu_and_escape_controls() {
        let mut app = AppState::new();
        app.start_echolocation();
        assert_eq!(
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyM), ElementState::Pressed),
            KeyboardAction::EnterMenu
        );

        app.start_echolocation();
        assert_eq!(
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Escape), ElementState::Pressed),
            KeyboardAction::ReleaseMouse
        );
    }

    #[test]
    fn echolocation_pursuer_spawns_walkable_past_the_closed_door() {
        let echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        assert!(can_walk_to_on_ground(
            &echo.world,
            echo.pursuer.position,
            ECHOLOCATION_WALK_PROFILE
        ));
        let navigation =
            NavigationField::build(&echo.world, echo.start_position, ECHOLOCATION_WALK_PROFILE)
                .expect("start should produce a navigation field");
        let distance = navigation
            .distance(
                echo.pursuer.position.x.floor() as i32,
                echo.pursuer.position.z.floor() as i32,
            )
            .expect("spawn should be in field");
        // A closed bulkhead isolates the pursuer beyond the first room. The
        // player cannot reach it until the receiver opens the puzzle door.
        assert_eq!(distance, u16::MAX);
        assert!(echo.pursuer.position.x > ECHO_DOOR_X as f32);
    }

    #[test]
    fn echolocation_pursuer_routes_and_emits_expiring_footsteps() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(8, 6, 8));
        echo.pursuer.position = Vec3::new(1.5, WALK_EYE_HEIGHT, 1.5);
        let player = Vec3::new(7.5, WALK_EYE_HEIGHT, 7.5);
        let before = horizontal_distance(echo.pursuer.position, player);
        let effects = echo.update_with_pursuer(0.2, player);
        assert!(horizontal_distance(echo.pursuer.position, player) < before);
        assert_eq!(echo.footprints.len(), 2);
        assert_eq!(echo.step_waves.len(), 2);
        assert_eq!(echo.step_waves[0].origin, echo.footprints[0].position);
        assert_eq!(echo.step_waves[1].origin, echo.footprints[1].position);
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, SoundEffect::InvisibleFootstep { .. })));
        echo.update_with_pursuer(ECHO_FOOTPRINT_LIFETIME + 0.1, player);
        assert!(echo.footprints.is_empty());
        assert!(echo.step_waves.is_empty());
    }

    #[test]
    fn echolocation_footprints_require_direct_line_of_sight() {
        let camera = look_at(Vec3::new(0.5, 2.5, 0.5), Vec3::new(0.5, 0.1, 4.5));
        let prints = [EchoFootprint {
            position: Vec3::new(0.5, ECHO_FOOTPRINT_SURFACE_Y, 4.5),
            remaining_seconds: ECHO_FOOTPRINT_LIFETIME,
            left: true,
            travel_direction: Vec3::new(0.0, 0.0, 1.0),
        }];
        let clear_world = VoxelWorld::new();
        assert_eq!(
            echo_footprint_cells(VIEWPORT, &camera, &clear_world, &prints).len(),
            1
        );

        let mut blocked_world = VoxelWorld::new();
        blocked_world.set(
            VoxelCoord::new(0, 1, 2),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        assert!(echo_footprint_cells(VIEWPORT, &camera, &blocked_world, &prints).is_empty());
    }

    #[test]
    fn echolocation_footprints_visibly_fade_with_age() {
        let fresh = EchoFootprint {
            position: Vec3::ZERO,
            remaining_seconds: ECHO_FOOTPRINT_LIFETIME,
            left: true,
            travel_direction: Vec3::new(0.0, 0.0, 1.0),
        };
        let old = EchoFootprint {
            remaining_seconds: ECHO_FOOTPRINT_LIFETIME * 0.1,
            ..fresh
        };
        let (fresh_glyph, fresh_color) = echo_footprint_visual(fresh);
        let (old_glyph, old_color) = echo_footprint_visual(old);
        assert_eq!(fresh_glyph, 'O');
        assert_eq!(old_glyph, '.');
        assert_ne!(fresh_color, old_color);
    }

    #[test]
    fn echolocation_step_waves_render_as_tiny_surface_echoes() {
        let world = echolocation_test_room(VoxelCoord::new(6, 6, 6));
        let source = Vec3::new(1.5, WALK_EYE_HEIGHT, 3.5);
        let camera = look_at(
            Vec3::new(4.5, WALK_EYE_HEIGHT, 3.5),
            Vec3::new(0.5, 3.5, 3.5),
        );
        let waves = [EchoStepWave {
            origin: source + Vec3::new(0.0, ECHO_FOOTPRINT_SURFACE_Y - WALK_EYE_HEIGHT, 0.0),
            impacts: build_echo_wave(
                &world,
                echo_pursuer_foot_source(source),
                1.0,
                0,
                ECHO_STEP_WAVE_MAX_RADIUS,
                source,
            )
            .impacts,
            age: 0.2,
            next_impact: 0,
        }];
        let cells = echo_step_wave_cells(VIEWPORT, &camera, &world, &waves);
        assert!(
            !cells.is_empty(),
            "impact distances: {:?}",
            waves[0]
                .impacts
                .iter()
                .map(|impact| impact.arrival_distance_milli)
                .collect::<Vec<_>>()
        );
        assert!(cells.iter().all(|cell| cell.glyph == '~'));
    }

    #[test]
    fn echolocation_step_waves_reveal_only_nearby_surfaces() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(6, 6, 6));
        let source = Vec3::new(1.5, WALK_EYE_HEIGHT, 3.5);
        echo.step_waves.push(EchoStepWave {
            origin: source + Vec3::new(0.0, ECHO_FOOTPRINT_SURFACE_Y - WALK_EYE_HEIGHT, 0.0),
            impacts: build_echo_wave(
                &echo.world,
                echo_pursuer_foot_source(source),
                1.0,
                0,
                ECHO_STEP_WAVE_MAX_RADIUS,
                source,
            )
            .impacts,
            age: 0.0,
            next_impact: 0,
        });
        echo.pursuer.position = source;
        echo.update_with_pursuer(0.25, Vec3::new(4.5, WALK_EYE_HEIGHT, 3.5));
        assert!(echo.revealed.contains_key(&VoxelCoord::new(0, 1, 3)));
        assert!(!echo.revealed.contains_key(&VoxelCoord::new(6, 1, 3)));
    }

    #[test]
    fn echolocation_scene_draws_a_directly_visible_print_in_the_starting_room() {
        let echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let camera = echolocation_start_camera(&echo);
        let prints = [EchoFootprint {
            position: Vec3::new(
                echo.start_position.x,
                ECHO_FOOTPRINT_SURFACE_Y,
                echo.start_position.z + 6.0,
            ),
            remaining_seconds: ECHO_FOOTPRINT_LIFETIME,
            left: true,
            travel_direction: Vec3::new(0.0, 0.0, 1.0),
        }];
        assert!(has_line_of_sight(
            &echo.world,
            camera.position,
            prints[0].position + Vec3::new(0.0, 0.04, 0.0)
        ));
        assert_eq!(
            echo_footprint_cells(VIEWPORT, &camera, &echo.world, &prints).len(),
            1
        );
    }

    #[test]
    fn echolocation_footsteps_pan_and_fade_by_relative_position() {
        let listener = Vec3::ZERO;
        let right = Vec3::new(1.0, 0.0, 0.0);
        let left_step = invisible_footstep_effect(Vec3::new(-8.0, 0.0, 0.0), listener, right);
        let right_step = invisible_footstep_effect(Vec3::new(8.0, 0.0, 0.0), listener, right);
        let near_step = invisible_footstep_effect(Vec3::new(2.0, 0.0, 0.0), listener, right);
        let far_step = invisible_footstep_effect(Vec3::new(38.0, 0.0, 0.0), listener, right);
        let SoundEffect::InvisibleFootstep {
            pan: left_pan,
            gain: left_gain,
        } = left_step
        else {
            unreachable!();
        };
        let SoundEffect::InvisibleFootstep {
            pan: right_pan,
            gain: right_gain,
        } = right_step
        else {
            unreachable!();
        };
        let SoundEffect::InvisibleFootstep {
            gain: near_gain, ..
        } = near_step
        else {
            unreachable!();
        };
        let SoundEffect::InvisibleFootstep { gain: far_gain, .. } = far_step else {
            unreachable!();
        };
        assert!(left_pan < 0.0 && right_pan > 0.0);
        assert_eq!(left_gain, right_gain);
        assert!(near_gain > far_gain, "near {near_gain}, far {far_gain}");
    }

    #[test]
    fn echolocation_pursuer_wanders_without_targeting_a_stationary_player() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let spawn = echo.pursuer.position;
        for _ in 0..600 {
            echo.update_with_pursuer(0.1, echo.start_position);
        }
        assert_ne!(echo.pursuer.position, spawn);
        assert_eq!(echo.pursuer.mode, EchoPursuerMode::Wander);
        assert_ne!(echo.run_status, EchoRunStatus::Dead);
    }

    #[test]
    fn echolocation_pursuer_is_absent_from_echo_visible_world() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let pursuer_cell = voxel_coord_at(echo.pursuer.position);
        echo.show_full_map = true;
        assert_eq!(
            echo.visible_world(echo.start_position).get(pursuer_cell),
            echo.world.get(pursuer_cell)
        );
        assert!(echo.world.get(pursuer_cell).is_none());
    }

    #[test]
    fn echolocation_contact_freezes_the_run_and_restart_restores_seeded_state() {
        let mut app = AppState::new();
        app.start_echolocation();
        let seeded_spawn = app.echolocation.pursuer.position;
        app.echolocation.pursuer.position = app.camera.position;
        app.frame(0.0, false);
        assert_eq!(app.echolocation.run_status, EchoRunStatus::Dead);
        app.drain_audio_events();
        let frozen_position = app.echolocation.pursuer.position;
        app.frame(1.0, false);
        assert_eq!(app.echolocation.pursuer.position, frozen_position);
        app.fire_weapon();
        assert!(app.drain_audio_events().is_empty());
        let scene = app.frame(0.0, false);
        assert!(scene
            .overlays
            .iter()
            .any(|overlay| overlay.text == "YOU WERE FOUND"));
        app.handle_keyboard(&PhysicalKey::Code(KeyCode::KeyR), ElementState::Pressed);
        assert_eq!(app.echolocation.run_status, EchoRunStatus::Active);
        assert_eq!(app.echolocation.seed, ECHOLOCATION_SEED);
        assert_eq!(app.echolocation.pursuer.position, seeded_spawn);
        assert_eq!(app.camera.position, app.echolocation.start_position);
    }

    fn echolocation_test_room(max: VoxelCoord) -> VoxelWorld {
        let mut world = VoxelWorld::new();
        for x in 0..=max.x {
            for y in 0..=max.y {
                for z in 0..=max.z {
                    if x == 0 || y == 0 || z == 0 || x == max.x || y == max.y || z == max.z {
                        world.set(
                            VoxelCoord::new(x, y, z),
                            VoxelCell::new(VoxelMaterial::Stone),
                        );
                    }
                }
            }
        }
        world
    }

    #[test]
    fn echolocation_contacts_continuous_room_surfaces_without_holes() {
        let max = VoxelCoord::new(6, 4, 5);
        let world = echolocation_test_room(max);
        let impacts = echo_impacts(&world, VoxelCoord::new(3, 2, 2), 40.0);
        let contacted: std::collections::HashSet<_> =
            impacts.iter().map(|impact| impact.solid_voxel).collect();

        for x in 1..max.x {
            for z in 1..max.z {
                assert!(contacted.contains(&VoxelCoord::new(x, 0, z)));
                assert!(contacted.contains(&VoxelCoord::new(x, max.y, z)));
            }
        }
        for y in 1..max.y {
            for z in 1..max.z {
                assert!(contacted.contains(&VoxelCoord::new(0, y, z)));
                assert!(contacted.contains(&VoxelCoord::new(max.x, y, z)));
            }
            for x in 1..max.x {
                assert!(contacted.contains(&VoxelCoord::new(x, y, 0)));
                assert!(contacted.contains(&VoxelCoord::new(x, y, max.z)));
            }
        }
    }

    #[test]
    fn echolocation_sealed_wall_blocks_every_voxel_behind_it() {
        let mut world = echolocation_test_room(VoxelCoord::new(7, 4, 4));
        for y in 1..4 {
            for z in 1..4 {
                world.set(
                    VoxelCoord::new(3, y, z),
                    VoxelCell::new(VoxelMaterial::ShipHull),
                );
            }
        }
        let hidden_target = VoxelCoord::new(5, 2, 2);
        world.set(hidden_target, VoxelCell::new(VoxelMaterial::Beacon));

        let impacts = echo_impacts(&world, VoxelCoord::new(1, 2, 2), 40.0);
        assert!(!impacts
            .iter()
            .any(|impact| impact.solid_voxel == hidden_target));
        assert!(!impacts.iter().any(|impact| impact.source_air_cell.x > 3));
    }

    #[test]
    fn echolocation_reaches_around_wall_only_through_doorway() {
        let mut open_world = echolocation_test_room(VoxelCoord::new(7, 4, 5));
        let target = VoxelCoord::new(5, 2, 1);
        open_world.set(target, VoxelCell::new(VoxelMaterial::Beacon));
        let direct_distance = echo_impacts(&open_world, VoxelCoord::new(1, 2, 1), 40.0)
            .into_iter()
            .find(|impact| impact.solid_voxel == target)
            .expect("target is directly reachable")
            .arrival_distance_milli;

        let mut doorway_world = open_world.clone();
        for y in 1..4 {
            for z in 1..5 {
                if (y, z) != (2, 4) {
                    doorway_world.set(
                        VoxelCoord::new(3, y, z),
                        VoxelCell::new(VoxelMaterial::ShipHull),
                    );
                }
            }
        }
        let doorway_distance = echo_impacts(&doorway_world, VoxelCoord::new(1, 2, 1), 40.0)
            .into_iter()
            .find(|impact| impact.solid_voxel == target)
            .expect("sound reaches the target through the doorway")
            .arrival_distance_milli;

        assert!(doorway_distance > direct_distance);
    }

    #[test]
    fn echolocation_diagonal_corner_cutting_is_blocked() {
        let mut world = VoxelWorld::new();
        let target = VoxelCoord::new(2, 1, 0);
        world.set(
            VoxelCoord::new(1, 0, 0),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        world.set(
            VoxelCoord::new(0, 1, 0),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        world.set(
            VoxelCoord::new(2, 0, 0),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        world.set(target, VoxelCell::new(VoxelMaterial::Beacon));

        let impacts = echo_impacts(&world, VoxelCoord::new(0, 0, 0), 10.0);
        assert!(!impacts.iter().any(|impact| impact.solid_voxel == target));
    }

    #[test]
    fn echolocation_open_diagonal_uses_weighted_distance() {
        let mut world = VoxelWorld::new();
        let target = VoxelCoord::new(2, 1, 0);
        world.set(
            VoxelCoord::new(2, 0, 0),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        world.set(
            VoxelCoord::new(0, 1, 0),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        world.set(target, VoxelCell::new(VoxelMaterial::Beacon));
        world.clear(VoxelCoord::new(0, 1, 0));

        let impact = echo_impacts(&world, VoxelCoord::new(0, 0, 0), 10.0)
            .into_iter()
            .find(|impact| impact.solid_voxel == target)
            .expect("open diagonal reaches the target");
        assert_eq!(impact.arrival_distance_milli, 2414);
    }

    #[test]
    fn echolocation_default_speed_is_deliberate_and_slow() {
        assert_eq!(EchoLocationConfig::default().ping_speed, 10.0);
        assert_eq!(EchoLocationConfig::default().echo_strength, 0.0);
    }

    #[test]
    fn echolocation_reveals_all_exposed_faces_but_not_shared_faces() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let coord = VoxelCoord::new(4, 2, 3);
        let cell = VoxelCell::new(VoxelMaterial::Stone);
        echo.world.set(coord, cell);
        echo.world.set(
            VoxelCoord::new(5, 2, 3),
            VoxelCell::new(VoxelMaterial::Stone),
        );
        echo.record_reveal(coord, cell, 1.0, VoxelCoord::new(3, 2, 3));

        assert!(echo.face_is_revealed(coord, Vec3::new(-1.0, 0.0, 0.0)));
        assert!(echo.face_is_revealed(coord, Vec3::new(0.0, 1.0, 0.0)));
        assert!(!echo.face_is_revealed(coord, Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn echolocation_map_has_a_continuous_enclosing_hull() {
        let (world, start, _) = build_echolocation_map(ECHOLOCATION_SEED);
        let mut exposed_floor_edges = 0;

        for_each_voxel(&world, |coord, cell| {
            if coord.y != 0 || cell.material != VoxelMaterial::Basalt {
                return;
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let neighbor = VoxelCoord::new(coord.x + dx, 0, coord.z + dz);
                if world.get(neighbor).is_none() {
                    exposed_floor_edges += 1;
                    for y in 1..=6 {
                        assert!(
                            world
                                .get(VoxelCoord::new(neighbor.x, y, neighbor.z))
                                .is_some(),
                            "open hull at {},{},{}",
                            neighbor.x,
                            y,
                            neighbor.z
                        );
                    }
                }
            }
        });

        assert!(exposed_floor_edges > 0);
        assert!(world
            .get(VoxelCoord::new(
                start.x.floor() as i32,
                0,
                start.z.floor() as i32
            ))
            .is_some());
    }

    #[test]
    fn echolocation_impacts_are_material_independent() {
        let mut signatures = Vec::new();
        for material in [
            VoxelMaterial::Stone,
            VoxelMaterial::Basalt,
            VoxelMaterial::Glass,
        ] {
            let mut world = echolocation_test_room(VoxelCoord::new(6, 4, 4));
            let target = VoxelCoord::new(4, 2, 2);
            world.set(target, VoxelCell::new(material));
            let impacts = echo_impacts(&world, VoxelCoord::new(1, 2, 2), 20.0);
            assert_eq!(
                impacts
                    .iter()
                    .find(|impact| impact.solid_voxel == target)
                    .expect("target is contacted")
                    .cell
                    .material,
                material
            );
            signatures.push(
                impacts
                    .into_iter()
                    .map(|impact| {
                        (
                            impact.solid_voxel,
                            impact.source_air_cell,
                            impact.arrival_distance_milli,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }

        assert!(signatures.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn echolocation_range_limits_propagation() {
        let mut world = echolocation_test_room(VoxelCoord::new(10, 4, 4));
        let target = VoxelCoord::new(8, 2, 2);
        world.set(target, VoxelCell::new(VoxelMaterial::Beacon));

        assert!(!echo_impacts(&world, VoxelCoord::new(1, 2, 2), 6.9)
            .iter()
            .any(|impact| impact.solid_voxel == target));
        assert!(echo_impacts(&world, VoxelCoord::new(1, 2, 2), 7.0)
            .iter()
            .any(|impact| impact.solid_voxel == target));
    }

    #[test]
    fn echolocation_speed_changes_timing_not_reachable_impacts() {
        let mut world = echolocation_test_room(VoxelCoord::new(8, 4, 4));
        let target = VoxelCoord::new(5, 2, 2);
        world.set(target, VoxelCell::new(VoxelMaterial::Beacon));
        let mut fast = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        fast.world = world.clone();
        fast.config.max_range = 20.0;
        fast.config.ping_speed = 10.0;
        fast.emit_ping(Vec3::new(1.5, 2.5, 2.5));
        let mut slow = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        slow.world = world;
        slow.config.max_range = 20.0;
        slow.config.ping_speed = 5.0;
        slow.emit_ping(Vec3::new(1.5, 2.5, 2.5));

        assert_eq!(fast.waves[0].impacts, slow.waves[0].impacts);
        fast.update(0.41);
        slow.update(0.41);
        assert!(fast.revealed.contains_key(&target));
        assert!(!slow.revealed.contains_key(&target));
    }

    #[test]
    fn echolocation_default_strength_has_no_secondary_waves() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(6, 4, 4));
        echo.emit_ping(Vec3::new(2.5, 2.5, 2.5));
        echo.update(0.3);

        assert_eq!(echo.config.echo_strength, 0.0);
        assert_eq!(echo.reflected_pulse_count(), 0);
    }

    #[test]
    fn echolocation_secondary_waves_are_weaker_and_wall_safe() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.config.echo_strength = 0.72;
        echo.world = echolocation_test_room(VoxelCoord::new(7, 4, 4));
        for y in 1..4 {
            for z in 1..4 {
                echo.world.set(
                    VoxelCoord::new(3, y, z),
                    VoxelCell::new(VoxelMaterial::ShipHull),
                );
            }
        }
        let hidden_target = VoxelCoord::new(5, 2, 2);
        echo.world
            .set(hidden_target, VoxelCell::new(VoxelMaterial::Beacon));
        let emission = Vec3::new(1.5, 2.5, 2.5);
        echo.emit_ping(emission);
        echo.update(0.25);

        let secondary = echo
            .waves
            .iter()
            .find(|wave| wave.bounce_depth == 1)
            .expect("wall contact spawns a secondary wave");
        assert!(secondary.energy < echo.config.initial_energy);
        assert_eq!(secondary.original_emission_position, emission);
        for _ in 0..8 {
            echo.update(0.5);
            assert!(!echo.revealed.contains_key(&hidden_target));
        }
    }

    #[test]
    fn echolocation_full_map_toggle_bypasses_pulse_filter() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let target = VoxelCoord::new(2, 1, 0);
        echo.world = VoxelWorld::new();
        echo.world.set(target, VoxelCell::new(VoxelMaterial::Glass));

        assert_eq!(echo.visible_world(Vec3::ZERO).get(target), None);
        echo.toggle_full_map();
        assert_eq!(
            echo.visible_world(Vec3::ZERO).get(target),
            Some(VoxelCell::new(VoxelMaterial::Glass))
        );
        echo.toggle_full_map();
        assert_eq!(echo.visible_world(Vec3::ZERO).get(target), None);
    }

    #[test]
    fn echolocation_strength_derives_return_gain_and_bounce_depth() {
        assert_eq!(echo_bounce_limit(0.10), 0);
        assert_eq!(echo_bounce_limit(0.50), 1);
        assert_eq!(echo_bounce_limit(0.75), 2);
        assert_eq!(echo_bounce_limit(1.00), 3);
        assert!(echo_reflection_gain(0.9) > echo_reflection_gain(0.3));
    }

    #[test]
    fn echolocation_large_frame_consumes_all_crossed_impacts_and_retains_reveals() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = echolocation_test_room(VoxelCoord::new(8, 4, 4));
        let near_target = VoxelCoord::new(3, 0, 2);
        let far_target = VoxelCoord::new(8, 2, 2);
        echo.emit_ping(Vec3::new(1.5, 2.5, 2.5));

        echo.update(1.0);
        assert!(echo.revealed.contains_key(&near_target));
        assert!(echo.revealed.contains_key(&far_target));
        assert!(echo.visible_world(Vec3::ZERO).get(near_target).is_some());
        assert!(echo.visible_world(Vec3::ZERO).get(far_target).is_some());

        echo.waves.clear();
        echo.update(echo.config.reveal_seconds + 0.1);
        assert!(echo.visible_world(Vec3::ZERO).get(near_target).is_none());
    }

    fn receiver_impact() -> EchoImpact {
        EchoImpact {
            solid_voxel: ECHO_RECEIVER_COORD,
            cell: VoxelCell::new(VoxelMaterial::Receiver),
            source_air_cell: VoxelCoord::new(ECHO_RECEIVER_COORD.x + 1, 1, 0),
            arrival_distance_milli: 1_000,
        }
    }

    fn advance_puzzle(
        echo: &mut EchoLocationState,
        dt: f32,
        hits: Vec<f32>,
        player: Vec3,
    ) -> EchoFrameUpdate {
        let mut sound_events = Vec::new();
        let corrected_player_position = echo.update_puzzle(
            dt,
            hits,
            player,
            Vec3::new(0.0, 0.0, 1.0),
            &mut sound_events,
        );
        EchoFrameUpdate {
            sound_events,
            corrected_player_position,
        }
    }

    #[test]
    fn echolocation_map_contains_receiver_pipe_and_sealed_bulkhead() {
        let echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let mut receiver_count = 0;
        let mut pipe_count = 0;
        let mut door_count = 0;
        for_each_voxel(&echo.world, |_, cell| match cell.material {
            VoxelMaterial::Receiver => receiver_count += 1,
            VoxelMaterial::SignalPipe => pipe_count += 1,
            VoxelMaterial::PuzzleDoor => door_count += 1,
            _ => {}
        });
        assert_eq!(receiver_count, 1);
        assert_eq!(pipe_count, 16);
        assert_eq!(door_count, 12);
        assert_eq!(echo.puzzle.endpoint_distance(), 15.0);
        for point in &echo.puzzle.pipe {
            let coord = VoxelCoord::new(point.position.x.floor() as i32, 0, 0);
            assert_eq!(
                echo.world.get(coord),
                Some(VoxelCell::new(VoxelMaterial::SignalPipe))
            );
        }
        for y in 1..=5 {
            for z in -5..=5 {
                let coord = VoxelCoord::new(ECHO_DOOR_X, y, z);
                let in_aperture = y <= 4 && (-1..=1).contains(&z);
                assert_eq!(
                    echo.world.get(coord).map(|cell| cell.material),
                    Some(if in_aperture {
                        VoxelMaterial::PuzzleDoor
                    } else {
                        VoxelMaterial::ShipHull
                    })
                );
            }
        }
    }

    #[test]
    fn primary_and_reflected_impacts_each_activate_receiver_once() {
        for bounce_depth in [0, 1] {
            let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
            echo.pursuer.position = Vec3::new(-30.5, WALK_EYE_HEIGHT, 7.5);
            echo.waves = vec![EchoWave {
                age: 0.0,
                energy: 1.0,
                bounce_depth,
                original_emission_position: echo.start_position,
                heard_by_pursuer: true,
                impacts: vec![receiver_impact()],
                next_impact: 0,
            }];
            let effects = echo.update_with_pursuer(0.2, echo.start_position);
            assert_eq!(echo.puzzle.emissions.len(), 1);
            assert_eq!(
                effects
                    .iter()
                    .filter(|effect| matches!(effect, SoundEffect::ReceiverActivation { .. }))
                    .count(),
                1
            );
            echo.update_with_pursuer(0.2, echo.start_position);
            assert_eq!(echo.puzzle.emissions.len(), 1);
        }
    }

    #[test]
    fn player_and_pursuer_step_impacts_activate_receiver() {
        let player = Vec3::new(-34.5, WALK_EYE_HEIGHT, 0.5);
        let mut player_echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        player_echo.pursuer.position = Vec3::new(-24.5, WALK_EYE_HEIGHT, 7.5);
        player_echo.pursuer.step_timer = 10.0;
        player_echo.update_player_footsteps(ECHO_PLAYER_STEP_DISTANCE, player);
        player_echo.update_with_pursuer(0.3, player);
        assert_eq!(player_echo.puzzle.emissions.len(), 1);

        let mut pursuer_echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        pursuer_echo.pursuer.position = player;
        pursuer_echo.pursuer.step_timer = 0.0;
        let mut effects = Vec::new();
        pursuer_echo.update_pursuer_footsteps(
            0.0,
            Vec3::new(-24.5, WALK_EYE_HEIGHT, 7.5),
            Vec3::new(1.0, 0.0, 0.0),
            &mut effects,
        );
        pursuer_echo.pursuer.step_timer = 10.0;
        pursuer_echo.update_with_pursuer(0.3, Vec3::new(-24.5, WALK_EYE_HEIGHT, 7.5));
        assert_eq!(pursuer_echo.puzzle.emissions.len(), 1);
    }

    #[test]
    fn waves_without_receiver_contact_do_not_activate_it() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.pursuer.step_timer = 10.0;
        echo.waves = vec![EchoWave {
            age: 0.0,
            energy: 1.0,
            bounce_depth: 0,
            original_emission_position: echo.start_position,
            heard_by_pursuer: true,
            impacts: Vec::new(),
            next_impact: 0,
        }];
        echo.update_with_pursuer(1.0, echo.start_position);
        assert!(echo.puzzle.emissions.is_empty());
    }

    #[test]
    fn receiver_signal_opens_after_two_and_a_half_seconds_then_closes_three_later() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let player = echo.start_position;
        advance_puzzle(&mut echo, 0.0, vec![0.0], player);
        assert!(!echo.puzzle.door_open);
        advance_puzzle(&mut echo, 2.49, Vec::new(), player);
        assert!(!echo.puzzle.door_open);
        advance_puzzle(&mut echo, 0.01, Vec::new(), player);
        assert!(echo.puzzle.door_open);
        assert_eq!(echo.puzzle.transitions, vec![EchoDoorTransition::Opened]);
        advance_puzzle(&mut echo, 2.99, Vec::new(), player);
        assert!(echo.puzzle.door_open);
        advance_puzzle(&mut echo, 0.01, Vec::new(), player);
        assert!(!echo.puzzle.door_open);
        assert_eq!(echo.puzzle.transitions, vec![EchoDoorTransition::Closed]);
    }

    #[test]
    fn receiver_signal_head_advances_at_six_voxels_per_second() {
        let mut puzzle = EchoPuzzle::new();
        puzzle.emissions = vec![EchoEmissionInterval {
            start: 0.0,
            end: ECHO_RECEIVER_OUTPUT_SECONDS,
        }];
        let energized_head = |puzzle: &EchoPuzzle, time: f32| {
            puzzle
                .pipe
                .iter()
                .filter(|point| puzzle.signal_active_at(point.distance, time))
                .map(|point| point.distance)
                .fold(0.0_f32, f32::max)
        };
        let half_second = energized_head(&puzzle, 0.5);
        let one_second = energized_head(&puzzle, 1.0);
        let two_seconds = energized_head(&puzzle, 2.0);
        assert_eq!((half_second, one_second, two_seconds), (3.0, 6.0, 12.0));
        assert!(half_second < one_second && one_second < two_seconds);
    }

    #[test]
    fn puzzle_materials_have_public_display_labels() {
        assert_eq!(material_label(VoxelMaterial::Receiver), "receiver");
        assert_eq!(material_label(VoxelMaterial::SignalPipe), "signal pipe");
        assert_eq!(material_label(VoxelMaterial::PuzzleDoor), "puzzle door");
        let (assets, _) = build_asset_catalog();
        for name in ["receiver block", "signal pipe block", "puzzle door block"] {
            assert!(assets.iter().any(|asset| asset.name == name));
        }
    }

    #[test]
    fn receiver_retrigger_extends_emission_but_post_shutoff_hit_preserves_gap() {
        let mut continuous = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let player = continuous.start_position;
        advance_puzzle(&mut continuous, 0.0, vec![0.0], player);
        advance_puzzle(&mut continuous, 2.0, Vec::new(), player);
        advance_puzzle(&mut continuous, 0.0, vec![2.0], player);
        assert_eq!(continuous.puzzle.emissions.len(), 1);
        assert_eq!(continuous.puzzle.emissions[0].end, 5.0);

        let mut gapped = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        advance_puzzle(&mut gapped, 0.0, vec![0.0], player);
        advance_puzzle(&mut gapped, 4.0, Vec::new(), player);
        advance_puzzle(&mut gapped, 0.0, vec![4.0], player);
        assert_eq!(gapped.puzzle.emissions.len(), 2);
        assert!(!gapped.puzzle.signal_active_at(15.0, 6.0));
        assert!(gapped.puzzle.signal_active_at(0.0, 6.0));
    }

    #[test]
    fn large_frame_consumes_all_door_edges_deterministically() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let player = echo.start_position;
        advance_puzzle(&mut echo, 0.0, vec![0.0], player);
        advance_puzzle(&mut echo, 6.0, Vec::new(), player);
        assert_eq!(
            echo.puzzle.transitions,
            vec![EchoDoorTransition::Opened, EchoDoorTransition::Closed]
        );
        assert!(!echo.puzzle.door_open);
        assert!(echo.puzzle.emissions.is_empty());
    }

    #[test]
    fn puzzle_door_controls_collision_navigation_sight_and_new_sound_paths() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        let west = Vec3::new(-22.5, WALK_EYE_HEIGHT, 0.5);
        let doorway = Vec3::new(-20.5, WALK_EYE_HEIGHT, 0.5);
        let east = Vec3::new(-18.5, WALK_EYE_HEIGHT, 0.5);
        let target = VoxelCoord::new(-17, 2, 0);
        echo.world
            .set(target, VoxelCell::new(VoxelMaterial::Beacon));
        assert!(!can_walk_to_with_profile(
            &echo.world,
            doorway,
            ECHOLOCATION_WALK_PROFILE
        ));
        let closed_navigation =
            NavigationField::build(&echo.world, east, ECHOLOCATION_WALK_PROFILE).unwrap();
        assert_eq!(closed_navigation.distance(-23, 0), Some(u16::MAX));
        assert!(!has_line_of_sight(&echo.world, west, east));
        assert!(!echo_impacts(&echo.world, voxel_coord_at(west), 12.0)
            .iter()
            .any(|impact| impact.solid_voxel == target));

        echo.set_puzzle_door_open(true);
        assert!(can_walk_to_with_profile(
            &echo.world,
            doorway,
            ECHOLOCATION_WALK_PROFILE
        ));
        let open_navigation =
            NavigationField::build(&echo.world, east, ECHOLOCATION_WALK_PROFILE).unwrap();
        assert_ne!(open_navigation.distance(-23, 0), Some(u16::MAX));
        assert!(has_line_of_sight(&echo.world, west, east));
        assert!(echo_impacts(&echo.world, voxel_coord_at(west), 12.0)
            .iter()
            .any(|impact| impact.solid_voxel == target));
    }

    #[test]
    fn closing_door_pushes_occupants_to_deterministic_nearest_sides() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.set_puzzle_door_open(true);
        echo.puzzle.time = 5.49;
        echo.puzzle.emissions = vec![EchoEmissionInterval {
            start: 0.0,
            end: 3.0,
        }];
        echo.pursuer.position = Vec3::new(-20.2, WALK_EYE_HEIGHT, 0.5);
        let update = advance_puzzle(
            &mut echo,
            0.02,
            Vec::new(),
            Vec3::new(-20.8, WALK_EYE_HEIGHT, 0.5),
        );
        assert_eq!(
            update.corrected_player_position,
            Some(echo.puzzle.door.starting_side_anchor)
        );
        assert_eq!(echo.pursuer.position, echo.puzzle.door.far_side_anchor);
        assert!(echo.pursuer.target_position.is_none());
    }

    #[test]
    fn unoccupied_door_close_does_not_move_entities() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.set_puzzle_door_open(true);
        echo.puzzle.time = 5.49;
        echo.puzzle.emissions = vec![EchoEmissionInterval {
            start: 0.0,
            end: 3.0,
        }];
        let player = echo.start_position;
        let pursuer = echo.pursuer.position;
        let update = advance_puzzle(&mut echo, 0.02, Vec::new(), player);
        assert_eq!(update.corrected_player_position, None);
        assert_eq!(echo.pursuer.position, pursuer);
    }

    #[test]
    fn active_signal_is_self_lit_but_line_of_sight_occluded() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.world = VoxelWorld::new();
        echo.revealed.clear();
        echo.puzzle.time = 5.0;
        echo.puzzle.emissions = vec![EchoEmissionInterval {
            start: 0.0,
            end: 10.0,
        }];
        let camera = look_at(Vec3::new(-28.5, 3.0, 8.5), Vec3::new(-28.5, 1.0, 0.5));
        let visible = echo_receiver_signal_cells(VIEWPORT, &camera, &echo);
        assert!(!visible.is_empty());
        fill_cuboid(
            &mut echo.world,
            VoxelCoord::new(-40, 0, 4),
            VoxelCoord::new(-15, 5, 4),
            VoxelMaterial::ShipHull,
        );
        assert!(echo_receiver_signal_cells(VIEWPORT, &camera, &echo).is_empty());
    }

    #[test]
    fn receiver_signal_layer_sits_above_footprints_and_below_static_and_hud() {
        let mut echo = EchoLocationState::new_seeded(ECHOLOCATION_SEED);
        echo.notify_pursuer_of_noise(echo.start_position);
        let camera = echolocation_start_camera(&echo);
        let mut scene = Scene::new(VIEWPORT);
        render_echolocation_scene(&mut scene, &echo, &camera, false);
        let signal_z = scene
            .layers
            .iter()
            .find(|layer| layer.name == "receiver-signal")
            .unwrap()
            .z;
        let footprint_z = scene
            .layers
            .iter()
            .find(|layer| layer.name == "footprints")
            .unwrap()
            .z;
        let static_z = scene
            .layers
            .iter()
            .find(|layer| layer.name == "search-static")
            .unwrap()
            .z;
        assert!(signal_z > footprint_z && signal_z < static_z);
        assert!(scene.overlays.iter().all(|overlay| overlay.z > signal_z));
    }

    #[test]
    fn puzzle_time_freezes_on_death_and_restart_rebuilds_closed_state() {
        let mut app = AppState::new();
        app.start_echolocation();
        let player = app.camera.position;
        advance_puzzle(&mut app.echolocation, 1.0, vec![0.0], player);
        let frozen_time = app.echolocation.puzzle.time;
        app.echolocation.run_status = EchoRunStatus::Dead;
        app.echolocation
            .update_with_pursuer_from_listener(3.0, player, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(app.echolocation.puzzle.time, frozen_time);
        app.start_echolocation();
        assert_eq!(app.echolocation.puzzle.time, 0.0);
        assert!(app.echolocation.puzzle.emissions.is_empty());
        assert!(!app.echolocation.puzzle.door_open);
        assert!(app.echolocation.puzzle.door.voxels.iter().all(|coord| app
            .echolocation
            .world
            .get(*coord)
            == Some(VoxelCell::new(VoxelMaterial::PuzzleDoor))));
    }

    #[test]
    fn puzzle_spatial_parameters_are_finite_limited_and_pan_consistently() {
        let listener = Vec3::ZERO;
        let right = Vec3::new(1.0, 0.0, 0.0);
        let (left_pan, near_gain) =
            spatial_sound_parameters(Vec3::new(-2.0, 0.0, 0.0), listener, right, 0.08, 0.72, 52.0);
        let (right_pan, far_gain) =
            spatial_sound_parameters(Vec3::new(40.0, 0.0, 0.0), listener, right, 0.08, 0.72, 52.0);
        assert!(left_pan < 0.0 && right_pan > 0.0);
        assert!(near_gain > far_gain);
        assert!([left_pan, right_pan, near_gain, far_gain]
            .iter()
            .all(|value| value.is_finite() && (-1.0..=1.0).contains(value)));
    }
}
