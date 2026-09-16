//! The game's own vehicle model, without rapier3d.
//!
//! The port used to drive rapier3d raycast vehicles with an invented mapping
//! from the `.car` stat bytes. The game does nothing of the sort: `RaceCar`
//! (`java/GenObfCl.java`, class `cl`) integrates a planar arcade model every
//! frame - heading from yaw, throttle and brake rates from the tune tables,
//! a speed-sensitive steering lock, quadratic drag, car-to-car shunts with a
//! drafting boost - and sets its height from the track collision mesh, so a
//! car can never beach itself or land on its roof. This module ports that
//! model 1:1 where the mapping is proven and says so where it is not.
//!
//! Traced constants (all from `java/PlayerTune.java`, `java/AiTune.java` and
//! `java/GenObfCl.java` unless noted):
//!
//! * throttle adds `h * dt` to speed and clamps it to `l()`
//!   (`cl.f(float)`, `PlayerTune.l()`); brake subtracts `h * dt` and floors
//!   at the idle creep `m` (`cl.g(float)`).
//! * steering rate is the tune `j` (20/s), scaled by
//!   `1.4 - 0.4 * |steer|` while gripping (`cl.n(float)`/`cl.o(float)`).
//! * steering lock is `min(o, n / max(speed, 0.5))` (`cl.n(float)`).
//! * drag is quadratic in the tune `r`/`s` (`GenObfCl.java` `c(float)`,
//!   `k_bz` term); the low-speed lateral cutoff is 0.2 (`c(float)`).
//! * a shunt fires below 0.9 units with 1.0/0.1 push factors, and a car
//!   close behind a faster one gets a 0.25 draft term (`cl.a(ObfCl,Z)`).
//! * the tune tables are near-constant: AI differs from player only in `i`
//!   (1.047 vs 0.628); the per-car bytes `u,v,a,b` are the `.car` stats
//!   themselves (`PlayerTune.a(InputStream)`, `StreamReader.a__int` reads
//!   one byte).
//!
//! Approximations, all marked below: the yaw-rate averager chain
//! (`di.e`/`a_z` drift state, whose feed `di.c__void` is dead code in the
//! shipped game) is replaced by the direct relation `yaw_rate =
//! clamp(steer * speed, +-1.5)`, which keeps the verbatim lock law and the
//! verbatim +-1.5 authority cap; the throttle-to-velocity coupling the
//! static trace cannot find is a chase of the capped drive state at
//! ~1/s (reaches cruise in about three seconds, brakes bite at ~4/s)
//! instead of the game's unknown rate - full throttle for a second must
//! not put the car at cruise, and a test pins that; wheel spin/slip
//! visuals (`cl.a(float)`) and the suspension settle (`cl.k(float)`) are
//! not simulated; tilt comes from the surface normal. The `.car` tail
//! past the four stat bytes feeds the menus, not the car.

use macroquad::prelude::{vec3, Quat, Vec3};

/// Player and AI controls, normalised to `-1..1`.
#[derive(Clone, Copy, Default)]
pub struct CarControl {
    pub throttle: f32,
    pub steer: f32,
    pub brake: bool,
}

/// The tune tables (`CarPhysics` + `PlayerTune`/`AiTune`/`StockTune`).
///
/// Field names are the game's (`a`..`v`); see the module docs for what the
/// port actually uses. Values below are the constructor defaults both tunes
/// share unless noted.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    /// `a`: base factor, always `b + c`.
    pub a: f32,
    /// `b`, `c`: unit factors.
    pub b: f32,
    pub c: f32,
    /// `d`, `e`: mass-like terms (1500).
    pub d: f32,
    pub e: f32,
    /// `f`: `d * 9.8 * 0.5`.
    pub f: f32,
    /// `g`: tire grip scale (150).
    pub g: f32,
    /// `h`: throttle/brake rate factor (0.5).
    pub h: f32,
    /// `i`: steering response (0.628 player, 1.047 AI).
    pub i: f32,
    /// `k`: speed cap scale (80).
    pub k: f32,
    /// `j`: steering rate (20/s) and throttle base.
    pub j: f32,
    /// `l`: idle floor helper (-30).
    pub l: f32,
    /// `m`: idle creep speed (0.785).
    pub m: f32,
    /// `n`: steering lock numerator (1.571).
    pub n: f32,
    /// `o`: steering lock cap (60).
    pub o: f32,
    /// `p`: brake rate (30).
    pub p: f32,
    /// `q`: steering centering rate (2, doubled in mode 2).
    pub q: f32,
    /// `r`, `s`: quadratic drag pair (-6, -3).
    pub r: f32,
    pub s: f32,
    /// `t`: 1.3 (draft/wobble helper).
    pub t: f32,
    /// `u`, `v`: per-car `.car` stat bytes 0-1 as floats.
    pub u: f32,
    pub v: f32,
    /// `ia`, `ib`: per-car `.car` stat bytes 2-3 as integers
    /// (`aw.a` writes the int fields `a`/`b`, not the float ones, so the
    /// float `a`/`b` keep their 2.0/1.0 defaults for every car).
    pub ia: i32,
    pub ib: i32,
}

