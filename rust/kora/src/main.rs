//! K.O. Racing 3D - Rust reimplementation of the Jollybox J2ME racer.
//!
//! The original MIDlet reads every asset out of its own `data`/`data.<n>`
//! archive; this port does exactly the same, parsing the model, tile, map,
//! car, campaign and font formats directly instead of converting them.
//! Rendering is macroquad, physics is the game's own arcade vehicle model.
//!
//! Menus: `KORA_SKIP_MENU=1` boots straight into a race, and `KORA_MAP`,
//! `KORA_CAR`, `KORA_LAPS`, `KORA_OPPONENTS` and `KORA_ASSETS` still override
//! what the menus would pick.  Progress is saved to `KORA_SAVE` (default
//! `kora-save.txt`).

use std::path::PathBuf;

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, PlaySoundParams, Sound};
use macroquad::models::{draw_mesh, Mesh};
use macroquad::prelude::*;

use kora::ai::AiDriver;
use kora::campaign::{self, RaceEvent};
use kora::labels;
use kora::menu::Outcome;
use kora::physics::{CarControl, Tuning, World};
use kora::engine::Engine;
use kora::progress::{self, Progress};
use kora::race::{Mode, Race, drift_points};
use kora::text;
use kora::settings::Settings;
use kora::{format, hud, map, menu, music, pack, paths, scene, sky, space, theme};

/// The asset directory: `KORA_ASSETS`, or `assets` in the working directory.
///
/// That directory is an extracted tree - every resource at its original path,
/// plus the `lists`, `ui` and `sounds` the game reads straight from the JAR -
/// as `setup.sh` leaves it.  Pointing `KORA_ASSETS` at `x` reads the packed
/// archive instead, which is the form the game shipped.
fn assets_dir() -> PathBuf {
    match std::env::var("KORA_ASSETS") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from("assets"),
    }
}

/// Environment override, if set to a valid number.
fn env_number(name: &str, max: u32) -> Option<u32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.min(max))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Main,
    Options,
    Career,
    Deluxe,
    Quick,
    Cars,
    Race,
    Paused,
    Results,
    /// The career map (`u`, or `br` for the deluxe tour), by table index.  The
    /// list screens stay as the fallback for a table with no markers on it.
    Map(usize),
}

/// The jump from the menu into a map: `bd.F()` sets the target and the ramp in
/// `bd.b(float)` runs the billboard's scale up at it.  The scale is also what
/// the stars are dragged at (`bd` hands the same number to `ce`), which is why
/// the planet grows and the starfield scatters at the same moment.
struct Jump {
    table: usize,
    scale: f32,
    elapsed: f32,
}

impl Jump {
    fn new(table: usize) -> Jump {
        Jump {
            table,
            scale: space::SCALE,
            elapsed: 0.0,
        }
    }

    fn update(&mut self, dt: f32) {
        self.elapsed += dt;
        // `bd.b(float)`, phase 0: the scale runs away from itself.
        self.scale += self.scale * dt * 3.0 * (self.scale + 0.3) / 0.5;
    }

    /// `bd`: past 15.2 the map takes over; the timer is only there so a hitch
    /// cannot leave the front end stuck in the jump.
    fn over(&self) -> bool {
        self.scale > space::SCALE_LIMIT || self.elapsed > 3.0
    }
}

/// A detached photo camera: Tab hands the viewport over, WASD moves,
/// R/F go up and down, the arrows look around.  Driving keys keep
/// working underneath, so pause (Esc) first for a static shot; the
/// eye/target print on toggle fits `dump_track` + `kora view --eye/--look`.
#[derive(Clone, Copy)]
struct FreeCam {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
}

impl FreeCam {
    fn forward(self) -> Vec3 {
        vec3(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }
}

/// A race in progress: the track, the cars and the running order.
struct Running {
    event: RaceEvent,
    back: Screen,
    track: scene::Track,
    sky: Option<sky::Sky>,
    geometry: scene::CarGeometry,
    texture: Option<Texture2D>,
    world: World,
    races: Vec<Race>,
    drivers: Vec<AiDriver>,
    player: usize,
    camera: Vec3,
    free: Option<FreeCam>,
    finish_order: Vec<usize>,
    outcome: Option<Outcome>,
    engine: Engine,
    last_throttle: f32,
    /// Start lights: the MIDlet counts `bt.a_F` up and holds the cars for
    /// the first three seconds while the screen shows the count.
    countdown: f32,
    /// "GO!" flash after the lights.
    go_flash: f32,
    /// Slideshow drift score (`help.txt`: points for drifting, zeroed by
    /// a wall touch).
    drift_score: f32,
    /// Seconds left on the clock for the time-boxed solo modes.
    time_left: Option<f32>,
    /// The offscreen the world is drawn into, and its size so it can be rebuilt
    /// when the window changes.
    target: Option<(RenderTarget, Vec2)>,
}

impl Running {
    /// Live standings: cars that have finished in their finishing order, then
    /// everyone else by race progress.
    fn standings(&self) -> Vec<usize> {
        let mut order = self.finish_order.clone();
        let mut rest: Vec<usize> = (0..self.world.cars.len())
            .filter(|car| !order.contains(car) && !self.races[*car].eliminated)
            .collect();
        rest.sort_by(|&a, &b| {
            let (pa, pb) = (
                self.races[a].progress(&self.track.grid, self.world.position(a)),
                self.races[b].progress(&self.track.grid, self.world.position(b)),
            );
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        });
        order.extend(rest);
        order
    }

    fn place_of(&self, car: usize) -> usize {
        self.standings().iter().position(|&c| c == car).unwrap_or(0)
    }

