//! Opponent driving, after `PlayerCar.e(float)` (the race AI; `OpponentCar`
//! drives the demo car the same way with a harder jamb and no line jitter).
//!
//! The game steers by absolute line waypoints, not by the nose: each car
//! owns a `RaceLine` over the flood order (its cells' midpoints, jittered
//! per car), aims ~half a cell ahead of its progress, slams the wheels by
//! the jamb (`1.5 * err * |err|`, clamped) and works the pedals by corner
//! severity (`7 * err / curve` against speed: brake over it, full throttle
//! when aligned, launch throttle when slow). There is no target speed and
//! no reverse gear: a pinned car just grinds its jamb until it turns out,
//! with knockdown stops for car-car shunts (unported - the port keeps its
//! kinder stuck-reverse so a beached car can still finish a race).
//!
//! Divergences, all marked below: the steering jamb is applied positionally
//! (`control.steer` is -1..1, the game integrates wheel angle), the curve
//! factor is continuous (`cos(turn / 2)`, the game's snaps only its
//! extremes), the per-car line jitter uses a stable per-slot seed (the game
//! rolls `KORa.rand` every run), and the stuck-reverse has no original.

use macroquad::prelude::*;

use crate::grid::Grid;
use crate::physics::CarControl;

/// Line jitter per car, `±0.07 * 14` world units (`ck.a(boolean)` only
/// jitters when the flag is false, which is how every race AI builds its
/// line; the demo car passes true and drives the perfect line).
const JITTER: f32 = 1.0;

/// Lookahead in flood cells: half a cell (`ck.a(float, int)` at tier 0).
/// One AI car's state: its line jitter plus the stuck detector so a car
/// pinned against a barrier reverses out instead of grinding there for the
/// rest of the race.
pub struct AiDriver {
    jitter: (f32, f32),
    /// Lap progress when stillness started counting, and how long the car
    /// has gone nowhere. Speed alone cannot strand-detect: a car pushing
    /// into a rail pocket keeps its velocity (like the game's, which the
    /// positional slide never bleeds), so the trigger is static progress.
    last_prog: f32,
    still: f32,
    reversing: f32,
    /// Reverse lock side, alternating every trigger so retries veer out
    /// both ways instead of re-snagging the same rail end straight on.
    veer: f32,
    steer: f32,
    /// Brake hold after a hard wall hit: the game's knockdown recovery
    /// (`cx.m(float)`) stops the car and holds it briefly before driving
    /// on. The port holds the brake without the full stop.
    hold: f32,
}

impl AiDriver {
    /// `slot` seeds the line jitter, so each grid slot drives a slightly
    /// different line every race.
    pub fn new(slot: usize) -> AiDriver {
        let mut rng = slot as u64 * 0x9E3779B97F4A7C15 + 0x243F6A8885A308D3;
        let mut unit = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32 * JITTER
        };
        AiDriver {
            jitter: (unit(), unit()),
            last_prog: 0.0,
            still: 0.0,
            reversing: 0.0,
            veer: 1.0,
            steer: 0.0,
            hold: 0.0,
        }
    }

    /// A hard wall hit: hold the brake briefly, then drive on.
    pub fn knock(&mut self) {
        self.hold = 1.0;
    }

    pub fn control(
        &mut self,
        grid: &Grid,
        position: Vec3,
        heading: Vec3,
        speed: f32,
        dt: f32,
    ) -> CarControl {
        if self.reversing > 0.0 {
            self.reversing -= dt;
            return CarControl {
                throttle: -0.7,
                steer: self.veer,
                brake: false,
            };
        }
        let mut control = drive(grid, position, heading, speed, self.jitter);
        // No donuts: at walking pace full lock spins the car in place
        // (yaw follows steer times speed, so the jamb's full lock whirls
        // the nose faster than the car travels and the error never
        // settles - a stable attractor the original sits in forever).
        // Reversing keeps full lock to manoeuvre out of pockets.
        if self.reversing <= 0.0 {
            let cap = 0.25f32.max((speed / 4.0).min(1.0));
            control.steer = control.steer.clamp(-cap, cap);
        }
        if self.hold > 0.0 {
            self.hold -= dt;
            control.throttle = 0.0;
            control.brake = true;
            return control;
        }
        self.steer = control.steer;
        let progress = grid.progress_at(position).unwrap_or(0.0);
        if (progress - self.last_prog).abs() < 0.002 {
            self.still += dt;
        } else {
            self.still = 0.0;
            self.last_prog = progress;
        }
        if self.still > 2.0 {
            self.still = 0.0;
            self.last_prog = progress;
            // Long enough to back properly out of a rail pocket at the
            // calmed reverse rate, not just rock in it.
            self.reversing = 2.5;
            self.veer = -self.veer;
        }
        control
    }
}

