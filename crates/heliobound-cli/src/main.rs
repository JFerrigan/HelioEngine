use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use font8x8::{UnicodeFonts, BASIC_FONTS};
use heliobound_core::{
    Camera, CityConfig, CityGenerator, DoomMapConfig, DoomMapGenerator, PlanetConfig,
    ProceduralPlanet, Ray, Vec3, VoxelCoord, VoxelWorld,
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
const ENEMY_EYE_HEIGHT: f32 = 2.1;
const ENEMY_SPEED: f32 = 5.5;
const ENEMY_ATTACK_RANGE: f32 = 2.4;
const ENEMY_ATTACK_DAMAGE: i32 = 8;
const WEAPON_RANGE: f32 = 95.0;
const WEAPON_DAMAGE: i32 = 55;
const SHOT_FLASH_TIME: f32 = 0.12;
const MOUSE_SENSITIVITY: f32 = 0.0025;
const PITCH_LIMIT: f32 = 1.52;
const ROLL_SPEED: f32 = 1.8;

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = AppState::new();
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
                            }
                        }
                        KeyboardAction::EnterMenu => {
                            app.enter_menu();
                            mouse_captured = set_mouse_captured(&window, false);
                        }
                        KeyboardAction::StartScene => {
                            mouse_captured = set_mouse_captured(&window, false);
                        }
                        KeyboardAction::Fire => {
                            app.fire_weapon();
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
    planet_builder: SceneBuilder,
    city_builder: SceneBuilder,
    camera: Camera,
    input: PlayerInput,
    shooter: ShooterState,
    tick: u64,
}