impl Tuning {
    fn shared() -> Tuning {
        Tuning {
            a: 2.0,
            b: 1.0,
            c: 1.0,
            d: 1500.0,
            e: 1500.0,
            f: 1500.0 * 9.8 * 0.5,
            g: 150.0,
            h: 0.5,
            i: 0.628_318_55,
            k: 80.0,
            j: 20.0,
            l: -30.0,
            m: 0.785_398_2,
            n: 1.570_796_4,
            o: 60.0,
            p: 30.0,
            q: 2.0,
            r: -6.0,
            s: -3.0,
            t: 1.3,
            u: 0.0,
            v: 0.0,
            ia: 0,
            ib: 0,
        }
    }

    /// The player tune: shared constants plus the four `.car` stat bytes
    /// (`PlayerTune.a(InputStream)` reads exactly four).
    pub fn player(stats: [u8; 4]) -> Tuning {
        let mut tune = Tuning::shared();
        tune.u = stats[0] as f32;
        tune.v = stats[1] as f32;
        tune.ia = stats[2] as i32;
        tune.ib = stats[3] as i32;
        tune
    }

    /// The opponent tune: identical except the steering response
    /// (`AiTune` constructor).
    pub fn ai(stats: [u8; 4]) -> Tuning {
        let mut tune = Tuning::player(stats);
        tune.i = 1.047_197_6;
        tune
    }

    /// The base tune (`StockTune`): unit factors only.
    pub fn stock() -> Tuning {
        Tuning::shared()
    }

    /// Backwards-compatible pick from the `.car` stats, as before.
    pub fn from_stats(stats: [u8; 4]) -> Tuning {
        Tuning::player(stats)
    }

    /// Top speed (`PlayerTune.l()`), with the class and mode made explicit.
    ///
    /// The game reads both from its settings (`Settings.c` is the class,
    /// `Settings.m` the mode); the port runs class 1 outside mode 2.
    pub fn top_speed(&self, klass: u8, mode2: bool) -> f32 {
        // v1 = 0.2 and v2 = c here (the c==0/c==1 rescalings do not fire).
        if mode2 {
            // `((14 - min(v2, 4)) * (2 * v2 + 5) / 7) * k / 20`.
            let v2 = self.c;
            ((14.0 - v2.min(4.0)) * (v2 * 2.0 + 5.0) / 7.0) * self.k / 20.0
        } else {
            // `((0.75 + klass * v1) * (v2 + 6) / 7) * k`.
            ((0.75 + klass as f32 * 0.2) * (self.c + 6.0) / 7.0) * self.k
        }
    }

    /// Throttle rate (`PlayerTune.h()` / `AiTune.h()`).
    ///
    /// `klass` is the game's `Settings.c` (class 1 here), `mode` its
    /// `Settings.m`; the AI form ignores both.
    pub fn throttle_rate(&self, klass: u8, mode: u8, ai: bool) -> f32 {
        if ai {
            return ((0.2 + 0.75) * (2.0 * self.c + 5.0) / 7.0) * self.g;
        }
        if mode == 2 {
            return (((2.0 * self.c + 5.0) / 7.0) * self.g) / 2.0;
        }
        let v1 = 0.2;
        if mode == 1 {
            (((0.75 + klass as f32 * v1) * (self.c + 6.0)) / 7.0) * self.g
        } else {
            (((0.75 + klass as f32 * v1) * (2.0 * self.c + 5.0)) / 7.0) * self.g
        }
    }

