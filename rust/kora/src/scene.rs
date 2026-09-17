//! Builds the static track from `.map` + `.tl` + `.md` + `.hd` + `.ob`.
//!
//! All geometry is baked into a handful of macroquad meshes (one per texture)
//! with the tile models transformed into world space on the CPU.  The same
//! tile triangles are also emitted as a collision trimesh so headless
//! tests can check the road surface the cars drive on.
//!
//! Coordinate convention: the game's models put **Z downwards** - `a`, the
//! collision mesh reader, negates the third component of every triangle it
//! builds, a tile raises its kerbs to *negative* z, and all twenty car models
//! stand their wheels on `z = 0` with the body above it.  macroquad is
//! Y-up, so the model axes are mapped as
//!
//! ```text
//! (x, y, z) -> (x, -z, -y)
//! ```
//!
//! That mapping reverses orientation, which is why the port's triangles come out
//! wound the opposite way to the MIDlet's.  The game draws its world with
//! `PolygonMode.setCulling(160)` (CULL_BACK) and macroquad culls nothing, so the
//! winding is cosmetic here and the orientation is not: copying z instead of
//! negating it lays the whole world out mirrored, hanging under the road, with
//! the cars driving along the underside of the track.
//!
//! Geometry building never touches the GPU, so it can be exercised headlessly;
//! [`Track::attach_textures`] uploads the atlases later, at runtime.

use std::collections::HashMap;
use std::path::Path;

use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;
use macroquad::texture::{Image, Texture2D};

use crate::format;
use crate::grid::Grid;
use crate::pack::{self, Resources};

/// The atlas a tile texture really means, which depends on the weather.
///
/// `ar.a(cf)` reads the tile's texture name and then, when it is the shared
/// atlas `texpack.png`, swaps in a recoloured one for the race's weather:
/// `ts.png` for snow, `td.png` for desert, `tf.png` for autumn.  The four share
/// a layout and differ in colour, which is why using the wrong one does not
/// look broken so much as *wrong* - green hills where the game shows autumn
/// ones.  `KORA_ATLAS` overrides it, which is how to try the others.
pub fn tile_atlas(theme: u8, texture: &str) -> String {
    if texture != "texpack.png" {
        return format!("tex/{texture}");
    }
    if let Ok(forced) = std::env::var("KORA_ATLAS") {
        return format!("tex/{forced}");
    }
    let swapped = match theme {
        2 => "ts.png",
        3 => "td.png",
        4 => "tf.png",
        _ => "texpack.png",
    };
    format!("tex/{swapped}")
}

/// The texture a mid-detail or object model really means, which also depends
/// on the weather.
///
/// `bc.a(cf)` (mid detail) swaps a texture containing `texpack` for the
/// seasonal atlas - `ts.png`, `td.png`, `tf.png` - and one starting with
/// `t.png` for the seasonal second sheet - `ts2.png`, `td2.png`, `tf2.png`.
/// `ai.a(cf)` (objects) only does the latter swap, so objects keep the plain
/// atlas whatever the season.  Anything else, and themes 0-1, keep the stored
/// name.  Without this, autumn races dress their trees in the summer sheet:
/// green walls where the game shows orange ones.
pub fn detail_texture(theme: u8, texture: &str) -> String {
    let swapped = match theme {
        2 => ("ts.png", "ts2.png"),
        3 => ("td.png", "td2.png"),
        4 => ("tf.png", "tf2.png"),
        _ => return format!("tex/{texture}"),
    };
    if texture.contains("texpack") {
        format!("tex/{}", swapped.0)
    } else if texture.starts_with("t.png") {
        format!("tex/{}", swapped.1)
    } else {
        format!("tex/{texture}")
    }
}

/// Which way a high-detail object faces: `ai.a(Lbq;Lj;FFFFFF)V` has two
/// render paths selected by the `.ob` flag (`ai.b`, set on trees, bushes
/// and cacti). The normal path posts the caller yaw (`bp` passes
/// 0/90/180/270) about `(0,0,1)`; the flagged path sets translation only
/// and renders with identity rotation, so the yaw is dropped for trees.
/// The port used to yaw every object, which turned each flat tree plane
/// (`t1` is all `y = 0.15`) edge-on or piled face-on into its neighbours:
/// the giant foliage wall on Timberton.
pub fn high_detail_yaw(flagged: bool, arg: u8) -> f32 {
    if flagged {
        0.0
    } else {
        arg as f32 * std::f32::consts::FRAC_PI_2
    }
}

/// What [`detail_texture`] does for object models: `ai.a(cf)` swaps only the
/// `t.png` sheet and leaves the shared atlas alone.
pub fn object_texture(theme: u8, texture: &str) -> String {
    let swapped = match theme {
        2 => "ts2.png",
        3 => "td2.png",
        4 => "tf2.png",
        _ => return format!("tex/{texture}"),
    };
    if texture.starts_with("t.png") {
        format!("tex/{swapped}")
    } else {
        format!("tex/{texture}")
    }
}

/// A deliberate handle on the tile atlas's texture coordinates.
///
/// The coordinates this port computes are the game's own, byte for byte, and
/// the whole-track renders say they land where they should.  Against a
/// rendering of the original they still look a little off, and there is no way
/// left to argue about that from inside the data, so this makes it testable by
/// eye instead: `KORA_UV_SCALE` multiplies the atlas's coordinates about the
/// centre and `KORA_UV_OFFSET="du,dv"` shifts them, both in texture units.
/// Neither touches the geometry, so a tile's shape cannot change - only which
/// part of the atlas paints it.  Unset, everything is untouched.
pub fn atlas_uv(uv: Vec2) -> Vec2 {
    let mut out = uv;
    if let Ok(scale) = std::env::var("KORA_UV_SCALE") {
        if let Ok(scale) = scale.parse::<f32>() {
            out = (out - vec2(0.5, 0.5)) * scale + vec2(0.5, 0.5);
        }
    }
    if let Ok(offset) = std::env::var("KORA_UV_OFFSET") {
        let parts: Vec<f32> = offset
            .split(',')
            .filter_map(|part| part.trim().parse::<f32>().ok())
            .collect();
        if parts.len() == 2 {
            out += vec2(parts[0], parts[1]);
        }
    }
    out
}

/// One of the game's vertices, in macroquad's Y-up world.
///
/// The game's models have **Z pointing down** - see the module comment - so the
/// third axis is negated here rather than copied.  `place` rotates each instance
/// in the game's own ground plane first and hands the result to this.
pub fn game_to_world(x: f32, y: f32, z: f32) -> Vec3 {
    vec3(x, -z, -y)
}