impl AppState {
    fn new() -> Self {
        Self {
            mode: AppMode::Menu,
            planet: build_demo_planet(),
            city: build_demo_city(),
            doom_map: build_doom_map(),
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
            shooter: ShooterState::new(),
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
    }

    fn start_shooter(&mut self) {
        self.mode = AppMode::CityShooter;
        self.camera = doom_start_camera();
        self.input = PlayerInput::default();
        self.shooter = ShooterState::new();
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
                let mut scene = self.city_builder.build(&self.city, &self.camera, self.tick);
                scene.overlays.push(Overlay {
                    x: 2,
                    y: 2,
                    z: 120,
                    text: format!(
                        "CITY WALK  mouse {}  M menu  pos {:.1},{:.1},{:.1}",
                        if mouse_captured { "locked" } else { "free" },
                        self.camera.position.x,
                        self.camera.position.y,
                        self.camera.position.z
                    ),
                    style: TextStyle::default(),
                });
                scene
            }
            AppMode::CityShooter => {
                update_walking_camera(&mut self.camera, &self.input, &self.doom_map, dt);
                self.shooter
                    .update(&self.doom_map, self.camera.position, dt);
                let mut scene = self
                    .city_builder
                    .build(&self.doom_map, &self.camera, self.tick);
                render_shooter_scene(
                    &mut scene,
                    &self.doom_map,
                    &self.camera,
                    &self.shooter,
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
            AppMode::CityWalk | AppMode::CityShooter => {
                apply_mouse_look(&mut self.camera, delta_x, delta_y, PitchMode::Clamped)
            }
        }
    }

    fn fire_weapon(&mut self) {
        if self.mode == AppMode::CityShooter {
            self.shooter.fire(&self.doom_map, &self.camera);
        }
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

#[derive(Clone, Debug)]
struct ShooterState {
    enemies: Vec<Enemy>,
    health: i32,
    kills: u32,
    shots_fired: u32,
    shot_flash_timer: f32,
}

impl ShooterState {
    fn new() -> Self {
        Self {
            enemies: spawn_enemies(),
            health: 100,
            kills: 0,
            shots_fired: 0,
            shot_flash_timer: 0.0,
        }
    }

    fn update(&mut self, city: &VoxelWorld, player_position: Vec3, dt: f32) {
        self.shot_flash_timer = (self.shot_flash_timer - dt).max(0.0);

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
    }

    fn fire(&mut self, city: &VoxelWorld, camera: &Camera) {
        self.shots_fired += 1;
        self.shot_flash_timer = SHOT_FLASH_TIME;

        let Some(index) = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, enemy)| enemy.is_alive())
            .filter_map(|(index, enemy)| {
                let target = enemy.target_position();
                let delta = target - camera.position;
                let distance = delta.length();
                if distance <= f32::EPSILON || distance > WEAPON_RANGE {
                    return None;
                }

                let direction = delta.normalized();
                let forward = direction.dot(camera.forward());
                let aim_x = direction.dot(camera.right()).abs();
                let aim_y = direction.dot(camera.up()).abs();

                if forward < 0.985 || aim_x > 0.08 || aim_y > 0.10 {
                    return None;
                }
                if !has_line_of_sight(city, camera.position, target) {
                    return None;
                }

                Some((index, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
        else {
            return;
        };

        let enemy = &mut self.enemies[index];
        enemy.health = (enemy.health - WEAPON_DAMAGE).max(0);
        if enemy.health == 0 {
            self.kills += 1;
        }
    }

    fn alive_count(&self) -> usize {
        self.enemies.iter().filter(|enemy| enemy.is_alive()).count()
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

    fn target_position(self) -> Vec3 {
        Vec3::new(self.position.x, ENEMY_EYE_HEIGHT, self.position.z)
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
            WALK_SPEED * WALK_BOOST_MULTIPLIER
        } else {
            WALK_SPEED
        };
        let candidate = camera.position + movement.normalized() * speed * dt;
        let candidate = Vec3::new(candidate.x, WALK_EYE_HEIGHT, candidate.z);

        if can_walk_to(city, candidate) {
            camera.position = candidate;
        }
    }
}

fn horizontal(direction: Vec3) -> Vec3 {
    Vec3::new(direction.x, 0.0, direction.z).normalized()
}

fn can_walk_to(city: &VoxelWorld, position: Vec3) -> bool {
    let x = position.x.floor() as i32;
    let z = position.z.floor() as i32;
    for y in 1..=WALK_EYE_HEIGHT.ceil() as i32 {
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
        y: 52,
        z: 10,
        text: "WASD MOVE   CLICK/SPACE FIRE   M MENU".to_string(),
        style: TextStyle::default(),
    });
    scene
}

fn render_shooter_scene(
    scene: &mut Scene,
    city: &VoxelWorld,
    camera: &Camera,
    shooter: &ShooterState,
    mouse_captured: bool,
) {
    let mut projected: Vec<(f32, Vec<SceneCell>)> = shooter
        .enemies
        .iter()
        .filter(|enemy| enemy.is_alive())
        .filter_map(|enemy| {
            let target = enemy.target_position();
            if !has_line_of_sight(city, camera.position, target) {
                return None;
            }
            project_world_point(camera, target, scene.viewport)
                .map(|projection| (projection.distance, enemy_cells(projection)))
        })
        .collect();
    projected.sort_by(|a, b| b.0.total_cmp(&a.0));

    scene.layers.push(Layer {
        name: "enemies".to_string(),
        z: 30,
        cells: projected.into_iter().flat_map(|(_, cells)| cells).collect(),
    });
    scene.layers.push(Layer {
        name: "weapon".to_string(),
        z: 40,
        cells: weapon_cells(scene.viewport, shooter.shot_flash_timer > 0.0),
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

fn enemy_cells(projection: Projection) -> Vec<SceneCell> {
    let sprite = enemy_sprite(projection.distance);
    let height = sprite.len() as i32;
    let width = sprite.iter().map(|line| line.len()).max().unwrap_or(0) as i32;
    let start_x = projection.x - width / 2;
    let start_y = projection.y - height / 2;
    let mut cells = Vec::new();

    for (row, line) in sprite.iter().enumerate() {
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

fn enemy_sprite(distance: f32) -> &'static [&'static str] {
    if distance < 12.0 {
        &[
            "  .---.  ",
            " /o o \\ ",
            "|  ^  | ",
            "| \\_/ | ",
            " /|||\\  ",
            "/_|||_\\ ",
            "  / \\   ",
        ]
    } else if distance < 28.0 {
        &[" .-. ", "/o o\\", "| ^ |", "/|||\\", " / \\ "]
    } else if distance < 55.0 {
        &["oOo", "/|\\", "/ \\"]
    } else {
        &["&"]
    }
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
            "weapon" => [0xf0, 0xc6, 0x5b, 0xff],
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
    fn menu_can_start_city_mode() {
        let mut app = AppState::new();

        let action =
            app.handle_keyboard(&PhysicalKey::Code(KeyCode::Digit2), ElementState::Pressed);

        assert_eq!(action, KeyboardAction::StartScene);
        assert_eq!(app.mode, AppMode::CityWalk);
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
    fn shooter_fire_damages_centered_enemy() {
        let mut app = AppState::new();
        app.start_shooter();

        let health_before = app.shooter.enemies[0].health;
        app.fire_weapon();

        assert!(app.shooter.enemies[0].health < health_before);
        assert_eq!(app.shooter.shots_fired, 1);
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
    fn shooter_scene_projects_visible_enemies_and_weapon() {
        let mut app = AppState::new();
        app.start_shooter();

        let scene = app.frame(0.0, true);

        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "enemies" && !layer.cells.is_empty()));
        assert!(scene
            .layers
            .iter()
            .any(|layer| layer.name == "weapon" && !layer.cells.is_empty()));
    }

    #[test]
    fn visible_enemy_uses_multi_cell_sprite() {
        let projection = Projection {
            x: 80,
            y: 45,
            distance: 24.0,
        };

        let cells = enemy_cells(projection);

        assert!(cells.len() > 10);
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
