//! Road graph derived from a `.map` plus its tiles.
//!
//! Adjacency comes from each tile's *open-side* flags (the `.tl` block read as
//! `open_sides`), rotated by the cell's 0-3 argument so that the flags line up
//! with world directions.  Two neighbouring cells are connected only when both
//! declare the shared side open, which is what makes the graph match the road
//! the MIDlet's `bs` flood walks: every one of the 40 shipped maps comes out
//! connected, with a mostly degree-2 ring topology.

use std::collections::HashMap;

use macroquad::prelude::*;

use crate::format::{Map, Tile};
use crate::scene::TILE;

/// Direction indices, matching the MIDlet's `b.a(i)`/`b.b(i)`.
/// 0 = +X, 1 = -Y, 2 = -X, 3 = +Y, in game space (X/Y ground, Z up).
pub const DIRS: [(i32, i32); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];

/// The same directions in macroquad space, where `(x, y, z) -> (x, z, -y)`.
pub fn dir_mq(dir: usize) -> Vec3 {
    match dir {
        0 => vec3(1.0, 0.0, 0.0),
        1 => vec3(0.0, 0.0, 1.0),
        2 => vec3(-1.0, 0.0, 0.0),
        _ => vec3(0.0, 0.0, -1.0),
    }
}

pub struct Grid {
    pub width: i32,
    pub height: i32,
    open: Vec<u8>,
    occupied: Vec<bool>,
    pub start: (i32, i32),
    pub finish: (i32, i32),
    pub checkpoints: Vec<(i32, i32)>,
    /// Visit order along the track-linking flood (`bs.a()V`): the start
    /// cell is 0 and indices grow in the raced direction, first visit
    /// wins. Cells the flood never reaches (void, verges) have no entry
    /// and read as zero progress, as in the game.
    prog: HashMap<(i32, i32), u32>,
    /// Cells from the start to the finish the first time the flood gets
    /// there; every numbered cell when start and finish coincide. The
    /// game counts the same length doubled (`bs.b_F`, two stamps per
    /// cell), which normalises away. First-visit numbering keeps every
    /// index below this length, so on-road progress never exceeds 1.0.
    lap_len: u32,
    /// First flood step out of the start cell: the direction the race is
    /// run. The flood scans sides in 0-3 order and takes the first
    /// connected one, so this is ground truth where the checkpoint aim
    /// used to guess.
    flood_dir: Option<usize>,
    /// Cells by flood index, for waypoint lookups (`ck` arrays its line
    /// the same way, by `bm.c()`/`bm.d()`).
    order: Vec<(i32, i32)>,
    /// Boundary midpoint between each flood cell and the next, in
    /// macroquad space, parallel to `order` (wrapping round the end).
    edges: Vec<Vec3>,
}

impl Grid {
    pub fn build(map: &Map, tiles: &HashMap<u8, Tile>) -> Grid {
        let width = map.width as i32;
        let height = map.height as i32;
        let mut open = vec![0u8; (width * height) as usize];
        let mut occupied = vec![false; (width * height) as usize];

        for y in 0..height {
            for x in 0..width {
                let Some((kind, arg)) = map.cells[y as usize][x as usize].tile else {
                    continue;
                };
                let Some(tile) = tiles.get(&kind) else {
                    continue;
                };
                let index = (y * width + x) as usize;
                occupied[index] = true;
                let mut mask = 0u8;
                for dir in 0..4 {
                    if tile.open_sides[((dir as u8 + arg) % 4) as usize] {
                        mask |= 1 << dir;
                    }
                }
                open[index] = mask;
            }
        }

        let mut grid = Grid {
            width,
            height,
            open,
            occupied,
            start: (map.start.0 as i32, map.start.1 as i32),
            finish: (map.finish.0 as i32, map.finish.1 as i32),
            checkpoints: map
                .checkpoints
                .iter()
                .map(|&(x, y)| (x as i32, y as i32))
                .collect(),
            prog: HashMap::new(),
            lap_len: 1,
            flood_dir: None,
            order: Vec::new(),
            edges: Vec::new(),
        };
        grid.link();
        grid
    }