/// How much of a track to build, which is what the MIDlet's graphics detail
/// setting (`al.d()`) controls.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Detail {
    /// Base tiles only.
    Base,
    /// Tiles and the mid-detail layer (`md_list`).
    Mid,
    /// Everything, including the high-detail layer (`hd_list` + `ol`).
    Full,
}

/// World size of one tile (`ar.c` in the MIDlet).
pub const TILE: f32 = 14.0;
/// Static node scale the game applies to tile/detail/object models (`ar.a`).
pub const WORLD_SCALE: f32 = 7.01;

/// Per-cell collision meshes, so the runtime can ask how high the road is at
/// any point and lift a car back onto it.
/// One tile's visual triangles in cell-fraction plan space with macroquad
/// heights, shared by every cell of that kind. Only used where the tile
/// ships no collision mesh: the collider falls back to what is drawn so
/// cars stand on hillsides instead of driving straight through them.
pub type VisualTris = Vec<[[f32; 3]; 3]>;

pub struct SurfaceGrid {
    width: i32,
    height: i32,
    cells: Vec<Option<(u8, Option<format::Collision>, Option<std::rc::Rc<VisualTris>>)>>,
}

impl SurfaceGrid {
    pub fn height_at(&self, position: Vec3) -> Option<f32> {
        let x = (position.x / TILE).round() as i32;
        let y = (-position.z / TILE).round() as i32;
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let (arg, collision, visual) = self.cells[(y * self.width + x) as usize].as_ref()?;
        let centre = vec3(x as f32 * TILE, 0.0, -(y as f32) * TILE);
        let local = vec2(
            (position.x - centre.x) / TILE + 0.5,
            (centre.z - position.z) / TILE + 0.5,
        );
        Some(surface_height(collision.as_ref(), visual.as_deref(), *arg, local))
    }

    /// The height the runtime holds a car to: the surface under its centre,
    /// or under its nose when that is higher and the car is up for climbing.
    ///
    /// The MIDlet sets its car's height from the collision mesh every frame,
    /// so a step between two tiles is something it climbs rather than a wall
    /// - the foot of the `h1` ramp is a 4.2-unit step.  A physics chassis
    /// meets that step nose-first while its centre is still on the lower
    /// tile, where `height_at` reports the low side and no lift happens, so
    /// it would grind there forever.  Sampling the nose too reproduces the
    /// original behaviour: steps are climbed, while a car in the air (surface
    /// below on both samples) is left alone, so jumps still work.
    ///
    /// A car buried *under* the centre surface - spawned into a slope, or
    /// fallen into a step - comes straight up level via the centre sample.
    /// Teleporting that case to the higher nose sample instead would stand
    /// the body on its nose, past what the suspension can take, and wedge it.
    ///
    /// The nose lead is capped at what the suspension can follow in one
    /// frame.  An uncapped nose suspends a climbing car above its own wheels:
    /// every pop puts the body past the (high) nose sample while the wheels
    /// dangle short of the surface, so it never gets traction and stalls
    /// mid-ramp.  Capped, the car walks up steps progressively and stays in
    /// contact; a 4.2-unit ramp foot still climbs in a third of a second.
    pub fn support_height(
        &self,
        position: Vec3,
        heading: Vec3,
        reach: f32,
        body_y: f32,
        ride: f32,
    ) -> Option<f32> {
        let centre = self.height_at(position)?;
        if body_y < centre + ride {
            return Some(centre);
        }
        let nose = position + vec3(heading.x, 0.0, heading.z).normalize_or_zero() * reach;
        const LEAD: f32 = 0.2;
        let nose = self.height_at(nose).unwrap_or(f32::MIN).min(centre + LEAD);
        Some(centre.max(nose))
    }
}

/// Turns a lookup that should wrap into one that can be clamped, since a
/// clamped lookup is the only kind macroquad can do.
///
/// The game leaves every texture on the default `GL_REPEAT` and a few scenery
/// models rely on it - `zdzn` spans u 0..8 - but cars and track tiles do not:
/// their coordinates land inside 0..1 once the mesh bytes are read as signed,
/// which is what the file stores.  miniquad's texture
/// parameters default to `Clamp` and macroquad exposes no way to change them,
/// so the repeat is baked into the data instead: the image is tiled over the
/// integer window the coordinates occupy and every coordinate is rescaled
/// into that window.  A clamped lookup in the tiled copy is then exactly a
/// repeating lookup in the original, and it costs a couple of megabytes
/// rather than a shader.
///
/// `map` sends coordinates into 0..1 and `size` gives the copy's pixels.
#[derive(Clone, Copy, Debug)]
pub struct Tiling {
    origin: Vec2,
    tiles: Vec2,
}

impl Tiling {
    /// The tiling that covers `bounds`, which is `[u0, u1, v0, v1]`.
    pub fn for_bounds(bounds: [f32; 4]) -> Tiling {
        let origin = vec2(bounds[0].floor(), bounds[2].floor());
        Tiling {
            origin,
            tiles: vec2(
                (bounds[1].ceil() - origin.x).max(1.0),
                (bounds[3].ceil() - origin.y).max(1.0),
            ),
        }
    }

    /// Rescale a model texture coordinate into the tiled copy.
    pub fn map(self, uv: Vec2) -> Vec2 {
        (uv - self.origin) / self.tiles
    }

    /// Pixel size of the tiled copy of a `width` x `height` image.
    pub fn size(self, width: u16, height: u16) -> (usize, usize) {
        (
            width as usize * self.tiles.x as usize,
            height as usize * self.tiles.y as usize,
        )
    }

    /// True when the coordinates already fit and nothing needs copying.
    pub fn is_identity(self) -> bool {
        self.origin == Vec2::ZERO && self.tiles == Vec2::ONE
    }
}

/// A texture tiled by [`Tiling`], ready to draw.
pub struct Repeat {
    texture: Texture2D,
    tiling: Tiling,
}

/// Texture filtering override for looks over authenticity.
///
/// The game asks for nearest (`cf.a` passes `al.d` = 210) and the port
/// honours that by default, but at grazing angles one screen pixel covers
/// many texels and nearest turns the road into large flat blocks while
/// KEmulator's desktop GL blends them.  `KORA_SMOOTH=1` selects linear
/// filtering on the world and car textures instead - closer to the
/// emulator, at the price the game was avoiding (a little paint bleed
/// along atlas seams).
pub fn smooth() -> bool {
    std::env::var("KORA_SMOOTH").is_ok_and(|value| value != "0")
}

fn texture_filter() -> FilterMode {
    if smooth() {
        FilterMode::Linear
    } else {
        FilterMode::Nearest
    }
}

