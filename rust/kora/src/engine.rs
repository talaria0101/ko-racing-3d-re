//! Engine and effect sounds, the way `EngineSounds` runs them.
//!
//! The game mixes engine loops on its own 50 ms thread (`EngineSounds.run`):
//! a clip table (`acc`, `acc2`, `acc3`, `dec`, `fade`, `slide`, `l0`, `l1`),
//! a command/state input each frame (`a` = pitch percent from speed over the
//! tune `k()`, `b` = working/skid flag or crash code 50, `c` = coasting),
//! volume through the player (`50 * Settings.s()`), and one crash one-shot
//! (`a(50 * t)` plays clip 0). macroquad is the backend (`Sound`,
//! `play_sound` looped, `set_sound_volume`), which has volume and looping
//! but no pitch - the RPM bands the game already splits across clips do the
//! pitch's job here.
//!
//! Two honest limits, both stated where they bite. The clips this MIDlet
//! build references (`sounds/acc_8k.amr` and friends) are not in the
//! shipped JAR at all - `EngineSounds.h()` preloads them inside a
//! try/catch precisely so the game runs silent without them - so this
//! mixer loads `.wav` twins from `assets/sounds/` when present and stays
//! silent otherwise (transcode any AMR originals with e.g.
//! `ffmpeg -i acc_8k.amr -ar 22050 acc.wav`). And without a pitch knob the
//! bands below quantize what the game sweeps continuously.

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound};
use macroquad::audio::{PlaySoundParams, Sound};

use crate::physics::EngineState;

/// The game's clip table in load order (`EngineSounds.a_String`).
const CLIPS: [&str; 8] = ["acc", "acc2", "dec", "fade", "slide", "l0", "l1", "acc3"];

pub struct Engine {
    clips: Vec<Option<Sound>>,
    current: Option<usize>,
    logged: bool,
}

impl Engine {
    /// Read the `.wav` twins beside the pack; missing files stay `None`
    /// and the mixer stays silent, like the game without its clips.
    pub async fn load(dir: &std::path::Path) -> Engine {
        let mut clips = Vec::with_capacity(CLIPS.len());
        for name in CLIPS {
            let path = dir.join(format!("sounds/{name}.wav"));
            let sound = match std::fs::read(&path) {
                Ok(bytes) => load_sound_from_bytes(&bytes).await.ok(),
                Err(_) => None,
            };
            clips.push(sound);
        }
        Engine {
            clips,
            current: None,
            logged: false,
        }
    }

    fn play_loop(&mut self, index: usize, volume: f32) {
        if self.current == Some(index) {
            if let Some(sound) = &self.clips[index] {
                set_sound_volume(sound, volume);
            }
            return;
        }
        if let Some(old) = self.current.and_then(|i| self.clips[i].as_ref()) {
            stop_sound(old);
        }
        self.current = Some(index);
        if let Some(sound) = &self.clips[index] {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume,
                },
            );
        }
    }

    /// One frame of the mix for the player's car. `throttle` tells accel
    /// clips from cruise the way the game's state flags do.
    pub fn update(&mut self, state: EngineState, throttle: f32, volume: f32) {
        if !self.logged {
            self.logged = true;
            let have = self.clips.iter().filter(|c| c.is_some()).count();
            if have == 0 {
                eprintln!("  no sounds/*.wav engine clips beside the pack, engine silent");
            } else {
                println!("  engine clips: {have}/{} (.wav twins)", CLIPS.len());
            }
        }
        if state.crashed {
            // Crash one-shot (`EngineSounds.a(50 * t)` plays clip 0).
            if let Some(sound) = self.clips[0].as_ref() {
                use macroquad::audio::play_sound_once;
                play_sound_once(sound);
            }
        }
        let pitch = state.speed_frac.clamp(0.0, 2.0);
        let band = if pitch < 0.08 {
            5 // l0: idle.
        } else if state.coasting {
            3 // fade: rolling off.
        } else if state.working && pitch > 0.4 {
            4 // slide: skidding.
        } else if throttle > 0.0 && pitch > 0.55 {
            1 // acc2: pulling hard.
        } else if throttle > 0.0 {
            0 // acc: pulling.
        } else {
            6 // l1: cruise.
        };
        // Volume follows load the way `50 * s` scales the player.
        let load = if throttle > 0.0 {
            0.55 + 0.45 * pitch.min(1.0)
        } else if state.coasting {
            0.25
        } else {
            0.4
        };
        self.play_loop(band, volume * load);
    }

    pub fn stop(&mut self) {
        if let Some(old) = self.current.and_then(|i| self.clips[i].as_ref()) {
            stop_sound(old);
        }
        self.current = None;
    }
}