    /// Survival knock-out: eliminate the last placed running car. Finished
    /// cars are safe. Returns the player's outcome when the knock-out ends
    /// their race, or hands them the win when nobody is left to beat.
    fn eliminate_last(&mut self) -> Option<Outcome> {
        let victim = self
            .standings()
            .iter()
            .rev()
            .find(|&&car| !self.races[car].finished)
            .copied()?;
        self.races[victim].eliminated = true;
        // Zero time pays no award and keeps no record: a knock-out is a
        // loss, whatever the stopwatch says.
        if victim == self.player {
            let place = self
                .races
                .iter()
                .filter(|race| !race.eliminated)
                .count();
            self.outcome = Some(Outcome {
                place,
                gained: 0,
                total_time: 0.0,
                best_lap: self.races[self.player].best,
                laps: self.races[self.player].laps,
                cars: self.world.cars.len(),
                improved: false,
                previous_best: None,
            });
            return self.outcome.clone();
        }
        let rivals_left = (0..self.world.cars.len())
            .any(|car| car != self.player && !self.races[car].eliminated);
        if !rivals_left {
            let race = &self.races[self.player];
            let now = get_time();
            self.outcome = Some(Outcome {
                place: self.finish_order.len(),
                gained: 0,
                total_time: race.total_time(now),
                best_lap: race.best,
                laps: race.laps,
                cars: self.world.cars.len(),
                improved: false,
                previous_best: None,
            });
            return self.outcome.clone();
        }
        None
    }
}

/// Build a race from an event: track, textures, cars and race state.
#[allow(clippy::too_many_arguments)]
fn start_race(
    resources: &pack::Resources,
    dir: &PathBuf,
    event: &RaceEvent,
    car_file: &str,
    settings: &Settings,
    back: Screen,
    engine: Engine,
) -> Option<Running> {
    let detail = match settings.quality {
        kora::settings::Quality::Low => scene::Detail::Base,
        kora::settings::Quality::Medium => scene::Detail::Mid,
        kora::settings::Quality::High => scene::Detail::Full,
    };
    let mut track = scene::build_detailed(dir, resources, &event.map, event.theme, detail);
    // Placement audit trail: every tile, detail instance, barrier and grid
    // slot, so a misplaced rail can be traced to its cell, kind and arg.
    let log_name = format!("placements-{}.log", event.map);
    if let Err(error) = std::fs::write(&log_name, track.placement_log(&event.map, event.theme)) {
        eprintln!("placements log: {log_name}: {error}");
    } else {
        println!(
            "placements: {} instances, {} walls -> {log_name}",
            track.placements.len(),
            track.walls.len()
        );
    }
    track.attach_textures(resources);
    // The theme byte picks one of the five backgrounds (`al.q(j)`).
    let sky = sky::Sky::load(resources, event.theme);

    let car_def = resources
        .get(&format!("cars/{car_file}"))
        .and_then(|bytes| kora::format::Car::parse(bytes))?;
    // `cl.a(cf, boolean)` picks the model by the graphics detail setting -
    // `al.d() > 0 ? car.model : car.low_model` - and the two are very
    // different: `bonus` is 92 triangles and `bonus_low` is 48.  The port used
    // the high-detail body whatever the setting said, so at LOW it was drawing
    // a car the game would not have.
    let mut car_def = car_def;
    if settings.quality == kora::settings::Quality::Low {
        car_def.model = car_def.low_model.clone();
    }
    let mut geometry = scene::build_car(resources, &car_def)?;
    let texture = scene::load_car_texture(resources, &mut geometry);
    let laps = env_number("KORA_LAPS", 99).unwrap_or(event.laps).max(1);
    let opponents = env_number("KORA_OPPONENTS", 7).unwrap_or(event.opponents);

    let mut world = World::new(&track.walls);
    let mut player = 0;
    for (index, &(spot, yaw)) in track.grid.grid_slots(1 + opponents as usize).iter().enumerate() {
        // The player runs the player tune, opponents the AI tune; both read
        // the same four `.car` stat bytes (`PlayerTune`/`AiTune`).
        let tune = if index == 0 {
            Tuning::player(car_def.stats)
        } else {
            Tuning::ai(car_def.stats)
        };
        let car = world.add_car(spot, yaw, tune, index != 0);
        if index == 0 {
            player = car;
        }
    }

    // Race clocks start at the green light, not at boot: `Race::new` with a
    // zero base would bill the menus to the first lap.
    let now = get_time();
    let races: Vec<Race> = (0..world.cars.len())
        .map(|index| Race::new(&track.grid, laps, world.position(index), now))
        .collect();
    let drivers: Vec<AiDriver> = (0..world.cars.len())
        .map(|index| AiDriver::new(index))
        .collect();

    let camera = track.spawn + vec3(0.0, 5.0, 9.0);
    println!(
        "race: {} ({}) mode {} laps {} opponents {} theme {}",
        event.name, event.map, event.mode, laps, opponents, event.theme
    );

    Some(Running {
        event: event.clone(),
        back,
        track,
        sky,
        geometry,
        texture,
        world,
        races,
        drivers,
        player,
        camera,
        free: None,
        finish_order: Vec::new(),
        outcome: None,
        target: None,
        engine,
        last_throttle: 0.0,
        countdown: 3.0,
        go_flash: 0.0,
        drift_score: 0.0,
        time_left: event.time_limit,
    })
}

impl Running {
    fn restart(&mut self, laps: u32) {
        let now = get_time();
        for index in 0..self.world.cars.len() {
            self.world.reset(index);
            let position = self.world.position(index);
            self.races[index] = Race::new(&self.track.grid, laps, position, now);
        }
        self.finish_order.clear();
        self.outcome = None;
        self.free = None;
        self.engine.stop();
        self.camera = self.track.spawn + vec3(0.0, 5.0, 9.0);
        self.countdown = 3.0;
        self.go_flash = 0.0;
        self.drift_score = 0.0;
        self.time_left = self.event.time_limit;
    }

