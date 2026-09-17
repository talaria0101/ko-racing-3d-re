//! A player driving at a roadside bank must end stopped, not touring
//! the hillside. Timberton's bank south of the (8,5) curve is the
//! regression case: full throttle dead south, twelve seconds.

use kora::format;
use kora::grid::Grid;
use kora::physics::{CarControl, Tuning, World};
use kora::{pack, scene};
use macroquad::math::Vec3;

fn assets() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

#[test]
fn banks_stop_cars_instead_of_letting_them_climb() {
    let dir = assets();
    let resources = pack::load(&dir);
    let track = scene::build(&dir, &resources, "ma1.map");
    let car = format::Car::parse(&resources["cars/rally.car"]).unwrap();
    let geometry = scene::build_car(&resources, &car).unwrap();
    let scene::Track {
        grid, surface, walls, ..
    } = track;
    let grid: Grid = grid;
    let wall_count = walls.len();
    let ride = geometry.half_extents.y + 0.02;
    let reach = geometry.half_extents.z + 0.5;

    let mut world = World::new(&walls);
    world.add_car(
        Vec3::new(112.0, 1.0, -70.0),
        0.0,
        Tuning::player([3, 5, 5, 1]),
        false,
    );
    let drive = [CarControl {
        throttle: 1.0,
        steer: 0.0,
        brake: false,
    }];
    let mut max_y: f32 = 0.0;
    for _ in 0..(12 * 60) {
        let mut heights = Vec::new();
        for index in 0..world.cars.len() {
            let (place, rotation) = world.pose(index);
            let heading = rotation * Vec3::new(0.0, 0.0, -1.0);
            heights.push(
                surface
                    .support_height(place, heading, reach, place.y, ride)
                    .map(|h| h + ride),
            );
        }
        let offroad: Vec<bool> = (0..world.cars.len())
            .map(|index| {
                let place = world.position(index);
                match grid.progress_at(place) {
                    None => true,
                    Some(f) => {
                        let a = grid.line_point(f, 0.0);
                        let b = grid.line_point(f, 1.0);
                        match (a, b) {
                            (Some(a), Some(b)) => {
                                let abx = b.x - a.x;
                                let abz = b.z - a.z;
                                let len2 = (abx * abx + abz * abz).max(1e-6);
                                let t = (((place.x - a.x) * abx + (place.z - a.z) * abz)
                                    / len2)
                                    .clamp(0.0, 1.0);
                                let dx = place.x - (a.x + abx * t);
                                let dz = place.z - (a.z + abz * t);
                                dx.hypot(dz) > 2.5
                            }
                            _ => true,
                        }
                    }
                }
            })
            .collect();
        world.step(1.0 / 60.0, &drive, &heights, &offroad);
        max_y = max_y.max(world.position(0).y);
    }
    // The tarmac-edge system itself must stay up: Timberton carries
    // hundreds of gated chunks (rails, voids and verge walls are the
    // other hundred).
    assert!(wall_count > 80, "tarmac walls gone: {wall_count}");
    let end = world.position(0);
    assert!(max_y < 1.5, "climbed the bank to {max_y:.2}: {end:?}");
    // Never up the bank and never far along it either: stopped at the
    // roadside (a wall grind reads speed while standing still, so pin
    // the place, not the speedometer).
    let start = Vec3::new(112.0, 1.0, -70.0);
    assert!(
        (end.x - start.x).hypot(end.z - start.z) < 12.0,
        "left the roadside: {end:?}"
    );
}