/// Waypoint `ahead` cells past `progress`, jittered, read off the
/// flood-ordered line like `ck.a(float, int)`.
fn waypoint(grid: &Grid, progress: f32, ahead: f32, jitter: (f32, f32)) -> Option<Vec3> {
    grid
        .line_point(progress, ahead)
        .map(|point| vec3(point.x + jitter.0, 0.0, point.z + jitter.1))
}

pub fn drive(
    grid: &Grid,
    position: Vec3,
    heading: Vec3,
    speed: f32,
    jitter: (f32, f32),
) -> CarControl {
    let mut control = CarControl::default();
    let progress = grid.progress_at(position).unwrap_or(0.0);
    const LOOKAHEAD: f32 = 0.5;
    let Some(target) = waypoint(grid, progress, LOOKAHEAD, jitter) else {
        // Off the road entirely: creep forward so it can recover.
        control.throttle = 0.35;
        return control;
    };

    let to = vec3(target.x - position.x, 0.0, target.z - position.z);
    let distance = to.length();
    if distance < 0.05 {
        control.throttle = 0.6;
        return control;
    }
    let direction = to / distance;
    let forward = heading.normalize_or_zero();
    // Signed alignment error, the jamb input: positive turns the wheels
    // left (about +Y), exactly what the sign bucket feeds `h()`/`i()`.
    let cross = forward.cross(direction).y;
    let dot = forward.x * direction.x + forward.z * direction.z;
    let err = cross.atan2(dot);

    // The jamb, positionalized: the game integrates the wheels at
    // `1.5 * err * |err|` clamped to ±1.5, which over the same clamp is
    // `err * |err|` clamped to ±1.
    let jamb = (err * err.abs()).clamp(-1.0, 1.0);
    control.steer = jamb;
    // Back to the game's scale for the pedal law below.
    let v4 = jamb * 1.5;

    // Corner severity: the alignment error over the curve factor. The curve
    // reads the turn two cells out (`cos(turn / 2)`: 1 on a straight,
    // towards 0 in a hairpin), floored so it never divides by zero, so the
    // brake comes in before the corner rather than inside it.
    let second = grid
        .line_point(progress, LOOKAHEAD + 2.0)
        .map(|point| vec3(point.x + jitter.0, 0.0, point.z + jitter.1))
        .unwrap_or(target);
    let after = vec3(second.x - target.x, 0.0, second.z - target.z);
    let curve = if after.length_squared() > 1e-6 {
        let turn = (direction.x * after.x + direction.z * after.z)
            / (direction.length() * after.length());
        (turn * 0.5 + 0.5).sqrt().max(0.05)
    } else {
        1.0
    };
    let severity = v4.abs() / curve;

    // Corner approach: the sharpest turn within the next few cells sets a
    // target speed, and anything over it brakes now rather than inside
    // the corner. The game's law is reactive only (it brakes on current
    // misalignment), which arrives too hot wherever rails pocket a bend:
    // the car overshoots into the pocket and funnels there, where the
    // original strands with it. Proactive braking threads such corners
    // instead of visiting them.
    let mut approach = 1.0f32;
    let mut prev_p = target;
    let mut prev_d = direction;
    for step in 1..=4 {
        let p = grid
            .line_point(progress, LOOKAHEAD + step as f32)
            .map(|point| vec3(point.x + jitter.0, 0.0, point.z + jitter.1))
            .unwrap_or(target);
        let seg = vec3(p.x - prev_p.x, 0.0, p.z - prev_p.z);
        if seg.length_squared() > 1e-6 && prev_d.length_squared() > 1e-6 {
            let cos = (prev_d.x * seg.x + prev_d.z * seg.z)
                / (prev_d.length() * seg.length());
            approach = approach.min((cos * 0.5 + 0.5).sqrt().max(0.05));
            prev_d = seg;
        }
        prev_p = p;
    }
    let corner_speed = 4.0 + 8.0 * approach;

    // The pedals: brake over corner speed, full throttle when aligned,
    // launch throttle when slow (`f()`/`g()` integrate the same drive the
    // player pedals do).
    if (7.0 * severity > speed && speed > 1.2) || speed > corner_speed {
        control.brake = true;
        control.throttle = 0.0;
    } else if (severity < 0.1 && v4.abs() < 1.0) || speed < 0.4 {
        control.throttle = 1.0;
    }
    // Pace governor, the one unverbatim law: the throttle law pins the
    // drive wherever the line is straight enough, which cruises near the
    // cap - too fast to hold the folds where the road passes near itself
    // (the car cuts across to an earlier section and laps half the track
    // forever) or to turn into railed corners (it overshoots into the
    // pocket and funnels there). The game fields roughly player pace, so
    // the AI grounds the drive back down past it with the verbatim brake
    // pedal and lets the laws above handle the corners.
    if speed > 9.0 {
        control.throttle = 0.0;
        control.brake = true;
    }
    control
}