    /// Walk the road from the start cell the way `bs.a()V` does: sides in
    /// 0-3 order, first connected side wins, cells numbered in visit
    /// order and restamped on revisit. The game walks open sides and ends
    /// at the first void step, which would strand every blind mouth; what
    /// survives observably is the numbered road loop in flood direction,
    /// and that is what this keeps: connected sides only, with a
    /// backtracking search so spurs get nearby indices instead of ending
    /// the walk.
    fn link(&mut self) {
        let mut order = 1u32;
        let mut consumed: std::collections::HashSet<((i32, i32), usize)> =
            std::collections::HashSet::new();
        self.prog.insert(self.start, 0);
        // Stack of (cell, next side to try): depth-first, sides in 0-3
        // order, backtracking out of dead ends.
        let mut stack = vec![(self.start, 0usize)];
        while !stack.is_empty() {
            let top = stack.len() - 1;
            let (cell, dir) = stack[top];
            if cell == self.finish && order > 1 && self.lap_len == 1 {
                self.lap_len = order;
            }
            let mut found = None;
            for d in dir..4 {
                if self.connected(cell.0, cell.1, d) && !consumed.contains(&(cell, d)) {
                    found = Some(d);
                    stack[top].1 = d + 1;
                    break;
                }
            }
            let Some(d) = found else {
                stack.pop();
                continue;
            };
            consumed.insert((cell, d));
            let next = (cell.0 + DIRS[d].0, cell.1 + DIRS[d].1);
            consumed.insert((next, (d + 2) % 4));
            if self.flood_dir.is_none() && cell == self.start {
                self.flood_dir = Some(d);
            }
            if self.prog.contains_key(&next) {
                if next == self.start {
                    break; // loop closed, as in the game
                }
                continue;
            }
            self.prog.insert(next, order);
            order += 1;
            stack.push((next, 0));
        }
        if self.lap_len == 1 {
            // Start and finish coincide (or the finish was never reached):
            // the lap is the whole numbered loop.
            self.lap_len = order.max(2);
        }
        let mut cells: Vec<((i32, i32), u32)> =
            self.prog.iter().map(|(&cell, &index)| (cell, index)).collect();
        cells.sort_by_key(|&(_, index)| index);
        self.order = cells.into_iter().map(|(cell, _)| cell).collect();
        let n = self.order.len();
        self.edges = (0..n)
            .map(|i| {
                let a = self.center(self.order[i].0, self.order[i].1);
                let b = self.center(self.order[(i + 1) % n].0, self.order[(i + 1) % n].1);
                vec3((a.x + b.x) * 0.5, 0.0, (a.z + b.z) * 0.5)
            })
            .collect();
    }

    /// Cells by flood index. Empty only when the flood never left the
    /// start cell.
    pub fn order(&self) -> &[(i32, i32)] {
        &self.order
    }

    /// Road-following point at `progress` (0..1 round the lap) plus
    /// `ahead` cells of lookahead, interpolated along boundary midpoints
    /// between consecutive flood cells: the port's `RaceLine`, which like
    /// `ck` arrays its line by flood index (`bs.a(x, y, vec)` writes one
    /// side midpoint per index) and reads it by progress. Midpoints hug
    /// the road round bends; interpolating cell centres instead would aim
    /// across corner mouths, straight into the rail pockets lining them.
    pub fn line_point(&self, progress: f32, ahead: f32) -> Option<Vec3> {
        if self.edges.is_empty() {
            return None;
        }
        let len = self.lap_len.max(1) as f32;
        let mut s = progress * len + ahead;
        s -= s.div_euclid(len) * len;
        let n = self.edges.len();
        // Edge midpoints sit half an index past their cell centres, so an
        // integer index lands on a centre and a half lands on a boundary.
        let e = (s - 0.5).rem_euclid(n as f32);
        let i0 = e.floor() as usize % n;
        let i1 = (i0 + 1) % n;
        let a = self.edges[i0];
        let b = self.edges[i1];
        let t = (e - e.floor()).clamp(0.0, 1.0);
        Some(vec3(
            a.x + (b.x - a.x) * t,
            0.0,
            a.z + (b.z - a.z) * t,
        ))
    }

    /// Flood index of a cell, if the walk numbered it. Exposed for tests
    /// and the placement log; the game keeps the same numbers in
    /// `bm.c()`/`bm.d()`.
    pub fn prog_index(&self, x: i32, y: i32) -> Option<u32> {
        self.prog.get(&(x, y)).copied()
    }

    /// How many cells the lap normalises over.
    pub fn lap_len(&self) -> u32 {
        self.lap_len
    }

