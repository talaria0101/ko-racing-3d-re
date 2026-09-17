//! Career tables: which campaign a track belongs to and how its race is set up.
//!
//! `<campaign>.000` lists the levels and the races, and each race record names
//! a game mode plus the byte offset of its setup inside `<campaign>.001`.
//! Level coordinates are read as 16-bit values even though they look like
//! bytes - `u.c()` calls an overloaded `be.a()` that returns a `short`, which
//! is invisible in the decompiled source because the overloads differ only by
//! return type.

use crate::format::{Campaign, RaceConfig};
use crate::pack::Resources;

/// The two career tables shipped in the pack.
const TABLES: [&str; 2] = ["campaign/campaign", "campaign/deluxe"];

pub struct RaceSetup {
    /// Resource base name, e.g. `campaign/campaign`.
    pub table: String,
    pub level_index: usize,
    pub level_name: String,
    pub config: RaceConfig,
}

impl RaceSetup {
    /// `(laps, opponents)` as the campaign intends them.  A solo time trial
    /// reports no opponents, which the caller may choose to top up.
    pub fn laps_and_opponents(&self) -> (u32, u32) {
        (self.config.laps, self.config.opponents)
    }
}

/// Parse every career table in the pack.
pub fn load(resources: &Resources) -> Vec<(String, Campaign)> {
    let mut tables = Vec::new();
    for base in TABLES {
        let index = format!("{base}.000");
        if let Some(bytes) = resources.get(&index) {
            if let Some(campaign) = Campaign::parse(bytes) {
                tables.push((base.to_string(), campaign));
            }
        }
    }
    tables
}

/// Find the race that drives `map_name`, preferring a mode that races
/// opponents over a solo time trial.
pub fn race_for(resources: &Resources, map_name: &str) -> Option<RaceSetup> {
    for (table, campaign) in load(resources) {
        let blob = resources.get(&format!("{table}.001"))?;
        let Some(level_index) = campaign
            .levels
            .iter()
            .position(|level| level.map == map_name)
        else {
            continue;
        };

        let mut records: Vec<_> = campaign
            .records
            .iter()
            .filter(|record| record.level as usize == level_index)
            .collect();
        // Race modes first, then the lowest mode number, so the choice is
        // stable rather than dependent on the table's row order.
        records.sort_by_key(|record| {
            let is_race = RaceConfig::RACE_MODES.contains(&record.mode);
            (!is_race, record.mode)
        });

        for record in records {
            let offset = record.values[2].max(0) as usize;
            if let Some(config) = RaceConfig::parse(blob, offset, record.mode) {
                return Some(RaceSetup {
                    table: table.clone(),
                    level_index,
                    level_name: campaign.levels[level_index].name.clone(),
                    config,
                });
            }
        }
    }
    None
}

/// One selectable race: a campaign record plus its decoded setup.
#[derive(Clone)]
pub struct RaceEvent {
    pub table: String,
    pub level_index: usize,
    pub name: String,
    pub map: String,
    pub mode: u8,
    pub laps: u32,
    pub opponents: u32,
    pub theme: u8,
    pub threshold: i32,
    /// Points the race is worth.  The MIDlet adds `record[3]` to the player's
    /// total on a win, but a *negative* value is not an award at all: `u.n()`
    /// negates it into a group index and sets that group's unlocked flag
    /// instead, which is how the bonus races hand over tracks and cars.  So
    /// this is the value floored at zero, and [`Self::unlocks`] carries the
    /// group when there is one.
    pub award: i32,
    pub unlocks: Option<u8>,
    /// Stable key for saving a best time against this race.
    pub key: String,
    /// Seconds on the clock for the time-boxed solo modes (time chase,
    /// slideshow, special), from the setup's `i32 time_limit`, which the
    /// game counts down in milliseconds (`Countdown.c`/`d`). `None` for
    /// the wheel-to-wheel modes.
    pub time_limit: Option<f32>,
    /// Player car pool index (`r.h`, the setup `car` byte): which of the
    /// eight garage cars the event fields the player in.
    pub car: u8,
    /// Opponent pool class (`r.m`, the setup `param` byte of race modes):
    /// which slice of the garage the rivals are drawn from.
    pub class: u8,
}

impl RaceEvent {
    fn build(table: &str, level_index: usize, name: &str, map: &str, record: &crate::format::RaceRecord, config: RaceConfig) -> RaceEvent {
        RaceEvent {
            table: table.to_string(),
            level_index,
            name: name.to_string(),
            map: map.to_string(),
            mode: record.mode,
            laps: config.laps,
            // Solo modes carry no opponents byte in their setup, so they
            // run one car against the clock or the judges.
            opponents: if config.is_race() { config.opponents } else { 0 },
            theme: config.theme,
            threshold: record.values[0],
            award: record.values[1].max(0),
            unlocks: (record.values[1] < 0).then(|| (-record.values[1]) as u8),
            key: format!("{table}:{level_index}:{}", record.mode),
            time_limit: config.time_limit.map(|ms| (ms as f32 / 1000.0).clamp(5.0, 1800.0)),
            car: config.car,
            class: config.param.unwrap_or(0),
        }
    }
}

/// The eight garage cars in `CarSpec.a` order: the pool the roster draws
/// from (rally, fashion/BIRDIE, vintage, sport, bonus, suv, cx, cool).
pub const CAR_POOL: [&str; 8] = [
    "rally", "fashion", "vintage", "sport", "bonus", "suv", "cx", "cool",
];