    /// Pitch reference (`PlayerTune.k()`): the engine sound divides speed
    /// by this (`bt` calls `EngineSounds.a` with the ratio).
    pub fn pitch_ref(&self, klass: u8) -> f32 {
        (((0.75 + klass as f32 * 0.2) * (2.0 * self.c + 5.0)) / 7.0) * self.j
    }

    /// Steering lock at this speed (`cl.n(float)`): `min(o, n / speed)`.
    pub fn steer_lock(&self, speed: f32) -> f32 {
        self.o.min(self.n / speed.max(0.5))
    }

    /// Tire force clamp bounds (`CarPhysics.t()`/`u()`/`v()`, inherited
    /// unchanged by both tunes): the only handling the stat bytes buy.
    /// Outside mode 2 this is `(r * 2, s, t * (1 + ib / 10))`.
    pub fn tire_bounds(&self) -> (f32, f32, f32) {
        (self.r * 2.0, self.s, self.t * (1.0 + self.ib as f32 / 10.0))
    }
}

impl Default for Tuning {
    /// The first car in `ba.a`'s list (`rally.car`, stats 3/5/5/1).
    fn default() -> Tuning {
        Tuning::player([3, 5, 5, 1])
    }
}

/// What the engine mixer needs each frame (`EngineSounds` state).
#[derive(Clone, Copy, Default)]
pub struct EngineState {
    /// `speed / k`, the pitch input (`bt` calls `EngineSounds.a` with it).
    pub speed_frac: f32,
    /// Coasting (`EngineSounds.c`).
    pub coasting: bool,
    /// Steering/skid flag (`EngineSounds.b`).
    pub working: bool,
    /// Set on a shunt, decays in `step` (crash sound trigger).
    pub crashed: bool,
}

/// One car: the game state that matters (`RaceCar` + its `CameraState`).
///
/// The game keeps two parallel speeds: the scalar `j` state (pedals drive
/// it via `cl.f()`/`cl.g()` between the idle `m` and the cap `l()`, and it
/// feeds the engine pitch, the AI thresholds and the steering lock) and the
/// velocity vector `b_bz` (force-integrated in `cl.c(float)`, planar-capped
/// at the tune `k()`, and the thing position integrates). The port keeps
/// both, with the same field roles and the same clamps.
pub struct Car {
    pos: Vec3,
    yaw: f32,
    /// Scalar drive state, `di.j` (`cl.f()` adds `h * dt`, `cl.g()`
    /// subtracts it, floored at the tune `m`, capped at `l()`).
    drive: f32,
    /// Velocity vector, `b_bz` (integrated in `cl.c(float)`, planar cap
    /// the tune `k()`).
    vel: Vec3,
    /// Steering angle, `di.d` (`cl.n()`/`cl.o()` add `j * dt` slide-scaled
    /// and clamp to `min(o, n / speed)`; `cl.b()` centers it at `q`/s).
    steer: f32,
    reversing: bool,
    airborne: bool,
    fall: f32,
    tune: Tuning,
    ai: bool,
    spawn: Vec3,
    spawn_yaw: f32,
    state: EngineState,
    slide: f32,
    /// Whether this step scraped a barrier wall. The slideshow mode
    /// zeroes its drift score on a wall touch (`help.txt`: "If you bump
    /// into a wall, the drift counter is zeroed").
    wall_hit: bool,
}

pub struct World {
    pub cars: Vec<Car>,
    walls: Vec<(Vec3, Vec3)>,
}

impl World {
    pub fn new(walls: &[(Vec3, Vec3)]) -> World {
        World {
            cars: Vec::new(),
            walls: walls.to_vec(),
        }
    }

    pub fn add_car(&mut self, spawn: Vec3, yaw: f32, tune: Tuning, ai: bool) -> usize {
        self.cars.push(Car {
            pos: spawn,
            yaw,
            drive: 0.0,
            vel: Vec3::ZERO,
            steer: 0.0,
            reversing: false,
            airborne: false,
            fall: 0.0,
            tune,
            ai,
            spawn,
            spawn_yaw: yaw,
            state: EngineState::default(),
            slide: 0.0,
            wall_hit: false,
        });
        self.cars.len() - 1
    }