/// The rectangle a texture is used over when it needs no tiling.
const UNIT_UV: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

impl Repeat {
    /// Tile `image` so that a clamped lookup covers `bounds`.  `None` if that
    /// would make an unreasonable texture.
    pub fn build(image: &Image, bounds: [f32; 4]) -> Option<Repeat> {
        let tiling = Tiling::for_bounds(bounds);
        let (w, h) = (image.width as usize, image.height as usize);
        let (tw, th) = tiling.size(image.width, image.height);
        if w == 0 || h == 0 || tw > 4096 || th > 4096 {
            return None;
        }
        let mut bytes = vec![0u8; tw * th * 4];
        for y in 0..th {
            let source = (y % h) * w * 4;
            let row = y * tw * 4;
            for x in 0..tw {
                let from = source + (x % w) * 4;
                bytes[row + x * 4..row + x * 4 + 4]
                    .copy_from_slice(&image.bytes[from..from + 4]);
            }
        }
        let texture = Texture2D::from_rgba8(tw as u16, th as u16, &bytes);
        // `FILTER_NEAREST`, which is what the game asks for: `cf.a(string,
        // al.d, ...)` passes `al.d`, and `al.d` is 210.  This matters far more
        // than it looks - the atlas is a patchwork of unrelated artwork, so
        // *linear* filtering bleeds the neighbouring tile's art into every
        // texel at the edges, which reads as grass growing over the road and as
        // the road's own paint landing beside it.  Nearest keeps every texel
        // its own; `KORA_SMOOTH=1` takes linear anyway (see [`smooth`]).
        texture.set_filter(texture_filter());
        Some(Repeat { texture, tiling })
    }

    pub fn tiling(&self) -> Tiling {
        self.tiling
    }
}

pub struct Track {
    /// Road graph: which cells exist and which sides are drivable.
    pub grid: Grid,
    /// The height function the collider was built from.
    pub surface: SurfaceGrid,
    pub meshes: Vec<Mesh>,
    /// Texture resource for each mesh, parallel to `meshes`.
    pub texture_paths: Vec<String>,
    /// Texture coordinate rectangle covered by each mesh's models, keyed by
    /// texture resource.
    pub uv_bounds: HashMap<String, [f32; 4]>,
    pub collision_vertices: Vec<[f32; 3]>,
    pub collision_indices: Vec<[u32; 3]>,
    /// Edge barriers as `(centre, half extents)` in macroquad space.
    pub walls: Vec<(Vec3, Vec3)>,
    pub spawn: Vec3,
    pub spawn_yaw: f32,
    /// Every placed instance, in build order (see [`Placement`]).
    pub placements: Vec<Placement>,
}

impl Track {
    /// Upload every batch's texture, tiled over the coordinates its meshes
    /// actually use, and pull those coordinates back into the tiled range.
    /// Needs a live macroquad graphics context.
    pub fn attach_textures(&mut self, res: &Resources) {
        let mut cache: HashMap<String, Repeat> = HashMap::new();
        for (mesh, path) in self.meshes.iter_mut().zip(self.texture_paths.iter()) {
            if !cache.contains_key(path) {
                let bounds = self.uv_bounds.get(path).copied().unwrap_or(UNIT_UV);
                let built = res.get(path.trim_start_matches('/')).and_then(|bytes| {
                    Image::from_file_with_format(bytes, Some(ImageFormat::Png)).ok()
                });
                let Some(repeat) = built.and_then(|image| Repeat::build(&image, bounds)) else {
                    continue;
                };
                cache.insert(path.clone(), repeat);
            }
            if let Some(repeat) = cache.get(path) {
                for vertex in mesh.vertices.iter_mut() {
                    vertex.uv = repeat.tiling().map(vertex.uv);
                }
                mesh.texture = Some(repeat.texture.clone());
            }
        }
    }
}

/// Samples per cell edge when tessellating a cell whose surface is not flat.
/// The collision mesh is a height function over the cell's local `[0,1]^2`
/// square and most tiles only cover part of it, so the collider samples the
/// function instead of emitting the raw triangles: a kerb strip, a sunken
/// floor and a raised platform then all come out as one surface.
const SURFACE_STEPS: usize = 8;

/// `bm.a(float, float)`: the sample point is rotated into the mesh's frame.
fn rotate_sample(point: Vec2, arg: u8) -> Vec2 {
    match arg % 4 {
        1 => vec2(point.y, 1.0 - point.x),
        2 => vec2(1.0 - point.x, 1.0 - point.y),
        3 => vec2(1.0 - point.y, point.x),
        _ => point,
    }
}

/// The game's height function: the interpolated collision mesh height in world
/// units, or zero when the tile has no mesh or the point falls outside it.
///
/// The MIDlet negates the third component when it builds each triangle (`a` -
/// three `fneg`s in its constructor), so the height in the game's own Z-down
/// world is `-z * 14`.  This port maps that world to Y-up in `game_to_world`
/// by negating the axis once more, so the height here is `+z * TILE`.  Using
/// the game's formula raw mirrors every nonzero height about the road plane:
/// ramps become pits and kerbs become slots, which is how cars ended up driving
/// underneath elevated track (the `h1` ramp's visual top sits at +4.2 while the
/// unflipped sampler reports -4.2; `b1`'s kerb visuals run 0..+1.4; the `vl`
/// dip's visuals run -0.69..0.03).  The barriers this port builds place
/// themselves with `height_at` too, so they stand on the true surface with it.
/// The game's height function: the interpolated collision mesh height in world
/// units, falling back to the drawn tile triangles where the tile ships no
/// mesh (or the point misses it), or zero under truly flat art.
///
/// The MIDlet negates the third component when it builds each triangle (`a` -
/// three `fneg`s in its constructor), so the height in the game's own Z-down
/// world is `-z * 14`.  This port maps that world to Y-up in `game_to_world`
/// by negating the axis once more, so the height here is `+z * TILE`.
/// Visual plan maps model `(x, y)` to cell fractions `((x + 1) / 2)` on the
/// assumption the model frame matches the collision frame (verified for
/// scale and height sign on the `vl` dip; if hills ever mirror, flip an
/// axis here).
pub fn surface_height(
    collision: Option<&format::Collision>,
    visual: Option<&VisualTris>,
    arg: u8,
    local: Vec2,
) -> f32 {
    let point = rotate_sample(local, arg);
    if let Some(collision) = collision {
        if !collision.vertices.is_empty() {
            for triangle in &collision.triangles {
                let [a, b, c] = [
                    collision.vertices[triangle[0]],
                    collision.vertices[triangle[1]],
                    collision.vertices[triangle[2]],
                ];
                if let Some(h) = barycentric(&point, a, b, c) {
                    // The mesh shares the tile's 14-unit scale, and the sign is the
                    // port's Y-up one: see the note on this function.
                    return h * TILE;
                }
            }
        }
    }
    if let Some(visual) = visual {
        // Plan is already cell fractions and heights are macroquad units,
        // so the interpolant is the answer unwound.
        for triangle in visual {
            let [a, b, c] = *triangle;
            if let Some(h) = barycentric(&point, a, b, c) {
                return h;
            }
        }
    }
    0.0
}