    /// Tab hands the viewport to a free photo camera (or back to the car).
    /// The eye/target print fits `dump_track` + `kora view --eye/--look`, so
    /// a framed shot can be re-rendered headlessly.
    fn toggle_free_cam(&mut self) {
        if let Some(free) = self.free {
            let target = free.pos + free.forward() * 3.0;
            println!(
                "free camera off: eye {:.1},{:.1},{:.1} look {:.1},{:.1},{:.1} fov 90",
                free.pos.x, free.pos.y, free.pos.z, target.x, target.y, target.z
            );
            self.free = None;
            set_cursor_grab(false);
            show_mouse(true);
            return;
        }
        let (_, rotation) = self.world.pose(self.player);
        let heading = rotation * vec3(0.0, 0.0, -1.0);
        let yaw = (-heading.x).atan2(-heading.z);
        let free = FreeCam {
            pos: self.camera,
            yaw,
            pitch: 0.0,
        };
        let target = free.pos + free.forward() * 3.0;
        println!(
            "free camera on: eye {:.1},{:.1},{:.1} look {:.1},{:.1},{:.1} fov 90 - mouse look, WASD move R/F up/down arrows look",
            free.pos.x, free.pos.y, free.pos.z, target.x, target.y, target.z
        );
        self.free = Some(free);
        // The mouse steers the camera, so it is captured here; the car
        // keeps none of the keys while the camera is out (see update).
        set_cursor_grab(true);
        show_mouse(false);
    }

    /// Mouse looks, WASD moves, R/F go up and down, the arrows look too.
    /// Shift is fast.
    fn update_free_cam(&mut self, dt: f32) {
        let Some(free) = &mut self.free else {
            return;
        };
        let mouse = mouse_delta_position();
        free.yaw -= mouse.x * 0.006;
        free.pitch = (free.pitch - mouse.y * 0.006).clamp(-1.45, 1.45);
        let turn = 1.8 * dt;
        if is_key_down(KeyCode::Left) {
            free.yaw += turn;
        }
        if is_key_down(KeyCode::Right) {
            free.yaw -= turn;
        }
        if is_key_down(KeyCode::Up) {
            free.pitch = (free.pitch + turn).min(1.45);
        }
        if is_key_down(KeyCode::Down) {
            free.pitch = (free.pitch - turn).max(-1.45);
        }
        let speed = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
            48.0
        } else {
            12.0
        } * dt;
        let forward = free.forward();
        let left = vec3(forward.z, 0.0, -forward.x).normalize_or_zero();
        if is_key_down(KeyCode::W) {
            free.pos += forward * speed;
        }
        if is_key_down(KeyCode::S) {
            free.pos -= forward * speed;
        }
        if is_key_down(KeyCode::A) {
            free.pos += left * speed;
        }
        if is_key_down(KeyCode::D) {
            free.pos -= left * speed;
        }
        if is_key_down(KeyCode::R) {
            free.pos.y += speed;
        }
        if is_key_down(KeyCode::F) {
            free.pos.y -= speed;
        }
    }