    /// Advance every car. `heights[i]` is the track surface under car `i`
    /// (`support_height`), or `None` past the edge - the game samples the
    /// collision mesh the same way (`cl.d(float)` grounds `a_bz`, and a big
    /// negative step means air).
    pub fn step(&mut self, dt: f32, controls: &[CarControl], heights: &[Option<f32>]) {
        let dt = dt.clamp(1.0 / 240.0, 1.0 / 30.0);
        for index in 0..self.cars.len() {
            let control = controls.get(index).copied().unwrap_or_default();
            let height = heights.get(index).copied().flatten();
            Self::step_car(&mut self.cars[index], &self.walls, dt, control, height);
        }
        Self::shunts(&mut self.cars);
        for car in self.cars.iter_mut() {
            car.state.crashed = false;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn step_car(
        car: &mut Car,
        walls: &[(Vec3, Vec3)],
        dt: f32,
        control: CarControl,
        height: Option<f32>,
    ) {
        let tune = car.tune;
        let top = tune.top_speed(1, false);
        let rate = tune.throttle_rate(1, 0, car.ai);
        let speed = car.vel.length();
        let forward = vec3(-yaw_sin(car.yaw), 0.0, -yaw_cos(car.yaw));
        let mut coasting = true;

        // Pedals drive the scalar state (`cl.f()` throttle adds `h * dt`
        // and clamps to `l()`; `cl.g()` brake subtracts and floors at the
        // idle creep `m`). No separate reverse key: holding brake past a
        // standstill flips the direction latch, after which the same
        // pedals back up.
        if control.throttle > 0.0 {
            coasting = false;
            car.drive += rate * control.throttle * dt;
            if car.drive > top {
                car.drive = top;
            }
            car.reversing = false;
        } else if coasting {
            // Lift-off slows the car instead of cruising forever: the
            // drive state eases back toward a standstill on its own.
            // (The game holds the last state and rolls on; the operator
            // prefers a car that stops.)
            // Exponential: roughly three seconds from full chat to a
            // standstill, smooth all the way down.
            car.drive *= (-1.5 * dt).exp();
            if car.drive.abs() < 0.05 {
                car.drive = 0.0;
            }
        } else if control.throttle < 0.0 || control.brake {
            coasting = false;
            let push = if control.brake { 1.0 } else { -control.throttle };
            if speed < 0.5 && car.drive < 0.5 {
                car.reversing = true;
            }
            if car.reversing {
                car.drive -= rate * push * dt;
                if car.drive < -top * 0.4 {
                    car.drive = -top * 0.4;
                }
            } else {
                car.drive -= rate * push * dt;
                if car.drive < tune.m {
                    car.drive = tune.m;
                }
            }
        }
        car.state.coasting = coasting;

        // The velocity vector chases the drive state. The game's exact
        // throttle-to-velocity coupling is the one open item in the port
        // (see module docs); this chase keeps the game's caps and creep
        // with an arcade response of roughly three seconds to cruise.
        // Braking bites harder, as it should. Neither is verbatim.
        // Chase the drive state, but no further than the planar cap:
        // chasing the raw 76-scale drive would pin the car at the cap
        // within a second whatever the blend. Reversing chases backwards.
        let cruise = tune.pitch_ref(1);
        let want = forward * car.drive.clamp(-cruise * 0.4, cruise);
        let rate = if control.brake { 4.0 } else { 1.1 };
        let blend = (dt * rate).min(1.0);
        car.vel += (want - car.vel) * blend;
        // Planar cap is the tune `k()` (`c(float)` rescales past it:
        // 19 units/s at class 1, the road speed everything else keys off).
        let planar = vec3(car.vel.x, 0.0, car.vel.z).length();
        let cap = tune.pitch_ref(1);
        if planar > cap && planar > 0.01 {
            let keep = cap / planar;
            car.vel.x *= keep;
            car.vel.z *= keep;
        }

        // Steering (`cl.n(float)`/`cl.o(float)` rate workers): the rate is
        // the tune `j`, scaled by `1.4 - 0.4 * |steer|` while gripping,
        // then clamped to the speed-sensitive lock.
        let want_steer = control.steer * if car.reversing { -1.0 } else { 1.0 };
        let lock = tune.steer_lock(speed);
        let target = (want_steer * lock).clamp(-lock, lock);
        let grip = car.ai || speed < 1.0;
        let mut step = tune.j * dt;
        if grip && car.slide.abs() < 1.0 {
            step *= 1.4 - 0.4 * car.steer.abs().min(1.0);
        }
        if (target - car.steer).abs() <= step {
            car.steer = target;
        } else {
            car.steer += step * (target - car.steer).signum();
        }
        // Self-centering (`cl.b(float)` steering return at the tune `q`).
        if want_steer.abs() < 0.05 {
            let back = tune.q * dt;
            if car.steer.abs() <= back {
                car.steer = 0.0;
            } else {
                car.steer -= back * car.steer.signum();
            }
        }
        car.state.working = want_steer.abs() > 0.05 && speed > 1.0;

        // Yaw authority (`cl.a(float)` clamps the yaw moment to +-1.5
        // rad/s, and the lock law `min(o, n / speed)` puts full lock at
        // n = 1.571 against that cap, so full lock always reaches about
        // maximum authority at any speed: rate = steer * speed capped.
        // APPROXIMATION (module docs): the game routes this through its
        // yaw-rate averager chain; the numbers at both ends are verbatim.
        let yaw_rate = if speed < tune.a * 0.2 {
            0.0
        } else {
            (car.steer * speed).clamp(-1.5, 1.5)
        };
        car.yaw += yaw_rate * dt;

        // Lateral grip: bleed the sideways velocity (`c(float)` tire forces
        // with the 0.2 low-speed cutoff; the tune `g` sets the rate).
        let fwd = vec3(-yaw_sin(car.yaw), 0.0, -yaw_cos(car.yaw));
        let fwd_speed = car.vel.dot(fwd);
        let mut side = car.vel - fwd * fwd_speed;
        let grip_rate = (tune.g / 150.0 * 8.0).min(12.0);
        let keep = (-grip_rate * dt).exp();
        side *= keep;
        if speed < 0.2 && side.length() > fwd_speed.abs() {
            side = Vec3::ZERO;
        }
        car.slide = side.length();
        car.vel = fwd * fwd_speed + side;

        // Quadratic drag (`c(float)` `k_bz` term in the tune `r`/`s`).
        let v = car.vel.length();
        if v > 0.01 {
            let drag = (tune.s * v + tune.r * v * v) / tune.f;
            car.vel *= (1.0 + drag * dt).max(0.0);
        }

        // Move and slide along barriers (kinematic: the game never beaches).
        let mut next = car.pos + car.vel * dt;
        for &(centre, half) in walls {
            next = slide_out(next, centre, half);
        }
        car.wall_hit = speed > 2.0 && (next - (car.pos + car.vel * dt)).length() > 0.001;
        car.pos = next;

        // Ground: snap to the sampled surface; past the edge, fall.
        // (`cl.d(float)`: a small step snaps, a big negative one means air.)
        match height {
            Some(h) => {
                let diff = h - car.pos.y;
                if diff > -2.5 {
                    car.pos.y = h;
                    car.airborne = false;
                    car.fall = 0.0;
                } else if !car.airborne {
                    car.airborne = true;
                    car.fall = 0.0;
                }
                if car.airborne {
                    car.fall += 9.8 * dt;
                    car.pos.y -= car.fall * dt;
                    if car.pos.y <= h {
                        car.pos.y = h;
                        car.airborne = false;
                        car.fall = 0.0;
                    }
                }
            }
            None => {
                car.airborne = true;
                car.fall += 9.8 * dt;
                car.pos.y -= car.fall * dt;
            }
        }

        // Engine pitch input is drive over the `k()` reference, not over
        // top speed (`bt`: `EngineSounds.a(speed / k * 100)`).
        car.state.speed_frac = car.drive.abs() / tune.pitch_ref(1).max(1.0);
    }
    /// Car-to-car shunts plus the draft (`cl.a(ObfCl, Z)`).
    ///
    /// A shunt fires below 0.9 units; slow pairs exchange the full push
    /// and fast ones a tenth. A car close behind a faster one picks up the
    /// 0.25 draft term.
    fn shunts(cars: &mut [Car]) {
        let n = cars.len();
        for a in 0..n {
            for b in (a + 1)..n {
                let delta = cars[b].pos - cars[a].pos;
                let flat = vec3(delta.x, 0.0, delta.z);
                let dist = flat.length();
                if dist >= 0.9 || dist < 1e-4 {
                    continue;
                }
                let push = flat / dist;
                let slow = cars[a].vel.length() < 0.1 && cars[b].vel.length() < 0.1;
                let factor = if slow { 1.0 } else { 0.1 };
                cars[a].vel -= push * factor;
                cars[b].vel += push * factor;
                // Draft: the slower car behind gains when the gap closes.
                let (ahead, behind) = if cars[a].vel.dot(push) >= 0.0 { (a, b) } else { (b, a) };
                let gap = (cars[ahead].pos - cars[behind].pos).length();
                if gap < 3.0 {
                    let dir = vec3(
                        -yaw_sin(cars[behind].yaw),
                        0.0,
                        -yaw_cos(cars[behind].yaw),
                    );
                    cars[behind].vel += dir * 0.25 * (1.0 - gap / 3.0);
                    cars[behind].state.working = true;
                }
                cars[a].state.crashed = true;
                cars[b].state.crashed = true;
            }
        }
    }

    /// Translation and rotation in macroquad space.
    pub fn pose(&self, car: usize) -> (Vec3, Quat) {
        let entry = &self.cars[car];
        (entry.pos, Quat::from_rotation_y(entry.yaw))
    }

    pub fn speed(&self, car: usize) -> f32 {
        self.cars[car].vel.length()
    }

    /// Sideways speed, the drift meter's input.
    pub fn slide(&self, car: usize) -> f32 {
        self.cars[car].slide
    }

    /// Whether the last step scraped a barrier wall.
    pub fn wall_hit(&self, car: usize) -> bool {
        self.cars[car].wall_hit
    }

    pub fn engine(&self, car: usize) -> EngineState {
        self.cars[car].state
    }

    pub fn reset(&mut self, car: usize) {
        let entry = &mut self.cars[car];
        entry.pos = entry.spawn;
        entry.yaw = entry.spawn_yaw;
        entry.vel = Vec3::ZERO;
        entry.steer = 0.0;
        entry.reversing = false;
        entry.airborne = false;
        entry.fall = 0.0;
        entry.state = EngineState::default();
    }

    /// Teleport a car onto the given point (used to rejoin after a fall).
    pub fn replace(&mut self, car: usize, position: Vec3, yaw: f32) {
        let entry = &mut self.cars[car];
        entry.pos = vec3(position.x, position.y + 1.0, position.z);
        entry.yaw = yaw;
        entry.vel = Vec3::ZERO;
        entry.steer = 0.0;
    }

    pub fn position(&self, car: usize) -> Vec3 {
        self.cars[car].pos
    }
}

/// Push a point out of a barrier box along the smallest penetration axis.
///
/// The game never beaches a car, so walls slide instead of stopping.
fn slide_out(point: Vec3, centre: Vec3, half: Vec3) -> Vec3 {
    let dx = point.x - centre.x;
    let dy = point.y - centre.y;
    let dz = point.z - centre.z;
    let px = half.x - dx.abs();
    if px <= 0.0 {
        return point;
    }
    let py = half.y - dy.abs();
    if py <= 0.0 {
        return point;
    }
    let pz = half.z - dz.abs();
    if pz <= 0.0 {
        return point;
    }
    if px <= py && px <= pz {
        vec3(centre.x + half.x * dx.signum(), point.y, point.z)
    } else if py <= pz {
        vec3(point.x, centre.y + half.y * dy.signum(), point.z)
    } else {
        vec3(point.x, point.y, centre.z + half.z * dz.signum())
    }
}

fn yaw_sin(yaw: f32) -> f32 {
    yaw.sin()
}

fn yaw_cos(yaw: f32) -> f32 {
    yaw.cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tune_tables_match_the_game() {
        // `PlayerTune`/`AiTune` constructor defaults.
        let player = Tuning::player([3, 5, 5, 1]);
        assert_eq!((player.u, player.v, player.ia, player.ib), (3.0, 5.0, 5, 1));
        assert!((player.f - 1500.0 * 9.8 * 0.5).abs() < 1.0);
        assert!((player.i - 0.628_318_55).abs() < 1e-6);
        let ai = Tuning::ai([3, 5, 5, 1]);
        assert!((ai.i - 1.047_197_6).abs() < 1e-6);
        assert_eq!(ai.k, player.k);
        // `PlayerTune.l()`: class 1 outside mode 2 is 0.95 * 80.
        assert!((player.top_speed(1, false) - 76.0).abs() < 1e-3);
        // `PlayerTune.h()`: class 1 outside mode 2 is 0.95 * 150.
        assert!((player.throttle_rate(1, 0, false) - 142.5).abs() < 1e-3);
        // `AiTune.h()`: setting-free, 0.95 * 7 / 7 * 150.
        assert!((ai.throttle_rate(1, 0, true) - 142.5).abs() < 1e-3);
    }

    #[test]
    fn full_throttle_does_not_shoot() {
        // One second of throttle must not put the car at cruise: the game
        // revs fast but the chassis answers over seconds, not frames.
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        for _ in 0..60 {
            world.step(1.0 / 60.0, &drive, &heights);
        }
        let early = world.speed(car);
        assert!(early < 14.0, "shoots: {early:.2} after one second");
        assert!(early > 4.0, "tractor: {early:.2} after one second");
    }

    #[test]
    fn lift_off_coasts_to_a_stop() {
        // No auto-cruise: release everything at speed and the car must
        // come back to a standstill on its own.
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        for _ in 0..300 {
            world.step(1.0 / 60.0, &drive, &heights);
        }
        assert!(world.speed(car) > 10.0);
        let coast = [CarControl::default()];
        for _ in 0..600 {
            world.step(1.0 / 60.0, &coast, &heights);
        }
        assert!(world.speed(car) < 1.0, "still rolling: {:.2}", world.speed(car));
    }

    #[test]
    fn throttle_climbs_to_top_speed_and_brake_floors_it() {
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        for _ in 0..600 {
            world.step(1.0 / 60.0, &drive, &heights);
        }
        // The drive state revs to the tune top while the velocity vector
        // settles where the chase meets the tune drag, below the planar
        // `k()` cap: two parallel speeds, as in game.
        let tune = Tuning::player([3, 5, 5, 1]);
        let top = tune.top_speed(1, false);
        assert!((world.cars[car].drive - top).abs() < top * 0.05);
        let cruise = world.speed(car);
        assert!(cruise > 12.0 && cruise < 17.0, "cruise {cruise:.2}");
        let brake = [CarControl { throttle: 0.0, steer: 0.0, brake: true }];
        for _ in 0..600 {
            world.step(1.0 / 60.0, &brake, &heights);
        }
        // Brake floors at the idle creep (`cl.g(float)`, tune `m`).
        assert!(world.speed(car) < 2.0, "speed {}", world.speed(car));
    }

    #[test]
    fn steering_lock_tightens_with_speed() {
        let tune = Tuning::player([3, 5, 5, 1]);
        // `min(o, n / speed)`: full lock crawling, ~9 degrees at speed 10.
        assert!(tune.steer_lock(0.0) > 3.0);
        assert!((tune.steer_lock(10.0) - 0.157_079_64).abs() < 1e-6);
    }

    #[test]
    fn cars_never_beach_or_turtle() {
        // No floor at all: the car falls but stays upright and drivable,
        // and `reset` reseats it. There is no `upright`/`conform` any more.
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 10.0, 0.0), 0.0, Tuning::default(), false);
        let drive = [CarControl { throttle: 1.0, steer: 1.0, brake: false }];
        for _ in 0..120 {
            world.step(1.0 / 60.0, &drive, &[None]);
        }
        let (_, quat) = world.pose(car);
        let up = quat * macroquad::prelude::vec3(0.0, 1.0, 0.0);
        assert!(up.y > 0.99);
        world.reset(car);
        assert_eq!(world.speed(car), 0.0);
    }
}