fn barycentric(point: &Vec2, a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<f32> {
    let determinant = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if determinant.abs() < 1e-9 {
        return None;
    }
    let u = ((b[1] - c[1]) * (point.x - c[0]) + (c[0] - b[0]) * (point.y - c[1])) / determinant;
    let v = ((c[1] - a[1]) * (point.x - c[0]) + (a[0] - c[0]) * (point.y - c[1])) / determinant;
    if u >= -1e-6 && v >= -1e-6 && u + v <= 1.0 + 1e-6 {
        Some(u * a[2] + v * b[2] + (1.0 - u - v) * c[2])
    } else {
        None
    }
}

fn tile_has_mesh(tile: Option<&format::Tile>) -> bool {
    tile.and_then(|tile| tile.collision.as_ref())
        .is_some_and(|collision| !collision.vertices.is_empty())
}

fn world_of(centre: Vec3, local: Vec2, height: f32) -> Vec3 {
    vec3(
        centre.x + (local.x - 0.5) * TILE,
        height,
        centre.z - (local.y - 0.5) * TILE,
    )
}

fn push_triangle(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<[u32; 3]>,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) {
    let base = vertices.len() as u32;
    for point in [a, b, c] {
        vertices.push([point.x, point.y, point.z]);
    }
    indices.push([base, base + 1, base + 2]);
}

struct Part {
    verts: Vec<Vertex>,
    idx: Vec<u32>,
}

struct Batch {
    parts: Vec<Part>,
}

impl Batch {
    fn new() -> Batch {
        Batch {
            parts: vec![Part {
                verts: Vec::new(),
                idx: Vec::new(),
            }],
        }
    }

    /// macroquad meshes index with `u16`, so start a new part before overflow.
    fn triangle(&mut self, a: (Vec3, Vec2), b: (Vec3, Vec2), c: (Vec3, Vec2)) {
        if self.parts.last().unwrap().verts.len() + 3 > 60_000 {
            self.parts.push(Part {
                verts: Vec::new(),
                idx: Vec::new(),
            });
        }
        let part = self.parts.last_mut().unwrap();
        let base = part.verts.len() as u32;
        part.verts.push(Vertex::new2(a.0, a.1, WHITE));
        part.verts.push(Vertex::new2(b.0, b.1, WHITE));
        part.verts.push(Vertex::new2(c.0, c.1, WHITE));
        part.idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
}

/// One placed instance, recorded for the placement log: which `.map` cell
/// and payload put which model where, with which yaw.  Written out on
/// every race start (see [`Track::placement_log`]) so a misplaced rail
/// can be traced back to its exact cell, kind and argument.
#[derive(Clone, Debug)]
pub struct Placement {
    /// `tile`, `mid` or `high`.
    pub layer: &'static str,
    /// `.map` cell.
    pub cell: (i32, i32),
    /// Tile kind, `.md` kind or `.hd` kind.
    pub kind: u8,
    /// Payload argument (yaw selector / mirror side).
    pub arg: u8,
    /// Model resource as placed (after the object lookup for `high`).
    pub model: String,
    /// Texture resource as placed (after the seasonal swap).
    pub texture: String,
    /// Game-space origin (cell origin plus the `.hd` offset, if any).
    pub origin: [f32; 3],
    /// Radians about the game's up axis (0 for flagged trees).
    pub yaw: f32,
    /// `.ob` flag for `high` (identity rotation when set), else false.
    pub flagged: bool,
}

struct Builder<'a> {
    res: &'a Resources,
    batches: HashMap<String, Batch>,
    /// Texture coordinate rectangle each texture's models cover, so the atlas
    /// can be tiled to suit them.
    uv_bounds: HashMap<String, [f32; 4]>,
    placements: Vec<Placement>,
}

impl<'a> Builder<'a> {
    fn new(res: &'a Resources) -> Builder<'a> {
        Builder {
            res,
            batches: HashMap::new(),
            uv_bounds: HashMap::new(),
            placements: Vec::new(),
        }
    }

    /// Transform one model instance and append it to its texture batch.
    fn place(
        &mut self,
        placement: Placement,
        scale: [f32; 3],
    ) {
        let model_path = placement.model.clone();
        let texture_path = placement.texture.clone();
        let yaw = placement.yaw;
        let origin = placement.origin;
        let Some(model) = parse_model(self.res, &model_path) else {
            return;
        };
        self.placements.push(placement);
        // Only the shared tile atlas is adjustable: everything else already
        // matches the artwork it names.
        let is_atlas = texture_path.ends_with("texpack.png")
            || texture_path.ends_with("ts.png")
            || texture_path.ends_with("td.png")
            || texture_path.ends_with("tf.png")
            || texture_path.ends_with("ts2.png")
            || texture_path.ends_with("td2.png")
            || texture_path.ends_with("tf2.png");

        let bounds = self
            .uv_bounds
            .entry(texture_path.to_string())
            .or_insert([f32::MAX, f32::MIN, f32::MAX, f32::MIN]);
        for &[u, v] in &model.texcoords {
            bounds[0] = bounds[0].min(u);
            bounds[1] = bounds[1].max(u);
            bounds[2] = bounds[2].min(v);
            bounds[3] = bounds[3].max(v);
        }

        let (cos, sin) = (yaw.cos(), yaw.sin());
        let mut geometry: Vec<[Vec3; 3]> = Vec::new();
        let mut uvs: Vec<[Vec2; 3]> = Vec::new();
        for face in model.triangles() {
            let mut points = [Vec3::ZERO; 3];
            let mut uv = [Vec2::ZERO; 3];
            for k in 0..3 {
                let i = face[k];
                let v = model.positions[i];
                let gx = v[0] * scale[0];
                let gy = v[1] * scale[1];
                let gz = v[2] * scale[2];
                // The instance's rotation is about the game's up axis, so it is
                // applied in the game's own plane before the axes are swapped.
                let rx = gx * cos - gy * sin;
                let ry = gx * sin + gy * cos;
                points[k] = game_to_world(rx + origin[0], ry + origin[1], gz + origin[2]);
                uv[k] = if is_atlas {
                    atlas_uv(vec2(model.texcoords[i][0], model.texcoords[i][1]))
                } else {
                    vec2(model.texcoords[i][0], model.texcoords[i][1])
                };
            }
            geometry.push(points);
            uvs.push(uv);
        }

        let batch = self
            .batches
            .entry(texture_path.to_string())
            .or_insert_with(Batch::new);
        for (points, uv) in geometry.iter().zip(uvs.iter()) {
            batch.triangle((points[0], uv[0]), (points[1], uv[1]), (points[2], uv[2]));
        }

    }
}

fn parse_model(res: &Resources, path: &str) -> Option<format::Model> {
    res.get(path.trim_start_matches('/'))
        .and_then(|bytes| format::Model::parse(bytes))
}

/// Fence rails as local arm segments `(x0, y0, x1, y1)` in tile units,
/// measured off the model vertices. The game collides its scenery
/// (`ai.a`), so rails are solid there; the port used to leave every
/// detail instance intangible, and cars drove straight through roadside
/// rails. Trees, rocks, bushes and canopy stay intangible (their
/// solidity is unverified, and foliage overhangs the road by design).
fn rail_arms(model: &str) -> Option<&'static [[f32; 4]]> {
    Some(match model {
        // Ribbon along local Y.
        "models/za" => &[[0.0, -1.0, 0.0, 1.0]],
        // Bent corner rails.
        "models/zazd" => &[[-1.0, -0.62, 0.17, -0.17], [0.17, -0.17, 0.62, 1.0]],
        // Corner with wings.
        "models/zc" => &[[-0.1, 0.52, -0.1, -0.52], [-0.2, 0.0, 0.0, -1.0], [-0.2, 0.0, 0.0, 1.0]],
        // Panel perimeters.
        "models/zdn" | "models/zdzn" => &[[-0.62, 0.5, 0.62, 0.5], [-0.62, -0.5, 0.62, -0.5], [0.62, -1.0, 0.62, 1.0], [-0.62, -1.0, -0.62, 1.0]],
        // Corner panels (the zdk pair carries an extra wing).
        "models/zdkn" | "models/zdkzn" => &[[-0.62, 1.0, -0.17, -0.17], [-0.17, -0.17, 1.0, -0.62], [0.62, 1.0, 1.0, 0.62]],
        "models/zn" | "models/zzdn" => &[[-0.62, 1.0, -0.17, -0.17], [-0.17, -0.17, 1.0, -0.62]],
        // Corner bit and posts.
        "models/zazk" => &[[0.62, 1.0, 1.0, 0.62]],
        "models/z2" | "models/z3" => &[[0.0, 0.0, 0.0, 0.0]],
        _ => return None,
    })
}

/// How far along `from -> to` the chase camera may sit before a wall gets
/// between it and the car, as a fraction (1.0 = clear). Tests the segment
/// against every wall box in the ground plane, expanded by the camera
/// margin, and stops just short of the nearest hit. A box holding the car
/// itself is skipped: a wedged car stays visible through the rail (a brief
/// clip) instead of hiding behind a wall close-up. The free camera flies
/// anywhere on purpose; the chase camera should never slice through rails
/// and walls it just watched the car crash into.
pub fn camera_pull_in(walls: &[(Vec3, Vec3)], from: Vec3, to: Vec3) -> f32 {
    let mut best = 1.0f32;
    let margin = 0.5;
    for (centre, half) in walls {
        if (to.x - centre.x).abs() < half.x
            && (to.z - centre.z).abs() < half.z
            && (to.y - centre.y).abs() < half.y + margin
        {
            continue;
        }
        let lo_x = centre.x - half.x - margin;
        let hi_x = centre.x + half.x + margin;
        let lo_z = centre.z - half.z - margin;
        let hi_z = centre.z + half.z + margin;
        // Slab march: earliest fraction where the segment enters the box.
        let dx = to.x - from.x;
        let dz = to.z - from.z;
        let mut tmin = 0.0f32;
        let mut tmax = 1.0f32;
        let mut ok = true;
        for (p, d, lo, hi) in [
            (from.x, dx, lo_x, hi_x),
            (from.z, dz, lo_z, hi_z),
        ] {
            if d.abs() < 1e-6 {
                if p < lo || p > hi {
                    ok = false;
                    break;
                }
            } else {
                let mut t0 = (lo - p) / d;
                let mut t1 = (hi - p) / d;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                tmin = tmin.max(t0);
                tmax = tmax.min(t1);
                if tmin > tmax {
                    ok = false;
                    break;
                }
            }
        }
        if ok && tmin < best {
            best = tmin.max(0.0);
        }
    }
    // Never closer than a bonnet-length: fully blocked means riding the
    // bumper, not wearing it.
    best.max(0.12)
}

/// Detail remap from `bm.a(ae/ap, kind, arg)`: on theme 3 the MIDlet drops a
/// range of mid/high-detail indices, and `bp` swaps two object kinds.
fn themed_detail(theme: u8, kind: u8) -> u8 {
    if theme == 3
        && (kind < 19
            || kind == 22
            || kind == 28
            || kind == 29
            || kind == 30
            || (32..=36).contains(&kind))
    {
        0
    } else {
        kind
    }
}

fn themed_object(theme: u8, kind: u8) -> u8 {
    if theme == 3 {
        match kind {
            0 | 1 => 5,
            2 | 3 => 4,
            other => other,
        }
    } else {
        kind
    }
}

pub fn build(dir: &Path, res: &Resources, map_name: &str) -> Track {
    build_themed(dir, res, map_name, 0)
}

/// Build a track with a specific tile/texture variant (the campaign's `theme`).
pub fn build_themed(dir: &Path, res: &Resources, map_name: &str, theme: u8) -> Track {
    build_detailed(dir, res, map_name, theme, Detail::Full)
}

/// Build a track at a graphics detail: `Detail::Base` skips the mid-detail
/// layer and `Detail::Mid` skips the high-detail one, which is what the
/// MIDlet's detail setting does.
pub fn build_detailed(
    dir: &Path,
    res: &Resources,
    map_name: &str,
    theme: u8,
    detail: Detail,
) -> Track {
    let tile_list = pack::lines(&pack::read_jar_file(dir, "lists/tile_list"));
    let md_list = pack::lines(&pack::read_jar_file(dir, "lists/md_list"));
    let hd_list = pack::lines(&pack::read_jar_file(dir, "lists/hd_list"));
    let ob_list = pack::lines(&pack::read_jar_file(dir, "lists/ol"));

    let map_path = format!("levels/{map_name}");
    let map = format::Map::parse(
        res.get(&map_path)
            .unwrap_or_else(|| panic!("resource {map_path} not in pack")),
    )
    .expect("malformed .map");

    let mut builder = Builder::new(res);
    // Rail arms (game-plane segments with the instance yaw and origin) to
    // wall off once the surface exists for their heights.
    let mut rail_segs: Vec<(f32, f32, f32, &'static [[f32; 4]])> = Vec::new();
    // Definition caches, keyed by the 1-based index stored in the map.
    let mut tiles: HashMap<u8, format::Tile> = HashMap::new();
    let mut mids: HashMap<u8, format::MidDetail> = HashMap::new();
    let mut highs: HashMap<u8, format::HighDetail> = HashMap::new();
    let mut objects: HashMap<u8, format::ObjectDef> = HashMap::new();

    let half = std::f32::consts::FRAC_PI_2;
    let list_entry = |list: &[String], index: u8| -> Option<String> {
        list.get((index as usize).checked_sub(1)?).cloned()
    };

    for (y, row) in map.cells.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            let ox = x as f32 * TILE;
            let oy = y as f32 * TILE;

            if let Some((kind, arg)) = cell.tile {
                if kind > 0 {
                    if !tiles.contains_key(&kind) {
                        if let Some(file) = list_entry(&tile_list, kind) {
                            if let Some(tile) = res
                                .get(&format!("tiles/{file}"))
                                .and_then(|bytes| format::Tile::parse(bytes))
                            {
                                tiles.insert(kind, tile);
                            }
                        }
                    }
                    if let Some(tile) = tiles.get(&kind) {
                        builder.place(
                            Placement {
                                layer: "tile",
                                cell: (x as i32, y as i32),
                                kind,
                                arg,
                                model: format!("models/p/{}", tile.name),
                                texture: tile_atlas(theme, &tile.texture),
                                origin: [ox, oy, 0.0],
                                yaw: arg as f32 * half,
                                flagged: false,
                            },
                            [WORLD_SCALE; 3],
                        );
                    }
                }
            }

            for &(kind, arg) in &cell.mid {
                if detail == Detail::Base {
                    continue;
                }
                let kind = themed_detail(theme, kind);
                if kind == 0 {
                    continue;
                }
                if !mids.contains_key(&kind) {
                    if let Some(file) = list_entry(&md_list, kind) {
                        if let Some(md) = res
                            .get(&format!("tiles/{file}"))
                            .and_then(|bytes| format::MidDetail::parse(bytes))
                        {
                            mids.insert(kind, md);
                        }
                    }
                }
                if let Some(md) = mids.get(&kind) {
                    builder.place(
                        Placement {
                            layer: "mid",
                            cell: (x as i32, y as i32),
                            kind,
                            arg,
                            model: format!("models/{}", md.model),
                            texture: detail_texture(theme, &md.texture),
                            origin: [ox, oy, 0.0],
                            yaw: arg as f32 * half,
                            flagged: false,
                        },
                        [WORLD_SCALE; 3],
                    );
                }
            }

            for &(kind, arg) in &cell.high {
                if detail != Detail::Full {
                    continue;
                }
                let kind = themed_detail(theme, kind);
                if kind == 0 {
                    continue;
                }
                if !highs.contains_key(&kind) {
                    if let Some(file) = list_entry(&hd_list, kind) {
                        if let Some(hd) = res
                            .get(&format!("tiles/{file}"))
                            .and_then(|bytes| format::HighDetail::parse(bytes))
                        {
                            highs.insert(kind, hd);
                        }
                    }
                }
                let Some(hd) = highs.get(&kind) else { continue };
                // `bp` places each entry at half-tile offsets, mirrored per side.
                for entry in &hd.entries {
                    let object_kind = themed_object(theme, entry.kind);
                    if object_kind == 0 {
                        continue;
                    }
                    if !objects.contains_key(&object_kind) {
                        if let Some(file) = list_entry(&ob_list, object_kind) {
                            if let Some(ob) = res
                                .get(&format!("objects/{file}"))
                                .and_then(|bytes| format::ObjectDef::parse(bytes))
                            {
                                objects.insert(object_kind, ob);
                            }
                        }
                    }
                    let Some(object) = objects.get(&object_kind) else {
                        continue;
                    };
                    let px = 7.0 * entry.position[0];
                    let py = 7.0 * entry.position[1];
                    let pz = -7.0 * entry.position[2];
                    let (lx, ly) = match arg {
                        0 => (ox - py, oy - px),
                        1 => (ox - px, oy - py),
                        2 => (ox + py, oy + px),
                        _ => (ox + px, oy + py),
                    };
                    let model = format!("models/{}", object.model);
                    let yaw = high_detail_yaw(object.flag, arg);
                    builder.place(
                        Placement {
                            layer: "high",
                            cell: (x as i32, y as i32),
                            kind,
                            arg,
                            model: model.clone(),
                            texture: object_texture(theme, &object.texture),
                            origin: [lx, ly, pz],
                            yaw,
                            flagged: object.flag,
                        },
                        [WORLD_SCALE; 3],
                    );
                    if let Some(arms) = rail_arms(&model) {
                        rail_segs.push((lx, ly, yaw, arms));
                    }
                }
            }
        }
    }

    let origin = [map.start.0 as f32 * TILE, map.start.1 as f32 * TILE, 0.0];
    let grid = Grid::build(&map, &tiles);
    // Start facing the way the circuit is raced.
    let (spawn, spawn_yaw) = grid
        .grid_slots(1)
        .first()
        .copied()
        .map(|(position, yaw)| (position, yaw))
        .unwrap_or((
            vec3(origin[0], 1.2, -origin[1]),
            0.0,
        ));

    let Builder {
        batches,
        uv_bounds,
        placements,
        ..
    } = builder;

    // Physics ground.  The MIDlet samples each tile's collision mesh for
    // height and falls back to a flat plane at zero when the tile has none -
    // which is most of them (`s.tl` and friends ship no collision data).
    //
    // The collider is that height function: a sub-divided patch over every
    // cell whose tile carries a mesh, a flat quad otherwise.  A mesh can cover
    // only part of its cell, so sampling it on a grid gives one surface rather
    // than overlapping sheets.  Steps between neighbouring cells are left as
    // they are and the car is lifted onto the surface (see `World::conform`),
    // which is how the MIDlet drives its own car over them.
    let mut collision_vertices: Vec<[f32; 3]> = Vec::new();
    let mut collision_indices: Vec<[u32; 3]> = Vec::new();
    let mut walls: Vec<(Vec3, Vec3)> = Vec::new();
    let half_tile = TILE * 0.5;

    let tile_at = |x: i32, y: i32| -> Option<(u8, u8)> {
        if !grid.occupied(x, y) {
            return None;
        }
        map.cells[y as usize][x as usize].tile
    };
    let mesh_of = |kind: u8| -> Option<&format::Collision> {
        tiles
            .get(&kind)
            .and_then(|tile| tile.collision.as_ref())
            .filter(|collision| !collision.vertices.is_empty())
    };

    for (x, y) in grid.path() {
        let centre = grid.center(x, y);
        let (kind, arg) = tile_at(x, y).unwrap();
        let mesh = mesh_of(kind);

        if mesh.is_none() {
            let base = collision_vertices.len() as u32;
            for (dx, dz) in [
                (-half_tile, -half_tile),
                (-half_tile, half_tile),
                (half_tile, half_tile),
                (half_tile, -half_tile),
            ] {
                collision_vertices.push([centre.x + dx, 0.0, centre.z + dz]);
            }
            collision_indices.push([base, base + 1, base + 2]);
            collision_indices.push([base, base + 2, base + 3]);
        } else {
            let steps = SURFACE_STEPS;
            for i in 0..steps {
                for j in 0..steps {
                    let lo = vec2(i as f32, j as f32) / steps as f32;
                    let hi = vec2((i + 1) as f32, (j + 1) as f32) / steps as f32;
                    let corner = |local: Vec2| {
                        world_of(centre, local, surface_height(mesh, None, arg, local))
                    };
                    let (a, b, c, d) = (
                        corner(vec2(lo.x, lo.y)),
                        corner(vec2(lo.x, hi.y)),
                        corner(vec2(hi.x, hi.y)),
                        corner(vec2(hi.x, lo.y)),
                    );
                    push_triangle(&mut collision_vertices, &mut collision_indices, a, b, c);
                    push_triangle(&mut collision_vertices, &mut collision_indices, a, c, d);
                }
            }
        }

        // A barrier on every side that is not drivable, standing on the
        // surface rather than at zero.  Sloped cells sample the whole edge:
        // a barrier at the midpoint height floats above the low end of a ramp
        // and cars slip under it, so the wall runs from the edge's lowest
        // surface point to 2.2 above its highest.
        //
        // Sides facing off the map get a barrier even when the tile calls
        // them open: beyond is the void past the world edge, and driving out
        // there is falling out of the world, not racing.  The same goes for
        // open sides facing an empty hole inside the map (23 of them across
        // 13 shipped maps, e.g. Timberton's (4, 7) west and (9, 7) south
        // spurs): no tile there means no ground either, so they get walls
        // too.  (The game rejoins fallers; the port keeps them in to begin
        // with.)
        for dir in 0..4 {
            let (dx, dy) = crate::grid::DIRS[dir];
            let (nx, ny) = (x + dx, y + dy);
            // `occupied` is false past the world edge too, so this covers
            // both the off-map void and the in-map holes.
            let road = grid.occupied(nx, ny);
            if grid.open_sides(x, y) & (1 << dir) != 0 && road {
                continue;
            }
            let edge = match dir {
                0 => [vec2(1.0, 0.0), vec2(1.0, 0.5), vec2(1.0, 1.0)],
                1 => [vec2(0.0, 0.0), vec2(0.5, 0.0), vec2(1.0, 0.0)],
                2 => [vec2(0.0, 0.0), vec2(0.0, 0.5), vec2(0.0, 1.0)],
                _ => [vec2(0.0, 1.0), vec2(0.5, 1.0), vec2(1.0, 1.0)],
            };
            let base = edge
                .iter()
                .map(|sample| surface_height(mesh, None, arg, *sample))
                .fold(f32::MAX, f32::min);
            let top = edge
                .iter()
                .map(|sample| surface_height(mesh, None, arg, *sample))
                .fold(f32::MIN, f32::max)
                + 2.2;
            let direction = crate::grid::dir_mq(dir);
            let (hx, hz) = if dir % 2 == 0 {
                (0.6, half_tile)
            } else {
                (half_tile, 0.6)
            };
            walls.push((
                centre + direction * half_tile + vec3(0.0, base, 0.0),
                vec3(hx, (top - base) / 2.0, hz),
            ));
        }
    }

    let mut meshes = Vec::new();
    let mut texture_paths = Vec::new();
    for (texture_path, batch) in batches {
        for part in batch.parts {
            if part.verts.is_empty() {
                continue;
            }
            meshes.push(Mesh {
                vertices: part.verts,
                indices: part.idx.iter().map(|&i| i as u16).collect(),
                texture: None,
            });
            texture_paths.push(texture_path.clone());
        }
    }

    // Visual triangles per tile kind, in cell-fraction plan space with
    // macroquad heights, backing the collider where a tile ships no
    // collision mesh of its own.
    let mut visuals: HashMap<u8, std::rc::Rc<VisualTris>> = HashMap::new();
    for (&kind, tile) in tiles.iter() {
        if tile.collision.as_ref().is_some_and(|c| !c.vertices.is_empty()) {
            continue;
        }
        if let Some(model) = parse_model(res, &format!("models/p/{}", tile.name)) {
            let tris: VisualTris = model
                .triangles()
                .into_iter()
                .map(|face| {
                    face.map(|i| {
                        let v = model.positions[i];
                        [(v[0] + 1.0) * 0.5, (v[1] + 1.0) * 0.5, -v[2] * WORLD_SCALE]
                    })
                })
                .collect();
            if !tris.is_empty() {
                visuals.insert(kind, std::rc::Rc::new(tris));
            }
        }
    }

    let surface = SurfaceGrid {
        width: grid.width,
        height: grid.height,
        cells: (0..grid.height)
            .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                map.cells[y as usize][x as usize].tile.map(|(kind, arg)| {
                    (
                        arg,
                        tiles
                            .get(&kind)
                            .and_then(|tile| tile.collision.clone())
                            .filter(|collision| !collision.vertices.is_empty()),
                        visuals.get(&kind).cloned(),
                    )
                })
            })
            .collect(),
    };

    // Rails are solid: one thin wall per arm, standing on the surface
    // under its midpoint (grass verges have no mesh, so those fall back
    // to the placement height). Like the edge barriers these only steer
    // the kinematic slide, never beach. Arm ends inset a metre so they
    // stop at the tarmac edge instead of snagging it: the artwork anchors
    // rail ends onto the road itself, and full-length hitboxes hook every
    // car that brushes past.
    for (lx, ly, yaw, arms) in &rail_segs {
        let (cos, sin) = (yaw.cos(), yaw.sin());
        for arm in arms.iter() {
            let (ax, ay, bx, by) = (arm[0], arm[1], arm[2], arm[3]);
            let length = (bx - ax).hypot(by - ay).max(1e-6);
            // Points (posts) keep their full hitbox.
            let inset = if length < 0.1 {
                0.0
            } else {
                (0.15 / length).min(0.49)
            };
            let mut corners = Vec::with_capacity(2);
            for [px, py] in [
                [ax + (bx - ax) * inset, ay + (by - ay) * inset],
                [bx - (bx - ax) * inset, by - (by - ay) * inset],
            ] {
                let gx = px * WORLD_SCALE;
                let gy = py * WORLD_SCALE;
                let point = game_to_world(
                    gx * cos - gy * sin + lx,
                    gx * sin + gy * cos + ly,
                    0.0,
                );
                corners.push(point);
            }
            let mid = (corners[0] + corners[1]) * 0.5;
            let ground = surface
                .support_height(mid, vec3(0.0, 0.0, -1.0), 0.5, mid.y + 2.0, 0.2)
                .unwrap_or(0.0);
            let half = vec3(
                (corners[0].x - corners[1].x).abs() * 0.5 + 0.4,
                1.4,
                (corners[0].z - corners[1].z).abs() * 0.5 + 0.4,
            );
            walls.push((vec3(mid.x, ground + 0.8, mid.z), half));
        }
    }

    Track {
        grid,
        surface,
        meshes,
        texture_paths,
        uv_bounds,
        collision_vertices,
        collision_indices,
        walls,
        spawn,
        spawn_yaw,
        placements,
    }
}