/// Tiny seeded RNG for the roster (the game rolls `KORa.rand`, unseeded;
/// the port seeds per race so results reproduce). Not the `rand` crate:
/// one `next_int` is all the roster needs.
pub struct RosterRng {
    state: u64,
}

impl RosterRng {
    pub fn new(seed: u64) -> RosterRng {
        RosterRng { state: seed } 
    }

    pub fn next_int(&mut self, bound: usize) -> usize {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 33) as usize) % bound.max(1)
    }
}

/// Opponent car pool indices for a race (`r.b()V` second loop): unique
/// random cars from the class range, never the player's, with the
/// verbatim remap chain. A `-1` roll (class 0) is an empty slot: the
/// field shrinks rather than fielding a duplicate. Verified for mode 1;
/// assumed across race modes pending the `cu`/`cd` reader check.
pub fn roster(
    player: usize,
    class: u8,
    opponents: usize,
    rng: &mut RosterRng,
) -> Vec<Option<usize>> {
    let mut out = Vec::with_capacity(opponents as usize);
    for _ in 0..opponents as usize {
        // Bounded retries: the game trusts its data to leave room, but
        // a forced override could ask for more unique cars than the pool
        // holds; an empty slot beats a hang.
        let mut tries = 0;
        loop {
            tries += 1;
            if tries > 100 {
                out.push(None);
                break;
            }
            let mut v = rng.next_int(4) as i32 + if class == 3 { 4 } else { class as i32 } - 1;
            if v == 3 {
                v = 2;
            } else if v == 4 {
                v = 3;
            } else if v == 5 {
                v = 4;
            } else if v == 2 {
                v = 5;
            }
            let cand = if v < 0 { None } else { Some(v as usize) };
            if cand == Some(player) {
                continue;
            }
            if out.contains(&cand) {
                continue;
            }
            out.push(cand);
            break;
        }
    }
    out
}

/// Every race in both career tables, in table order, races before time trials.
pub fn events(resources: &Resources) -> Vec<RaceEvent> {
    let mut events = Vec::new();
    for (table, campaign) in load(resources) {
        let Some(blob) = resources.get(&format!("{table}.001")) else {
            continue;
        };
        for (level_index, level) in campaign.levels.iter().enumerate() {
            let mut records: Vec<_> = campaign
                .records
                .iter()
                .filter(|record| record.level as usize == level_index)
                .collect();
            records.sort_by_key(|record| {
                let is_race = RaceConfig::RACE_MODES.contains(&record.mode);
                (!is_race, record.mode)
            });
            for record in records {
                let offset = record.values[2].max(0) as usize;
                let Some(config) = RaceConfig::parse(blob, offset, record.mode) else {
                    continue;
                };
                events.push(RaceEvent::build(
                    &table,
                    level_index,
                    &level.name,
                    &level.map,
                    record,
                    config,
                ));
            }
        }
    }
    events
}

/// Every track in the pack, for a quick race: the campaign setup where the
/// track has one, otherwise three laps against three opponents.
pub fn quick_events(resources: &Resources) -> Vec<RaceEvent> {
    let mut maps: Vec<String> = resources
        .keys()
        .filter(|name| name.starts_with("levels/") && name.ends_with(".map"))
        .map(|name| name.trim_start_matches("levels/").to_string())
        .collect();
    maps.sort();

    maps.into_iter()
        .map(|map| {
            let setup = race_for(resources, &map);
            let (laps, opponents, theme, mode) = match &setup {
                Some(setup) => (
                    setup.config.laps,
                    if setup.config.is_race() {
                        setup.config.opponents
                    } else {
                        0
                    },
                    setup.config.theme,
                    setup.config.mode,
                ),
                None => (3, 3, 0, 0),
            };
            RaceEvent {
                table: setup
                    .as_ref()
                    .map(|setup| setup.table.clone())
                    .unwrap_or_default(),
                level_index: setup.as_ref().map(|setup| setup.level_index).unwrap_or(0),
                name: map.trim_end_matches(".map").to_uppercase(),
                key: format!("quick:{map}:{mode}"),
                map,
                mode,
                laps,
                opponents,
                theme,
                threshold: 0,
                award: 1,
                unlocks: None,
                car: setup.as_ref().map(|setup| setup.config.car).unwrap_or(0),
                class: setup
                    .as_ref()
                    .and_then(|setup| setup.config.param)
                    .unwrap_or(0),
                time_limit: setup.and_then(|setup| {
                    setup
                        .config
                        .time_limit
                        .map(|ms| (ms as f32 / 1000.0).clamp(5.0, 1800.0))
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roster_is_unique_and_never_fields_the_player_car() {
        // Hundred seeded draws: no duplicates (one empty at most, from
        // the class-0 `-1` roll), never the player's own index, always in
        // pool range.
        for seed in 0..100 {
            let mut rng = RosterRng::new(seed * 7919 + 13);
            let field = roster(2, 1, 4, &mut rng);
            assert_eq!(field.len(), 4);
            let mut seen = std::collections::HashSet::new();
            for slot in field {
                if let Some(car) = slot {
                    assert!(car < 8, "out of pool: {car}");
                    assert_ne!(car, 2, "player's car fielded");
                    assert!(seen.insert(car), "duplicate car {car}");
                }
            }
        }
    }
}