    /// Advance the race by one frame.  Returns the outcome once the player has
    /// finished.
    fn update(&mut self, dt: f32, laps: u32, settings: &Settings) -> Option<Outcome> {
        // The start lights hold everything for three seconds.
        if self.countdown > 0.0 {
            self.countdown -= dt;
            if self.countdown <= 0.0 {
                self.go_flash = 1.0;
            }
            return None;
        }
        if self.go_flash > 0.0 {
            self.go_flash -= dt;
        }
        let mut controls = vec![CarControl::default(); self.world.cars.len()];
        // The control scheme picks the keys; auto-throttle drives for you.
        // An eliminated survival car coasts with locked controls (`cl.l()`).
        // The free camera owns every key while it is out, so looking
        // around never steers the car; the player coasts meanwhile.
        if !self.races[self.player].eliminated && self.free.is_none() {
            let (accelerate, brake) = settings.scheme.throttle_keys();
            let (left, right) = settings.scheme.steer_keys();
            controls[self.player].throttle = if settings.auto_throttle || is_key_down(accelerate) {
                1.0
            } else if is_key_down(brake) {
                -0.6
            } else {
                0.0
            };
            self.last_throttle = controls[self.player].throttle.max(0.0);
            // Positive steering turns the wheels left (about +Y).
            controls[self.player].steer = if is_key_down(left) {
                1.0
            } else if is_key_down(right) {
                -1.0
            } else {
                0.0
            };
            controls[self.player].brake = is_key_down(KeyCode::Space);
        }

        for index in 0..self.world.cars.len() {
            if index == self.player || self.races[index].eliminated {
                continue;
            }
            // A hard wall hit holds the car briefly, the knockdown
            // recovery (`cx.m(float)`) lite.
            if self.world.wall_hit(index) && self.world.speed(index) > 8.0 {
                self.drivers[index].knock();
            }
            let (position, rotation) = self.world.pose(index);
            let heading = rotation * vec3(0.0, 0.0, -1.0);
            controls[index] = self.drivers[index].control(
                &self.track.grid,
                position,
                heading,
                self.world.speed(index),
                dt,
            );
        }

        // The MIDlet sets its car's height from the track's collision mesh
        // every frame, which is how it crosses the steps between tiles.
        let ride = self.geometry.half_extents.y + 0.02;
        let reach = self.geometry.half_extents.z + 0.5;
        let mut heights = Vec::with_capacity(self.world.cars.len());
        for index in 0..self.world.cars.len() {
            let (place, rotation) = self.world.pose(index);
            if place.y < -40.0 {
                self.world.reset(index);
                heights.push(None);
                continue;
            }
            let heading = rotation * vec3(0.0, 0.0, -1.0);
            heights.push(
                self.track
                    .surface
                    .support_height(place, heading, reach, place.y, ride)
                    .map(|height| height + ride),
            );
        }

        // Verges bog down: further than a road half-width plus margin off
        // the line, the car earns the off-road drag. Shoulders stay open
        // (cutting costs pace, it does not end the race); rails, walls
        // and cliffs do the stopping.
        let offroad: Vec<bool> = (0..self.world.cars.len())
            .map(|index| {
                let place = self.world.position(index);
                match self.track.grid.progress_at(place) {
                    None => true,
                    Some(f) => match self.track.grid.line_point(f, 0.0) {
                        None => true,
                        Some(line) => (place.x - line.x).hypot(place.z - line.z) > 5.0,
                    },
                }
            })
            .collect();
        let substeps = ((dt / (1.0 / 60.0)).ceil() as i32).clamp(1, 4);
        for _ in 0..substeps {
            self.world.step(dt / substeps as f32, &controls, &heights, &offroad);
        }

        let mode = Mode::from_u8(self.event.mode);
        let laps_before: Vec<u32> = self.races.iter().map(|race| race.lap).collect();
        let now = get_time();
        for index in 0..self.world.cars.len() {
            let before = self.races[index].finished;
            self.races[index].update(now, &self.track.grid, self.world.position(index));
            if self.races[index].finished && !before {
                self.finish_order.push(index);
            }
        }

        // Slideshow: slides pay while the clock runs, and a wall touch
        // zeroes the counter.
        if mode == Mode::Slideshow && !self.races[self.player].finished {
            if self.world.wall_hit(self.player) {
                self.drift_score = 0.0;
            } else {
                self.drift_score += drift_points(
                    self.world.slide(self.player),
                    self.world.speed(self.player),
                    dt,
                );
            }
        }

        // Survival: each completed lap knocks out the last placed car.
        if mode == Mode::Survival
            && self.outcome.is_none()
            && laps_before.iter().zip(self.races.iter()).any(|(before, race)| {
                !race.eliminated && race.lap > *before
            })
        {
            if let Some(outcome) = self.eliminate_last() {
                return Some(outcome);
            }
        }

        // Time chase and special: the clock runs out or the laps get done.
        if matches!(mode, Mode::TimeChase | Mode::Special)
            && self.outcome.is_none()
            && !self.races[self.player].finished
        {
            if let Some(left) = self.time_left.as_mut() {
                *left -= dt;
                if *left <= 0.0 {
                    // The clock beat the car: last place, no award, no
                    // record. The results screen shows the loss as it is.
                    let race = &self.races[self.player];
                    // Zero time pays no award and keeps no record.
                    self.outcome = Some(Outcome {
                        place: self.world.cars.len().saturating_sub(1),
                        gained: 0,
                        total_time: 0.0,
                        best_lap: race.best,
                        laps,
                        cars: self.world.cars.len(),
                        improved: false,
                        previous_best: None,
                    });
                    return self.outcome.clone();
                }
            }
        }

        // Slideshow: the clock ends the run and the score is the result.
        if mode == Mode::Slideshow
            && self.outcome.is_none()
            && !self.races[self.player].finished
        {
            if let Some(left) = self.time_left.as_mut() {
                *left -= dt;
                if *left <= 0.0 {
                    let race = &self.races[self.player];
                    self.outcome = Some(Outcome {
                        place: 0,
                        gained: self.drift_score as u32,
                        total_time: race.total_time(now),
                        best_lap: race.best,
                        laps,
                        cars: self.world.cars.len(),
                        improved: false,
                        previous_best: None,
                    });
                    return self.outcome.clone();
                }
            }
        }

        {
            let state = self.world.engine(self.player);
            let throttle = self.last_throttle;
            self.engine.update(state, throttle, settings.volume);
        }

        if self.outcome.is_none() && self.races[self.player].finished {
            let place = self
                .finish_order
                .iter()
                .position(|&car| car == self.player)
                .unwrap_or(0);
            let race = &self.races[self.player];
            self.outcome = Some(Outcome {
                place,
                gained: 0,
                total_time: race.finish_time.unwrap_or(0.0),
                best_lap: race.best,
                laps,
                cars: self.world.cars.len(),
                improved: false,
                previous_best: None,
            });
            return self.outcome.clone();
        }
        None
    }