impl Track {
    /// Human-readable placement dump: one line per placed instance plus
    /// the grid slots and the barriers.  Written to
    /// `placements-<map>.log` on every race start so a misplaced rail can
    /// be traced to its cell, kind and argument.  Origins are game-space
    /// `(x, y, z)`, yaws are radians about the game's up axis.
    pub fn placement_log(&self, map_name: &str, theme: u8) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# placements for {map_name} theme {theme}: {} instances, {} walls\n",
            self.placements.len(),
            self.walls.len()
        ));
        out.push_str(&format!(
            "# race_dir {:?} spawn ({:.1},{:.1},{:.1}) yaw {:.3}\n",
            self.grid.race_dir(),
            self.spawn.x,
            self.spawn.y,
            self.spawn.z,
            self.spawn_yaw
        ));
        out.push_str("# cell layer kind arg model texture origin yaw flagged\n");
        for placement in &self.placements {
            out.push_str(&format!(
                "{},{} {} {} {} {} {} [{:.2},{:.2},{:.2}] {:.3} {}\n",
                placement.cell.0,
                placement.cell.1,
                placement.layer,
                placement.kind,
                placement.arg,
                placement.model,
                placement.texture,
                placement.origin[0],
                placement.origin[1],
                placement.origin[2],
                placement.yaw,
                placement.flagged as u8
            ));
        }
        out.push_str("# walls as macroquad centre + half extents\n");
        for (centre, half) in &self.walls {
            out.push_str(&format!(
                "wall [{:.2},{:.2},{:.2}] [{:.2},{:.2},{:.2}]\n",
                centre.x, centre.y, centre.z, half.x, half.y, half.z
            ));
        }
        out.push_str("# grid slots as macroquad position + yaw\n");
        for (position, yaw) in self.grid.grid_slots(4) {
            out.push_str(&format!(
                "slot [{:.2},{:.2},{:.2}] {:.3}\n",
                position.x, position.y, position.z, yaw
            ));
        }
        out
    }
}

