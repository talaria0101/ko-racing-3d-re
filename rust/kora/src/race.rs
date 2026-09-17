//! Lap, progress and timing state.
//!
//! Laps come off the game's progress float (`bs.a(Ldi;I)F`, ticked per car
//! per frame by `cl.a(int, ObfR)`): each road cell carries its index along
//! the track-linking flood, and progress is that index plus a sub-cell
//! fraction over the lap length, running 0..1 round the lap. A lap
//! completes when progress wraps from ~1 back to ~0; a forward jump of the
//! same size undoes one (driving the line backwards, or a reset flinging
//! the car ahead). The wrap thresholds (0.8 down, 0.9 up) are verbatim, so
//! cutting the course can never score: progress is continuous along the
//! road, and skipping tarmac only ever reads less of it.
//!
//! Per car the engine keeps what the original keeps: total milliseconds
//! (`c`), current-lap milliseconds (`j_I`), best lap (`i_I`), laps counted
//! down (`g_I`) and up (`h_I`), the last progress read (`a_F`) and the
//! position score (`j_F`, laps plus progress). The frame time reaches the
//! tick as milliseconds, as `(int)(dt * 1000)` does in `bt.b`.

use macroquad::prelude::*;

use crate::grid::Grid;

/// The seven event modes (`ui.txt` 202-208: circuit, race, time chase,
/// survival, head to head, slideshow, special). Modes 0, 1, 3 and 4 race
/// opponents (`RaceConfig::RACE_MODES`); 2, 5 and 6 are solo, with 2 and 6
/// carrying a time limit and 5 paying drift points instead of places.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Circuit,
    Race,
    TimeChase,
    Survival,
    HeadToHead,
    Slideshow,
    Special,
}

impl Mode {
    pub fn from_u8(mode: u8) -> Mode {
        match mode {
            1 => Mode::Race,
            2 => Mode::TimeChase,
            3 => Mode::Survival,
            4 => Mode::HeadToHead,
            5 => Mode::Slideshow,
            6 => Mode::Special,
            _ => Mode::Circuit,
        }
    }

    /// Whether cars race each other rather than the clock or the judges.
    pub fn is_race(self) -> bool {
        matches!(
            self,
            Mode::Circuit | Mode::Race | Mode::Survival | Mode::HeadToHead
        )
    }
}

/// Drift points for one step. The longer the slide, the more points
/// (`help.txt`: "The longer your drift is, the more points are added").
/// Slides only count at speed, so parking sideways earns nothing.
pub fn drift_points(slide: f32, speed: f32, dt: f32) -> f32 {
    if speed > 3.0 {
        slide * speed * dt
    } else {
        0.0
    }
}

pub struct Race {
    pub laps: u32,
    /// Laps completed (`h_I`).
    pub lap: u32,
    pub lap_times: Vec<f32>,
    pub best: Option<f32>,
    pub finished: bool,
    pub finish_time: Option<f32>,
    /// Survival knock-outs: after each lap the last placed car is
    /// eliminated (`help.txt`), locks its controls and leaves the
    /// standings. An eliminated car can no longer finish.
    pub eliminated: bool,
    /// Total race milliseconds (`c`).
    total_ms: f64,
    /// Current-lap milliseconds (`j_I`).
    lap_ms: f64,
    /// Laps counted down (`g_I`): wraps add one, forward jumps take one.
    downs: i32,
    /// Last progress read (`a_F`).
    base: f32,
    /// Position score (`j_F`): laps plus progress.
    score: f32,
    started: f64,
    last_now: f64,
}

impl Race {
    pub fn new(_grid: &Grid, laps: u32, _position: Vec3, now: f64) -> Race {
        Race {
            laps: laps.max(1),
            lap: 0,
            lap_times: Vec::new(),
            best: None,
            finished: false,
            finish_time: None,
            eliminated: false,
            total_ms: 0.0,
            lap_ms: 0.0,
            downs: 0,
            base: 0.0,
            score: 0.0,
            started: now,
            last_now: now,
        }
    }

    pub fn update(&mut self, now: f64, grid: &Grid, position: Vec3) {
        if self.finished || self.eliminated {
            return;
        }
        let dt_ms = ((now - self.last_now) * 1000.0).max(0.0);
        self.last_now = now;
        // Time runs even where no progress scores (locked cars stop the
        // clock by never reaching the tick: finished and eliminated return
        // above, as `l_Z`/`n_Z` skip the accumulators).
        self.total_ms += dt_ms;
        self.lap_ms += dt_ms;
        // Past the map edge the game reads its 1000.0 sentinel, which only
        // ever confuses the counters (minus one, then plus one a frame);
        // the port scores nothing there instead.
        let Some(f) = grid.progress_at(position) else {
            return;
        };
        if f < 0.0 {
            return;
        }
        let delta = f - self.base;
        if delta > 0.8 {
            self.downs -= 1;
        } else if delta < -0.9 || f > 1.0 {
            if self.downs == self.lap as i32 {
                // A full lap. Past-the-finish cells also run this gate
                // every frame, as in the game; those milliseconds are not
                // laps, so only plausible lap times get recorded.
                if self.lap_ms > 1000.0 {
                    let time = (self.lap_ms / 1000.0) as f32;
                    self.lap_times.push(time);
                    if self.best.is_none_or(|best| time < best) {
                        self.best = Some(time);
                    }
                }
                self.lap_ms = 0.0;
                self.lap += 1;
            }
            self.downs += 1;
        }
        self.base = f;
        self.score = self.downs as f32 + f;
        if self.lap >= self.laps {
            self.finished = true;
            self.finish_time = Some((self.total_ms / 1000.0) as f32);
        }
    }

    pub fn lap_time(&self, now: f64) -> f32 {
        if self.finished {
            self.lap_times.last().copied().unwrap_or(0.0)
        } else {
            (now - (self.started + (self.total_ms - self.lap_ms) / 1000.0)) as f32
        }
    }

    pub fn total_time(&self, now: f64) -> f32 {
        if self.finished {
            self.finish_time.unwrap_or(0.0)
        } else {
            (now - self.started) as f32
        }
    }

    /// Race order: the position score, laps plus live progress. Higher is
    /// further ahead, as with `j_F`.
    pub fn progress(&self, grid: &Grid, position: Vec3) -> f32 {
        if self.finished {
            return 1_000_000.0 - self.finish_time.unwrap_or(0.0);
        }
        self.lap as f32 + grid.progress_at(position).unwrap_or(0.0)
    }
}