    fn draw_world(&mut self, dt: f32, settings: &Settings) {
        let (position, rotation) = self.world.pose(self.player);
        let forward = rotation * vec3(0.0, 0.0, -1.0);
        let up = vec3(0.0, 1.0, 0.0);
        let (back, height) = settings.camera.placement();
        let desired = position - forward * back + up * height;
        // Rails and walls stop the camera, not just the car: pull the
        // viewpoint in front of whatever the segment crosses so it never
        // slices through a rail it just watched the car hit.
        let pull = scene::camera_pull_in(&self.track.walls, desired, position);
        let desired = desired.lerp(position, 1.0 - pull);
        self.camera = self.camera.lerp(desired, (dt * 5.0).min(1.0));

        // The world is drawn at the height the MIDlet drew it at and scaled up,
        // because the artwork was made for that: a track tile carries about a
        // hundred pixels of texture, so on a 720-line window one near tile is
        // magnified four to eight times and reads as a mosaic of its own texels.
        // The HUD and the menus stay at the window's own resolution, where text
        // belongs.
        let target = self.world_target();

        let mut backdrop = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
        ));
        backdrop.render_target = Some(target.clone());
        set_camera(&backdrop);
        // The sky is a 2D backdrop, the way M3G draws a `Background` into the
        // viewport before the scene, so it belongs under a screen camera.
        // Drawing it after the 3D one would drop the whole image into the world
        // as a quad standing at the origin instead.
        match &self.sky {
            Some(sky) if settings.background => sky.draw(),
            _ => clear_background(Color::new(0.53, 0.81, 0.92, 1.0)),
        }

        let mut camera = Camera3D::default();
        camera.render_target = Some(target.clone());
        if let Some(free) = self.free {
            camera.position = free.pos;
            camera.target = free.pos + free.forward() * 3.0;
        } else {
            camera.position = self.camera;
            camera.target = position + forward * 3.0 + up * 0.8;
        }
        camera.up = up;
        // The game's own field of view: `bq.a` sets the M3G perspective to a 90
        // degree vertical FOV for every view.  A narrower one magnifies the near
        // field far more than the original does - a track tile carries about a
        // hundred pixels of artwork, and the game drew it at 240 lines.
        camera.fovy = 90f32.to_radians();
        camera.z_far = settings.visibility.far_plane();
        set_camera(&camera);

        for mesh in &self.track.meshes {
            draw_mesh(mesh);
        }
        for index in 0..self.world.cars.len() {
            // From inside you sit in the car; drawing its shell would put the
            // camera behind a wall of polygons.
            if settings.camera == kora::settings::Camera::Inside && index == self.player {
                continue;
            }
            let (place, spin) = self.world.pose(index);
            let mut vertices = self.geometry.vertices.clone();
            for vertex in &mut vertices {
                vertex.position = spin * vertex.position + place;
            }
            draw_mesh(&Mesh {
                vertices,
                indices: self.geometry.indices.clone(),
                texture: self.texture.clone(),
            });
        }

        set_default_camera();
        draw_texture_ex(
            &target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                // macroquad's render targets come out upside down when drawn
                // back: the framebuffer's origin is the bottom left.
                flip_y: true,
                ..Default::default()
            },
        );
    }

    /// The offscreen the world is drawn into: the window's shape, but 240 lines
    /// tall, which is what the MIDlet rendered into.  Rebuilt when the window
    /// changes size, and kept between frames because the sky and the meshes are
    /// drawn into it every frame.  `KORA_RES` overrides the height outright -
    /// KEmulator draws at desktop resolution, so a larger target (say 586)
    /// with `KORA_SMOOTH=1` is what gets the port looking like the emulator
    /// instead of a 2008 phone.
    fn world_target(&mut self) -> RenderTarget {
        let height = std::env::var("KORA_RES")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|&value| value >= 64.0)
            .unwrap_or(240.0);
        let width = (screen_width() * height / screen_height()).max(64.0).round();
        if let Some((target, size)) = &self.target {
            if size.x == width {
                return target.clone();
            }
        }
        // `render_target` is documented as a render target with **no depth
        // buffer**, and this one needs one: without depth testing every
        // triangle is painted in submission order, so a car's far side and its
        // own interior show through its near side - "four wheels a side" - and
        // the inside of every track slab shows through the road it is made of.
        // The game has a depth buffer; so does this now.
        let target = render_target_ex(
            width as u32,
            height as u32,
            RenderTargetParams {
                sample_count: 1,
                depth: true,
            },
        );
        // Smooth, because the point of the small target is to stop the artwork
        // reading as a grid of blocks; the original's phones had no choice.
        target.texture.set_filter(FilterMode::Linear);
        self.target = Some((target.clone(), vec2(width, height)));
        target
    }

    fn draw_hud(&self, now: f64, laps: u32, settings: &Settings) {
        if !settings.hud {
            return;
        }
        let race = &self.races[self.player];
        let place = self.place_of(self.player) + 1;
        text::draw_shadow(&self.event.name, 16.0, 14.0, 28.0, WHITE);
        text::draw_shadow(
            &labels::format(
                "hud_lap",
                &[&(race.lap + 1).min(laps).to_string(), &laps.to_string()],
            ),
            16.0,
            50.0,
            28.0,
            WHITE,
        );
        text::draw_shadow(
            &labels::format(
                "hud_pos",
                &[&place.to_string(), &self.world.cars.len().to_string()],
            ),
            16.0,
            86.0,
            28.0,
            WHITE,
        );
        text::draw_shadow(
            &labels::format("hud_time", &[&menu::format_time(race.total_time(now))]),
            16.0,
            122.0,
            28.0,
            WHITE,
        );
        // The start lights count down over the frozen scene, then the flag.
        if self.countdown > 0.0 {
            let count = self.countdown.ceil().max(1.0) as u32;
            let text = count.to_string();
            let size = 96.0;
            let width = screen_width();
            text::draw_shadow(&text, width / 2.0 - 20.0, 200.0, size, RED);
        } else if self.go_flash > 0.0 {
            let width = screen_width();
            text::draw_shadow("GO!", width / 2.0 - 60.0, 200.0, 96.0, GREEN);
        }
        let mode = Mode::from_u8(self.event.mode);
        // The solo clock ticks away in the corner, like `Countdown`.
        if matches!(mode, Mode::TimeChase | Mode::Special | Mode::Slideshow) {
            if let Some(left) = self.time_left {
                text::draw_shadow(
                    &labels::format("hud_time_left", &[&menu::format_time(left.max(0.0))]),
                    16.0,
                    194.0,
                    28.0,
                    Color::new(1.0, 0.4, 0.3, 1.0),
                );
            }
        }
        if mode == Mode::Slideshow {
            text::draw_shadow(
                &labels::format("hud_drift", &[&format!("{:.0}", self.drift_score)]),
                16.0,
                230.0,
                28.0,
                Color::new(1.0, 0.85, 0.2, 1.0),
            );
        }
        let best = race
            .best
            .map(menu::format_time)
            .unwrap_or_else(|| labels::get("no_time").to_string());
        text::draw_shadow(
            &labels::format(
                "hud_lap_best",
                &[&menu::format_time(race.lap_time(now)), &best],
            ),
            16.0,
            158.0,
            28.0,
            Color::new(1.0, 0.85, 0.2, 1.0),
        );
        text::draw_shadow(
            labels::get("keys_race"),
            16.0,
            screen_height() - 40.0,
            20.0,
            Color::new(0.9, 0.9, 0.9, 1.0),
        );
        if self.free.is_some() {
            text::draw_shadow(
                "TAB CAM  WASD MOVE  R/F UP/DOWN  ARROWS LOOK",
                16.0,
                screen_height() - 64.0,
                20.0,
                Color::new(1.0, 0.85, 0.2, 1.0),
            );
        }

        let markers: Vec<hud::Marker> = (0..self.world.cars.len())
            .map(|car| {
                let (position, rotation) = self.world.pose(car);
                hud::Marker {
                    position,
                    heading: rotation * vec3(0.0, 0.0, -1.0),
                    player: car == self.player,
                }
            })
            .collect();
        hud::draw_minimap(&self.track.grid, &markers);
        hud::draw_speedometer(self.world.speed(self.player));
    }
}