    /// Track progress at a world position, `bs.a(Ldi;I)F` without the
    /// parts the port does not need. Returns `None` past the map edge
    /// (the game's 1000.0 sentinel, which only ever confuses the lap
    /// counters) and 0.0 on cells the flood never reached.
    pub fn progress_at(&self, position: Vec3) -> Option<f32> {
        // The game truncates `x / 14 + 0.5`, which matches round-half-up
        // for non-negative coordinates.
        let fx = position.x / TILE + 0.5;
        let fy = -position.z / TILE + 0.5;
        if fx < 0.0 || fy < 0.0 || fx >= self.width as f32 || fy >= self.height as f32 {
            return None;
        }
        let (cx, cy) = (fx.floor() as i32, fy.floor() as i32);
        let base = match self.prog.get(&(cx, cy)) {
            Some(&index) => index,
            None => return Some(0.0),
        };
        let frac = self.progress_frac(cx, cy, fx - cx as f32 - 0.5, fy - cy as f32 - 0.5);
        Some((base as f32 + frac) / self.lap_len as f32)
    }

    /// Sub-cell interpolation along the travel direction: the neighbour
    /// the flood reaches next decides which fractional axis counts, with
    /// the same per-direction formulas (`+x`, `-y`, `-x`, `+y`).
    fn progress_frac(&self, x: i32, y: i32, fx: f32, fy: f32) -> f32 {
        let here = self.prog.get(&(x, y)).copied().unwrap_or(0);
        let mut dir = None;
        for (d, (dx, dy)) in DIRS.iter().enumerate() {
            let next = (x + dx, y + dy);
            if self.connected(x, y, d)
                && self.prog.get(&next).is_some_and(|&index| index > here)
            {
                dir = Some(d);
                break;
            }
        }
        let dir = dir.unwrap_or_else(|| {
            self.neighbours(x, y).first().map(|&(d, _, _)| d).unwrap_or(0)
        });
        match dir {
            0 => fx + 0.5,
            1 => 0.5 - fy,
            2 => 0.5 - fx,
            _ => fy + 0.5,
        }
        .clamp(0.0, 1.0)
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    pub fn occupied(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some_and(|i| self.occupied[i])
    }

    pub fn open_sides(&self, x: i32, y: i32) -> u8 {
        self.index(x, y).map_or(0, |i| self.open[i])
    }

    /// Is the shared side between `(x, y)` and its neighbour in `dir` drivable?
    pub fn connected(&self, x: i32, y: i32, dir: usize) -> bool {
        if self.open_sides(x, y) & (1 << dir) == 0 {
            return false;
        }
        let (dx, dy) = DIRS[dir];
        let (nx, ny) = (x + dx, y + dy);
        self.open_sides(nx, ny) & (1 << ((dir + 2) % 4)) != 0
    }

    pub fn neighbours(&self, x: i32, y: i32) -> Vec<(usize, i32, i32)> {
        (0..4)
            .filter(|&dir| self.connected(x, y, dir))
            .map(|dir| {
                let (dx, dy) = DIRS[dir];
                (dir, x + dx, y + dy)
            })
            .collect()
    }

    /// Cell containing a macroquad-space point (cell centres are `TILE` apart).
    pub fn cell_of(&self, position: Vec3) -> (i32, i32) {
        (
            (position.x / TILE).round() as i32,
            (-position.z / TILE).round() as i32,
        )
    }

    pub fn center(&self, x: i32, y: i32) -> Vec3 {
        vec3(x as f32 * TILE, 0.0, -(y as f32) * TILE)
    }

    /// Base surface height of a cell (the tile models sit at `y = 0`).
    pub fn path(&self) -> Vec<(i32, i32)> {
        (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&(x, y)| self.occupied(x, y))
            .collect()
    }

    /// Direction out of `(x, y)` that best matches `heading` (a horizontal
    /// vector); falls back to the least-bad open side rather than reversing.
    pub fn pick_dir(&self, x: i32, y: i32, heading: Vec2) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (dir, _, _) in self.neighbours(x, y) {
            let d = dir_mq(dir);
            let dot = d.x * heading.x + d.z * heading.y;
            if best.is_none_or(|(best_dot, _)| dot > best_dot) {
                best = Some((dot, dir));
            }
        }
        best.map(|(_, dir)| dir)
    }

    /// A point on the road ahead of the car, used both by the AI and to sanity
    /// check the player's progress.
    pub fn target(&self, position: Vec3, heading: Vec3) -> Option<Vec3> {
        let (x, y) = self.cell_of(position);
        let heading2 = vec2(heading.x, heading.z).normalize_or_zero();
        let dir = self.pick_dir(x, y, heading2)?;
        let (dx, dy) = DIRS[dir];
        let (nx, ny) = (x + dx, y + dy);
        let near = self.center(nx, ny);
        // Half a cell further along, so the car aims through the corner.
        let ahead = match self.pick_dir(nx, ny, vec2(dir_mq(dir).x, dir_mq(dir).z)) {
            Some(next) => {
                let (ex, ey) = DIRS[next];
                self.center(nx + ex, ny + ey)
            }
            None => near,
        };
        Some(near.lerp(ahead, 0.5))
    }

