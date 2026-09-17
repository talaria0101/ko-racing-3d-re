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
//! verbatim +-1.5 authority cap; the longitudinal coupling is the force
//! model (`100 * drive` engine, `(3v - 6v|v|)` drag, over mass 1500,
//! capped at `k()` = 19) with the drive at the full pedal rate - top
//! speed is cap-limited exactly like the game. Wheel spin, steered-tyre
//! and slope forces, the lateral tyre clamps and the suspension settle
//! (`cl.k(float)`) are not simulated; lateral velocity keeps a grip
//! chase toward the heading, tilt comes from the surface normal, and
//! lift-off bleeds velocity by operator order (the game rolls on). The
//! `.car` tail past the four stat bytes feeds the menus, not the car.

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
    /// How long the climb veto has held this car under a steady surface.
    /// A car buried past the step limit can never drive out: every exit
    /// step reverts, forward or reverse. Past two pinned seconds it pops
    /// up onto its own surface instead of freezing there forever. The
    /// How long the climb veto has held this car under one steady
    /// surface. A car buried past the step limit can never drive out:
    /// every exit step reverts, forward or reverse. Past four and a
    /// half pinned seconds - one full stuck-reverse cycle - it pops up
    /// onto its own surface instead of freezing there forever. The
    /// surface must match the window start, so rising grades (ramps,
    /// banks) never trigger it, and short rams (the cliff unit test)
    /// stay safely under it. Escapes reset the window the moment any
    /// normal step runs, so a reverse that works never pops.
    pin: f32,
    pin_h: f32,
    /// Metres climbed since flat ground, bled over about a second, while
    /// off the road. A bank is a sustained ascent (the accumulator grows
    /// toward a metre and the extra drag, fifteen per metre, stalls the
    /// car near the foot); a lip, kerb or shoulder is a brief one (it
    /// passes before the drag bites). This is what stops gradual banks,
    /// where no single step ever trips the climb veto: the veto sees
    /// steps, this sees hillsides.
    climb: f32,
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
            pin: 0.0,
            pin_h: 0.0,
            climb: 0.0,
        });
        self.cars.len() - 1
    }

    /// Advance every car. `heights[i]` is the track surface under car `i`
    /// (`support_height`), or `None` past the edge - the game samples the
    /// collision mesh the same way (`cl.d(float)` grounds `a_bz`, and a big
    /// negative step means air).
    pub fn step(
        &mut self,
        dt: f32,
        controls: &[CarControl],
        heights: &[Option<f32>],
        offroad: &[bool],
    ) {
        let dt = dt.clamp(1.0 / 240.0, 1.0 / 30.0);
        for index in 0..self.cars.len() {
            let control = controls.get(index).copied().unwrap_or_default();
            let height = heights.get(index).copied().flatten();
            let rough = offroad.get(index).copied().unwrap_or(false);
            Self::step_car(&mut self.cars[index], &self.walls, dt, control, height, rough);
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
        offroad: bool,
    ) {
        let tune = car.tune;
        let top = tune.top_speed(1, false);
        let rate = tune.throttle_rate(1, 0, car.ai);
        let speed = car.vel.length();
        let forward = vec3(-yaw_sin(car.yaw), 0.0, -yaw_cos(car.yaw));
        // Decided from the input up front: testing the initial `true`
        // down the branch chain sent every brake pedal down the coast
        // path instead, so brakes never bit and reverse never latched.
        let mut coasting = control.throttle == 0.0 && !control.brake;

        // Pedals drive the scalar state (`cl.f()` throttle adds `h * dt`
        // and clamps to `l()`; `cl.g()` brake subtracts and floors at the
        // idle creep `m`). No separate reverse key: holding brake past a
        // standstill flips the direction latch, after which the same
        // pedals back up. Both pedals integrate at the full tune rate:
        // the drive snaps 0-76 in half a second exactly like the game,
        // and the launch stays calm because force, not the drive value,
        // moves the chassis (see below).
        let tip_in = rate;
        if control.throttle > 0.0 {
            coasting = false;
            // Climb cap: off the road, sustained ascent closes the
            // throttle (see `climb`): the drive state cannot build past
            // what the slope allows, so banks stall near the foot instead
            // of grinding up. Reverse and brake stay free, which is the
            // way back down and out.
            let cap = if offroad {
                (1.0 - 3.0 * car.climb).max(0.0)
            } else {
                1.0
            };
            car.drive += tip_in * control.throttle * cap * dt;
            let allow = top * cap;
            if car.drive > allow {
                car.drive = allow;
            }
            car.reversing = false;
        } else if coasting {
            // Lift-off slows the player's car instead of cruising forever:
            // the drive state eases back toward a standstill on its own.
            // (The game holds the last state and rolls on; the operator
            // prefers a car that stops.) AI cars keep the game's
            // persistent drive: shared decay starves them to walking pace
            // wherever the laws coast a lot, while their pace comes from
            // the brake governor in `ai.rs` instead.
            // Exponential: roughly three seconds from full chat to a
            // standstill, smooth all the way down.
            if !car.ai {
                car.drive *= (-1.5 * dt).exp();
                if car.drive.abs() < 0.05 {
                    car.drive = 0.0;
                }
                // The force model coasts on weak drag alone and would roll
                // nearly forever (as the game does); the operator wants a
                // car that stops, so lift-off also bleeds velocity at the
                // same rate. Player only, AI keeps the game's roll.
                car.vel *= (-1.5 * dt).exp();
            }
        } else if control.throttle < 0.0 || control.brake {
            coasting = false;
            let push = if control.brake { 1.0 } else { -control.throttle };
            // The latch trips at the creep floor, not below it: brake
            // floors the drive at `m`, which sits above the old 0.5 test,
            // so holding brake past a standstill could never back up.
            if speed < 0.5 && car.drive <= tune.m + 0.05 {
                car.reversing = true;
            }
            if car.reversing {
                car.drive -= tip_in * push * dt;
                if car.drive < -top * 0.4 {
                    car.drive = -top * 0.4;
                }
            } else if control.brake {
                car.drive -= rate * push * dt;
                if car.drive < tune.m {
                    car.drive = tune.m;
                }
            } else {
                car.drive -= tip_in * push * dt;
                if car.drive < tune.m {
                    car.drive = tune.m;
                }
            }
        }
        car.state.coasting = coasting;

        // Longitudinal force, after `c(float)`: the engine `i_bz` pushes
        // 100 times the drive state along the heading, drag `k_bz` answers
        // `(3v - 6v|v|)` on the forward axis (a push below 0.5 units/s:
        // the idle creep), all over mass `e` (1500). Top speed is
        // cap-limited, not drag-limited: the engine still pulls at the
        // cap, so the `k()` rescale below is what tops the car out,
        // exactly like the game. Spin, steered-tyre and slope forces of
        // the original are not modelled (see module docs).
        // Lateral velocity keeps the old grip chase toward the heading
        // (rate as before); only the forward axis is force-driven.
        let fwd = car.vel.x * forward.x + car.vel.z * forward.z;
        if control.brake && !car.reversing {
            // Brake pads are not among the traced forces; the pedal
            // chases the chassis down hard, as before.
            let cruise = tune.pitch_ref(1);
            let want = forward * car.drive.clamp(-cruise * 0.4, cruise);
            let blend = (dt * 8.0).min(1.0);
            car.vel += (want - car.vel) * blend;
        } else {
            let engine = 100.0 * car.drive;
            let drag = 3.0 * fwd - 6.0 * fwd * fwd.abs();
            let push = (engine + drag) / 1500.0 * dt;
            let rate = if car.reversing { 2.0 } else { 0.9 };
            let blend = (dt * rate).min(1.0);
            let side_x = car.vel.x - forward.x * fwd;
            let side_z = car.vel.z - forward.z * fwd;
            let fwd_new = fwd + push;
            car.vel.x = forward.x * fwd_new + side_x * (1.0 - blend);
            car.vel.z = forward.z * fwd_new + side_z * (1.0 - blend);
        }
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

        // No multiplier drag here: drag already went in as a force above
        // (`k_bz`), and the MIDlet bogs nothing on grass (verified in
        // `c(float)` - no surface term anywhere). `offroad` gates only
        // the bank machinery below (climb cap, grade memory).

        // Move and slide along barriers (kinematic: the game never beaches).
        let mut next = car.pos + car.vel * dt;
        for &(centre, half) in walls {
            next = slide_out(next, centre, half);
        }
        car.wall_hit = speed > 2.0 && (next - (car.pos + car.vel * dt)).length() > 0.001;
        car.pos = next;

        // Ground: snap to the sampled surface; past the edge, fall.
        // (`cl.d(float)`: a small step snaps, a big negative one means air.)
        // A big step UP is a cliff, not a ramp: ramps and tile lips pass
        // under the limit, walls do not, so the car stops instead of
        // teleporting onto them. Off the road, sustained climbs are
        // rate-limited too: gentle grass passes slowly, but banks too
        // steep to drive stay unclimbable (the car stops at their foot
        // instead of pointing uphill and going). That is the roadside
        // collision the original has where there is no rail. On the road
        // the limit never applies - roads are drivable by definition.
        let y_before = car.pos.y;
        match height {
            Some(h) => {
                let diff = h - car.pos.y;
                // Uphill roads and bridge ramps stay well under 3 m/s of
                // rise even at cruise; banks do not.
                if diff > 1.2 {
                    car.pos.x -= car.vel.x * dt;
                    car.pos.z -= car.vel.z * dt;
                    car.vel *= 0.2;
                    // Buried past the step the car can never drive out, so
                    // a steady surface pops it up after two pinned seconds.
                    // A moving grade resets the timer, which is what keeps
                    // long climbs (and the bank unit test) from popping.
                    if car.pin == 0.0 {
                        car.pin_h = h;
                    }
                    car.pin += dt;
                    if car.pin > 4.5 && (h - car.pin_h).abs() < 0.1 {
                        car.pos.y = h;
                        car.airborne = false;
                        car.fall = 0.0;
                        car.pin = 0.0;
                    }
                } else if offroad && diff > 0.0 && diff / dt > 3.0 {
                    car.pos.x -= car.vel.x * dt;
                    car.pos.z -= car.vel.z * dt;
                    car.vel *= 0.2;
                } else if diff > -2.5 {
                    car.pos.y = h;
                    car.airborne = false;
                    car.fall = 0.0;
                    car.pin = 0.0;
                } else if !car.airborne {
                    car.airborne = true;
                    car.fall = 0.0;
                    car.pin = 0.0;
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
                car.pin = 0.0;
            }
        }
        // Ascent memory for the climb cap: gains with every metre risen
        // while off the road, bleeds only while moving. A stall holds
        // its value (nothing moves, nothing bleeds), so the cap stays
        // shut and cannot ratchet the car up in bleed-and-climb cycles;
        // anything with momentum washes the memory out, so lips, kerbs
        // and crests taken at speed are unaffected.
        // Gravity along the slope goes with it: ascent spends kinetic
        // energy (`v^2 = 2 g dh`), so momentum alone carries a car about
        // a metre up a bank and no further. It keys off actual ascent,
        // never the support gap, so a car held at a slope foot loses
        // nothing standing still - the cut vanishes with the motion,
        // which is what the gap-based attempt got wrong.
        if offroad {
            let dy = (car.pos.y - y_before).max(0.0);
            car.climb += dy;
            if dy > 0.0 {
                let v = car.vel.length();
                if v > 1e-3 {
                    car.vel *= ((v * v - 2.0 * 9.8 * dy).max(0.0)).sqrt() / v;
                }
            }
        }
        if car.vel.length() > 1.5 {
            car.climb *= (-1.5 * dt).exp();
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
        // 3.8 is the force model racing the stopwatch (100 * j over 1500
        // with the full pedal rate), not a tuned number.
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        for _ in 0..60 {
            world.step(1.0 / 60.0, &drive, &heights, &[false]);
        }
        let early = world.speed(car);
        assert!(early < 14.0, "shoots: {early:.2} after one second");
        assert!(early > 2.5, "tractor: {early:.2} after one second");
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
            world.step(1.0 / 60.0, &drive, &heights, &[false]);
        }
        assert!(world.speed(car) > 10.0);
        let coast = [CarControl::default()];
        for _ in 0..600 {
            world.step(1.0 / 60.0, &coast, &heights, &[false]);
        }
        assert!(world.speed(car) < 1.0, "still rolling: {:.2}", world.speed(car));
    }

    #[test]
    fn brake_at_standstill_reverses() {
        // Holding brake past a standstill must back up: the latch trips
        // at the creep floor (brake floors the drive at `m`, which used
        // to sit above the latch and block reverse entirely).
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let (_, rotation) = world.pose(car);
        let forward = rotation * vec3(0.0, 0.0, -1.0);
        let start = world.position(car);
        let brake = [CarControl { throttle: -0.6, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        for _ in 0..240 {
            world.step(1.0 / 60.0, &brake, &heights, &[false]);
        }
        assert!(world.cars[car].drive < 0.0, "drive never went negative");
        let travelled = world.position(car) - start;
        assert!(
            travelled.dot(forward) < -0.5,
            "did not back up: {travelled:?}"
        );
    }

    #[test]
    fn verges_cost_pace_nothing_but_cliffs_block() {
        // The MIDlet runs identical forces on grass and road, so the
        // verge cruise matches the road cruise; cutting is priced by
        // lost progress, not the tyres. Into a cliff face the car stops
        // instead of teleporting up.
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        let mut road = World::new(&[]);
        let car = road.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let mut rough = World::new(&[]);
        let car2 = rough.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        for _ in 0..600 {
            road.step(1.0 / 60.0, &drive, &heights, &[false]);
            rough.step(1.0 / 60.0, &drive, &heights, &[true]);
        }
        assert!(road.speed(car) > 12.0, "road cruise {}", road.speed(car));
        assert!(
            (rough.speed(car2) - road.speed(car)).abs() < 1.0,
            "verge cruise {} vs road {}",
            rough.speed(car2),
            road.speed(car)
        );
        // A five-metre step up blocks; the flat control case moves off.
        let mut cliff = World::new(&[]);
        let car3 = cliff.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let wall = [Some(5.0)];
        for _ in 0..60 {
            cliff.step(1.0 / 60.0, &drive, &wall, &[false]);
        }
        let moved = (cliff.position(car3) - vec3(0.0, 0.0, 0.0)).length();
        assert!(moved < 1.0, "climbed the cliff: {moved:.2}");
    }

    #[test]
    fn banks_too_steep_stay_unclimbable() {
        // Off the road a gentle rise passes slowly but a bank blocks;
        // on the road even the bank passes, since roads are drivable by
        // definition.
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let mut gentle = World::new(&[]);
        let g = gentle.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let mut bank = World::new(&[]);
        let b = bank.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let mut road = World::new(&[]);
        let r = road.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        for step in 0..120 {
            // 1.2 m/s of rise passes, 6 m/s does not.
            gentle.step(1.0 / 60.0, &drive, &[Some(step as f32 * 0.02)], &[true]);
            bank.step(1.0 / 60.0, &drive, &[Some(step as f32 * 0.10)], &[true]);
            road.step(1.0 / 60.0, &drive, &[Some(step as f32 * 0.10)], &[false]);
        }
        assert!(gentle.position(g).y > 1.0, "gentle rise blocked");
        let moved = (bank.position(b) - vec3(0.0, 0.0, 0.0)).length();
        assert!(moved < 2.0, "climbed the bank: {moved:.2}");
        assert!(road.position(r).y > 3.0, "road climb blocked");
    }

    #[test]
    fn buried_cars_pop_out_after_a_pinned_burial() {
        // A car held under a steady surface pops up after one full
        // stuck-reverse cycle (4.5 s) instead of freezing there forever
        // (1.map's south bank wedged AI cars exactly this way); shorter
        // pins pop nothing, which is what keeps escapes and the cliff
        // test's rammer down.
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let roof = [Some(2.0)];
        for _ in 0..180 {
            world.step(1.0 / 60.0, &drive, &roof, &[false]);
        }
        assert!(world.position(car).y < 0.5, "short pin must not pop");
        for _ in 0..180 {
            world.step(1.0 / 60.0, &drive, &roof, &[false]);
        }
        assert!(
            (world.position(car).y - 2.0).abs() < 1e-3,
            "burial never popped: {:.2}",
            world.position(car).y
        );
    }

    #[test]
    fn sustained_offroad_climbs_stall_but_lips_pass() {
        // A sustained ascent off the road stalls the car low (banks);
        // a brief lip mounts and stays mounted (kerbs, shoulders).
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let mut hill = World::new(&[]);
        let h = hill.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        for step in 0..600 {
            hill.step(1.0 / 60.0, &drive, &[Some(step as f32 * 0.05)], &[true]);
        }
        assert!(
            hill.position(h).y < 1.5,
            "climbed the hill: {:.2}",
            hill.position(h).y
        );
        let mut lip = World::new(&[]);
        let l = lip.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        for step in 0..300 {
            lip.step(
                1.0 / 60.0,
                &drive,
                &[Some((step as f32 * 0.04).min(0.5))],
                &[true],
            );
        }
        assert!(
            lip.position(l).y > 0.4,
            "lip never mounted: {:.2}",
            lip.position(l).y
        );
    }


    #[test]
    fn throttle_climbs_to_top_speed_and_brake_floors_it() {
        let mut world = World::new(&[]);
        let car = world.add_car(vec3(0.0, 0.0, 0.0), 0.0, Tuning::player([3, 5, 5, 1]), false);
        let drive = [CarControl { throttle: 1.0, steer: 0.0, brake: false }];
        let heights = [Some(0.0)];
        for _ in 0..600 {
            world.step(1.0 / 60.0, &drive, &heights, &[false]);
        }
        // The drive state revs to the tune top while the velocity pins
        // at the planar `k()` cap: the engine still pulls there, so the
        // cap - not drag equilibrium - tops the car out, exactly like
        // the game (drive 76 parallel with speed 19).
        let tune = Tuning::player([3, 5, 5, 1]);
        let top = tune.top_speed(1, false);
        assert!((world.cars[car].drive - top).abs() < top * 0.05);
        let cruise = world.speed(car);
        assert!((cruise - 19.0).abs() < 0.5, "cruise {cruise:.2}");
        let brake = [CarControl { throttle: 0.0, steer: 0.0, brake: true }];
        for _ in 0..600 {
            world.step(1.0 / 60.0, &brake, &heights, &[false]);
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
            world.step(1.0 / 60.0, &drive, &[None], &[false]);
        }
        let (_, quat) = world.pose(car);
        let up = quat * macroquad::prelude::vec3(0.0, 1.0, 0.0);
        assert!(up.y > 0.99);
        world.reset(car);
        assert_eq!(world.speed(car), 0.0);
    }
}