/// Render one car on a turntable in the right half of the screen.
///
/// The camera orbits a car centred on its own origin, and the viewport keeps it
/// off the list.  `Camera3D` works its aspect out from the whole window, so a
/// half-width viewport has to be told its own.
fn draw_showroom(geometry: &scene::CarGeometry, texture: &Option<Texture2D>, degrees: f32) {
    let (left, top) = (screen_width() * 0.46, 40.0);
    let width = screen_width() - left - 16.0;
    let height = screen_height() - top - 60.0;
    if width < 32.0 || height < 32.0 {
        return;
    }

    // The MIDlet's own preview is a low, near-level three-quarter view: `bd`
    // draws the car through `postTranslate(0, -0.55, -1.9)` with 180 degrees
    // about Y, 90 about X and 125 about Z - a car seen from its own height, not
    // from above.  Looking down on it, as this did, shows the far pair of
    // wheels through the open wheel arches, which reads as a car with too many
    // wheels when the original is showing two.
    let angle = degrees.to_radians();
    let distance = 2.4;
    let mut camera = Camera3D::default();
    camera.position = vec3(distance * angle.sin(), 0.62, distance * angle.cos());
    camera.target = vec3(0.0, 0.22, 0.0);
    camera.up = vec3(0.0, 1.0, 0.0);
    camera.fovy = 45f32.to_radians();
    camera.aspect = Some(width / height);
    camera.viewport = Some((left as i32, top as i32, width as i32, height as i32));
    camera.z_far = 60.0;
    set_camera(&camera);

    draw_mesh(&Mesh {
        vertices: geometry.vertices.clone(),
        indices: geometry.indices.clone(),
        texture: texture.clone(),
    });
    set_default_camera();
}