    /// Walk backwards along the road from `from`, for laying out a starting
    /// grid.  Returns the cells behind the start, nearest first.
    pub fn cells_behind(&self, from: (i32, i32), heading: Vec3, count: usize) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        let mut cur = from;
        let mut back = -vec2(heading.x, heading.z).normalize_or_zero();
        for _ in 0..count {
            let mut best: Option<(f32, i32, i32)> = None;
            for (_, nx, ny) in self.neighbours(cur.0, cur.1) {
                let delta = self.center(nx, ny) - self.center(cur.0, cur.1);
                let dot = delta.x * back.x + delta.z * back.y;
                if best.is_none_or(|(best_dot, _, _)| dot > best_dot) {
                    best = Some((dot, nx, ny));
                }
            }
            let Some((_, nx, ny)) = best else { break };
            let delta = self.center(nx, ny) - self.center(cur.0, cur.1);
            back = vec2(delta.x, delta.z).normalize_or_zero();
            cur = (nx, ny);
            out.push(cur);
        }
        out
    }

    /// Which way round the circuit is raced.
    ///
    /// The first step of the track-linking flood: `bs.a()V` scans the
    /// start cell's sides in 0-3 order and walks the first connected one,
    /// and the lap counters only ever complete in that direction. Aiming
    /// at the first checkpoint instead heads the wrong way wherever both
    /// arms connect (Timberton races east, towards the long way round to
    /// the (2, 5) checkpoint, not west at it).
    pub fn race_dir(&self) -> Option<usize> {
        if let Some(dir) = self.flood_dir {
            return Some(dir);
        }
        self.neighbours(self.start.0, self.start.1)
            .first()
            .map(|&(dir, _, _)| dir)
    }

    /// Starting grid: the first slot is the start cell itself and the rest sit
    /// on the road behind it, each facing the direction of travel there.
    pub fn grid_slots(&self, count: usize) -> Vec<(Vec3, f32)> {
        let Some(dir) = self.race_dir() else {
            return Vec::new();
        };
        let mut cells = vec![self.start];
        let mut cur = self.start;
        let mut previous = None;
        // Heading we are walking in, i.e. backwards along the track.
        let mut backward = -dir_mq(dir);
        for _ in 1..count {
            let mut best: Option<(f32, i32, i32)> = None;
            for (_, nx, ny) in self.neighbours(cur.0, cur.1) {
                if previous == Some((nx, ny)) {
                    continue;
                }
                let delta = self.center(nx, ny) - self.center(cur.0, cur.1);
                let dot = delta.x * backward.x + delta.z * backward.z;
                if best.is_none_or(|(best_dot, _, _)| dot > best_dot) {
                    best = Some((dot, nx, ny));
                }
            }
            let Some((_, nx, ny)) = best else { break };
            let delta = self.center(nx, ny) - self.center(cur.0, cur.1);
            backward = vec3(delta.x, 0.0, delta.z).normalize_or_zero();
            previous = Some(cur);
            cur = (nx, ny);
            cells.push(cur);
        }

        // Short tracks may not have room for a full grid; stack the surplus
        // on the last cell rather than handing back fewer slots than asked.
        while cells.len() < count {
            let last = *cells.last().unwrap();
            cells.push(last);
        }

        let mut slots = Vec::with_capacity(count);
        let mut stack = 0.0f32;
        for (index, &cell) in cells.iter().enumerate() {
            if index > 0 && cell == cells[index - 1] {
                stack += 1.0;
            } else {
                stack = 0.0;
            }
            let here = self.center(cell.0, cell.1);
            let ahead = if index == 0 {
                here + dir_mq(dir) * TILE
            } else {
                let previous = cells[index - 1];
                self.center(previous.0, previous.1)
            };
            let facing = (ahead - here).normalize_or_zero();
            let yaw = (-facing.x).atan2(-facing.z);
            let side = if index % 2 == 0 { -1.0 } else { 1.0 };
            let lateral = if index == 0 { 0.0 } else { side * 2.5 };
            let offset = vec3(-facing.z, 0.0, facing.x) * lateral - facing * (stack * 3.5);
            slots.push((here + offset + vec3(0.0, 1.2 + stack * 0.3, 0.0), yaw));
        }
        slots
    }

    /// Ordered gate cells for a lap: checkpoint list, if the map has one.
    pub fn gates(&self) -> Vec<(i32, i32)> {
        let mut gates = vec![self.finish];
        gates.extend(self.checkpoints.iter().copied());
        if gates.len() > 1 && gates.last() == gates.first() {
            gates.pop();
        }
        gates
    }
}