/// A car body in macroquad space, centred on its local origin.
pub struct CarGeometry {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub texture_path: String,
    /// Texture coordinate rectangle the unwrap covers.
    pub uv_bounds: [f32; 4],
    pub half_extents: Vec3,
}

/// Build the player car from its `.car` descriptor (class `ba`).
///
/// Unlike tiles the car is *not* scaled by `ar.a`; the MIDlet only nudges it
/// with `at.b(0.96, 0.96, 0.9)`, so the natural model size is used.
pub fn build_car(res: &Resources, car: &format::Car) -> Option<CarGeometry> {
    let model = parse_model(res, &format!("models/{}", car.model))?;

    // Per-axis squash the MIDlet applies to the rally body.
    let scale = [0.96f32, 0.96, 0.9];
    let (min, max) = model.bounds();
    // The model stands its wheels on z = 0 and rises above it, so centring the
    // body on its own origin is what puts the wheels and the roof either side.
    let centre_y = 0.5 * (min[2] + max[2]) * scale[2];

    let (mut u0, mut v0) = (f32::MAX, f32::MAX);
    let (mut u1, mut v1) = (f32::MIN, f32::MIN);
    let mut vertices = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    for face in model.triangles() {
        let base = vertices.len() as u16;
        for &i in &face {
            let v = model.positions[i];
            let p = game_to_world(
                v[0] * scale[0],
                v[1] * scale[1],
                v[2] * scale[2] - centre_y,
            );
            let uv = model.texcoords[i];
            u0 = u0.min(uv[0]);
            u1 = u1.max(uv[0]);
            v0 = v0.min(uv[1]);
            v1 = v1.max(uv[1]);
            vertices.push(Vertex::new2(p, vec2(uv[0], uv[1]), WHITE));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    Some(CarGeometry {
        vertices,
        indices,
        texture_path: format!("tex/{}", car.texture),
        uv_bounds: [u0, u1, v0, v1],
        half_extents: vec3(
            (max[0] - min[0]) * 0.5 * scale[0],
            (max[2] - min[2]) * 0.5 * scale[2],
            (max[1] - min[1]) * 0.5 * scale[1],
        ),
    })
}

/// Upload a car texture, tiled over the unwrap's coordinates (see [`Repeat`]),
/// and pull the unwrap back into the tiled range.  Needs a live macroquad
/// graphics context.
pub fn load_car_texture(res: &Resources, geometry: &mut CarGeometry) -> Option<Texture2D> {
    let image = Image::from_file_with_format(
        res.get(geometry.texture_path.trim_start_matches('/'))?,
        Some(ImageFormat::Png),
    )
    .ok()?;
    // The car sheet is an unwrap whose parts sit edge to edge, so it wants the
    // same nearest filtering the game asks for - linear bleeds one panel's
    // paint into the next along every seam.
    let repeat = Repeat::build(&image, geometry.uv_bounds)?;
    repeat.texture.set_filter(texture_filter());
    for vertex in geometry.vertices.iter_mut() {
        vertex.uv = repeat.tiling().map(vertex.uv);
    }
    Some(repeat.texture.clone())
}