#[macroquad::main("K.O. Racing 3D - Rust port")]
async fn main() {
    let dir = assets_dir();
    println!("resource pack: {}", dir.display());
    let resources = pack::load(&dir);
    println!("  {} resources indexed", resources.len());

    let cars = progress::car_infos(&resources);
    // The two career tables are listed separately, the way the original has
    // one screen for the campaign and another for the deluxe levels.  Unlike
    // the original, the deluxe list is gated by career points rather than by
    // an SMS purchase: nothing here needs a server or a payment.
    let all_events = campaign::events(&resources);
    let career: Vec<RaceEvent> = all_events
        .iter()
        .filter(|event| event.table.ends_with("campaign"))
        .cloned()
        .collect();
    let deluxe: Vec<RaceEvent> = all_events
        .iter()
        .filter(|event| event.table.ends_with("deluxe"))
        .cloned()
        .collect();
    let quick = campaign::quick_events(&resources);
    let settings_path = match std::env::var("KORA_SETTINGS") {
        Ok(path) => PathBuf::from(path),
        Err(_) => paths::file(paths::Dir::Config, "settings.txt"),
    };
    let mut settings = Settings::load(&settings_path);
    println!("settings: {}", settings_path.display());

    // The showroom draws the picked car in 3D, the way `u.j()` renders
    // `bd.a.a(car, angle)` through a transform while the panel spins at ten
    // degrees a second.  Display only: nothing here can be bought.
    let showroom: Vec<(scene::CarGeometry, Option<Texture2D>)> = cars
        .iter()
        .filter_map(|car| {
            let mut definition =
                format::Car::parse(resources.get(&format!("cars/{}", car.file))?)?;
            if settings.quality == kora::settings::Quality::Low {
                definition.model = definition.low_model.clone();
            }
            let mut geometry = scene::build_car(&resources, &definition)?;
            let texture = scene::load_car_texture(&resources, &mut geometry);
            Some((geometry, texture))
        })
        .collect();
    println!("  {} cars in the showroom", showroom.len());
    println!(
        "  {} cars, {} career events, {} deluxe events, {} quick-race tracks",
        cars.len(),
        career.len(),
        deluxe.len(),
        quick.len()
    );

    // The one sound the game ships is a MIDI file at the JAR root, not in the
    // resource pack, so `setup.sh` puts it beside the pack.  macroquad cannot
    // play MIDI, so it is rendered here and handed over as a WAV.
    if std::env::var("KORA_MUSIC").map(|v| v == "0").unwrap_or(false) {
        settings.music = false;
    }
    let theme = std::fs::read(dir.join("sounds/theme.mid")).ok();
    let now = || std::time::Instant::now();
    let started = now();
    let track: Option<Sound> = match theme.as_deref().map(music::render) {
        Some(Some(wav)) => {
            println!(
                "  theme rendered to {:.1} s of audio in {:?}",
                wav.len() as f32 / (music::SAMPLE_RATE as f32 * 2.0),
                started.elapsed()
            );
            load_sound_from_bytes(&wav).await.ok()
        }
        _ => {
            eprintln!("  no sounds/theme.mid beside the pack, running silent");
            None
        }
    };
    let mut music_playing = false;
    if settings.music {
        if let Some(track) = &track {
            play_sound(
                track,
                PlaySoundParams {
                    looped: true,
                    volume: settings.volume,
                },
            );
            music_playing = true;
        }
    }

    let theme = theme::build();

    let save_path = match std::env::var("KORA_SAVE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => paths::file(paths::Dir::Data, "save.txt"),
    };
    let mut progress = Progress::load(&save_path);
    println!("progress: {}", save_path.display());
    if let Ok(name) = std::env::var("KORA_CAR") {
        if let Some(index) = cars.iter().position(|car| format!("cars/{}", car.file) == name) {
            progress.car = index;
        }
    }

    // `KORA_SKIP_MENU=1` boots straight into a race, which is handy from a
    // shell and keeps the environment overrides meaningful.
    let mut screen = Screen::Main;
    let mut cursor = 0usize;
    // The front end's backdrop (the Earth, and the stars the jump scatters),
    // the bar its entries sit in, and the map a jump lands on.
    let mut space = space::Space::new(&resources, screen_width(), screen_height());
    let mut bar = menu::Bar::new(0);
    let mut jump: Option<Jump> = None;
    let mut map_screen: Option<map::MapScreen> = None;
    // The career tables with their level lists, which is what the maps need:
    // a level's marker position, its name and whether it is open.
    let tables = campaign::load(&resources);
    let table_at = |index: usize| -> Option<(String, Vec<RaceEvent>)> {
        let (name, _) = tables.get(index)?;
        let events: Vec<RaceEvent> = all_events
            .iter()
            .filter(|event| &event.table == name)
            .cloned()
            .collect();
        Some((name.clone(), events))
    };
    let mut showroom_spin = 0.0f32;
    let mut confirming_reset = false;
    let mut running: Option<Running> = None;
    let mut message = String::new();

    if std::env::var("KORA_SKIP_MENU").is_ok() {
        let wanted = std::env::var("KORA_MAP").ok();
        let event = wanted
            .as_ref()
            .and_then(|map| quick.iter().find(|event| &event.map == map))
            .or_else(|| quick.first())
            .cloned();
        if let Some(event) = event {
            let file = cars
                .get(progress.car)
                .map(|car| car.file.clone())
                .unwrap_or_else(|| "rally.car".to_string());
            running = start_race(&resources, &dir, &event, &file, &settings, Screen::Quick, Engine::load(&dir).await);
            if running.is_some() {
                screen = Screen::Race;
            }
        }
    }

    loop {
        let dt = get_frame_time().min(0.05);
        clear_background(theme::BACKDROP);

        if let Some(track) = &track {
            if is_key_pressed(KeyCode::M) {
                settings.music = !settings.music;
                settings.save(&settings_path);
                music_playing = settings.music;
                if !music_playing {
                    macroquad::audio::stop_sound(track);
                }
            }
            // The settings screen owns the volume; M and the row both feed it.
            if settings.music && !music_playing {
                play_sound(
                    track,
                    PlaySoundParams {
                        looped: true,
                        volume: settings.volume,
                    },
                );
                music_playing = true;
            }
            set_sound_volume(track, if music_playing { settings.volume } else { 0.0 });
        }

        // The jump runs on its own: the map is built when the planet has
        // swallowed the view, which is where `bd.b(float)` changes screen.
        if let Some(active) = jump.as_mut() {
            active.update(dt);
            space.update(dt, active.scale - 0.4, active.scale);
            space.draw();
            if active.over() {
                let index = active.table;
                map_screen = table_at(index).and_then(|(name, events)| {
                    let (_, campaign) = tables.get(index)?;
                    let title = match index {
                        0 => labels::get("menu_career").to_string(),
                        _ => labels::get("menu_deluxe").to_string(),
                    };
                    map::MapScreen::new(
                        &resources,
                        &name,
                        title,
                        &events,
                        &campaign.levels,
                        &progress,
                    )
                });
                cursor = 0;
                screen = if map_screen.is_some() {
                    Screen::Map(index)
                } else {
                    // A table with nothing on its map keeps its list screen.
                    if index == 0 {
                        Screen::Career
                    } else {
                        Screen::Deluxe
                    }
                };
                jump = None;
            }
            next_frame().await;
            continue;
        }

        match screen {
            Screen::Main => {
                if let menu::Action::Activate(choice) =
                    menu::bar_menu(&resources, &mut space, &mut bar, &progress, &mut cursor)
                {
                    message.clear();
                    match choice {
                        0 | 1 => {
                            // CAREER and DELUXE open the map through the jump;
                            // the MIDlet's front end does the same.
                            cursor = 0;
                            jump = Some(Jump::new(choice));
                        }
                        2 => {
                            cursor = 0;
                            screen = Screen::Quick;
                        }
                        3 => {
                            cursor = progress.car.min(cars.len().saturating_sub(1));
                            screen = Screen::Cars;
                        }
                        4 => {
                            cursor = 0;
                            screen = Screen::Options;
                        }
                        _ => break,
                    }
                }
                if !message.is_empty() {
                    text::draw_shadow(&message, 16.0, 12.0, 19.0, WHITE);
                }
            }

            Screen::Map(index) => {
                let Some(active) = map_screen.as_mut() else {
                    screen = Screen::Main;
                    next_frame().await;
                    continue;
                };
                active.update(dt);
                let (action, _) = active.input();
                active.draw(&progress);
                match action {
                    map::MapAction::Back => {
                        map_screen = None;
                        cursor = 0;
                        screen = Screen::Main;
                    }
                    map::MapAction::Start => {
                        if let Some(event) = active.event().cloned() {
                            if !progress.open(event.threshold) {
                                message = labels::format(
                                    "race_needs",
                                    &[&event.name.to_uppercase(), &event.threshold.to_string()],
                                );
                            } else {
                                let file = cars
                                    .get(progress.car)
                                    .map(|car| car.file.clone())
                                    .unwrap_or_else(|| "rally.car".to_string());
                                running = start_race(
                                    &resources,
                                    &dir,
                                    &event,
                                    &file,
                                    &settings,
                                    Screen::Map(index),
                                    Engine::load(&dir).await,
                                );
                                if running.is_some() {
                                    message.clear();
                                    screen = Screen::Race;
                                }
                            }
                        }
                    }
                    map::MapAction::None => {}
                }
            }

            Screen::Options => {
                match menu::options(&theme, &mut settings, &mut cursor, &mut confirming_reset) {
                    menu::OptionsAction::ResetCareer => {
                        progress = Progress::default();
                        progress.save(&save_path);
                        message = labels::get("reset_complete").to_string();
                    }
                    menu::OptionsAction::Back => {
                        settings.save(&settings_path);
                        cursor = 0;
                        screen = Screen::Main;
                    }
                    menu::OptionsAction::None => {}
                }
            }

            Screen::Career | Screen::Deluxe | Screen::Quick => {
                let (events, title) = match screen {
                    Screen::Career => (&career, labels::get("menu_career")),
                    Screen::Deluxe => (&deluxe, labels::get("menu_deluxe")),
                    _ => (&quick, labels::get("menu_quick")),
                };
                if events.is_empty() {
                    screen = Screen::Main;
                } else {
                    match menu::event_list(&theme, &progress, events, &mut cursor, title) {
                        menu::Action::Activate(index) => {
                            let event = events[index].clone();
                            if !progress.open(event.threshold) {
                                message = labels::format(
                                    "race_needs",
                                    &[&event.name.to_uppercase(), &event.threshold.to_string()],
                                );
                            } else {
                                let file = cars
                                    .get(progress.car)
                                    .map(|car| car.file.clone())
                                    .unwrap_or_else(|| "rally.car".to_string());
                                running = start_race(&resources, &dir, &event, &file, &settings, screen, Engine::load(&dir).await);
                                if running.is_some() {
                                    screen = Screen::Race;
                                } else {
                                    message = labels::format(
                                        "load_failed",
                                        &[&event.map.to_uppercase()],
                                    );
                                }
                            }
                        }
                        menu::Action::Back => {
                            cursor = 0;
                            screen = Screen::Main;
                        }
                        menu::Action::None => {}
                    }
                }
            }

            Screen::Cars => {
                if cars.is_empty() {
                    screen = Screen::Main;
                } else {
                    cursor = cursor.min(cars.len() - 1);
                    // Spin at the game's own rate: `u.b(float)` advances its
                    // angle by ten degrees a second and wraps at 360.
                    showroom_spin = (showroom_spin + 10.0 * dt) % 360.0;
                    if let Some((geometry, texture)) = showroom.get(cursor) {
                        draw_showroom(geometry, texture, showroom_spin);
                    }
                    match menu::car_list(&theme, &cars, &progress, &mut cursor) {
                        menu::Action::Activate(index) => {
                            progress.car = index;
                            progress.save(&save_path);
                            message = labels::format("car_selected", &[&cars[index].name]);
                            cursor = 0;
                            screen = Screen::Main;
                        }
                        menu::Action::Back => {
                            cursor = 0;
                            screen = Screen::Main;
                        }
                        menu::Action::None => {}
                    }
                }
            }

            Screen::Race => {
                let Some(run) = running.as_mut() else {
                    screen = Screen::Main;
                    continue;
                };
                if is_key_pressed(KeyCode::Escape) {
                    cursor = 0;
                    screen = Screen::Paused;
                } else {
                    if is_key_pressed(KeyCode::Tab) {
                        run.toggle_free_cam();
                    }
                    run.update_free_cam(dt);
                    let laps = env_number("KORA_LAPS", 99).unwrap_or(run.event.laps).max(1);
                    let finished = run.update(dt, laps, &settings);
                    run.draw_world(dt, &settings);
                    run.draw_hud(get_time(), laps, &settings);
                    if let Some(outcome) = finished {
                        // Keep the time if it beats the record, and pay the
                        // record's award the first time a race is passed.
                        // Slideshow pays its drift score as points on top,
                        // every run.
                        let slideshow =
                            Mode::from_u8(run.event.mode) == Mode::Slideshow;
                        let score = run.drift_score as u32;
                        let result =
                            progress.record(&run.event.key, outcome.total_time, run.event.award);
                        if slideshow {
                            progress.points += score;
                        }
                        progress.save(&save_path);
                        if slideshow {
                            if let Some(run) = running.as_mut() {
                                if let Some(outcome) = run.outcome.as_mut() {
                                    outcome.gained += score;
                                }
                            }
                        }
                        if let Some(run) = running.as_mut() {
                            if let Some(outcome) = run.outcome.as_mut() {
                                outcome.gained = result.gained;
                                outcome.improved = result.improved;
                                outcome.previous_best = result.previous_best;
                            }
                        }
                        screen = Screen::Results;
                    }
                }
            }

            Screen::Paused => {
                let music = track.as_ref().map(|_| music_playing);
                match menu::pause_menu(&theme, &mut cursor, music) {
                    menu::Action::Activate(0) => screen = Screen::Race,
                    menu::Action::Activate(1) => {
                        if let Some(run) = running.as_mut() {
                            let laps =
                                env_number("KORA_LAPS", 99).unwrap_or(run.event.laps).max(1);
                            run.restart(laps);
                        }
                        screen = Screen::Race;
                    }
                    menu::Action::Activate(_) => {
                        running = None;
                        cursor = 0;
                        screen = Screen::Main;
                    }
                    _ => {}
                }
            }

            Screen::Results => {
                let action = match running.as_ref() {
                    Some(run) => menu::results(
                        &theme,
                        &run.event,
                        &progress,
                        run.outcome.as_ref().expect("an outcome"),
                    ),
                    None => menu::Action::Back,
                };
                if !matches!(action, menu::Action::None) {
                    let back = running.as_ref().map(|run| run.back).unwrap_or(Screen::Main);
                    running = None;
                    cursor = 0;
                    screen = back;
                }
            }
        }

        next_frame().await;
    }
}
