use std::hash::{DefaultHasher, Hash, Hasher};

use game_build_tools::level::{self, alpha_blend_new, AbyssPolicy, LevelSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn main() {
    println!("cargo:rerun-if-changed=src/assets/level_specs");

    let (tile_sheet, level_layer) = LevelSpec::new(
        image::open("src/assets/level_specs/test_layout.png")
            .expect("Failed to load level layout")
            .into(),
        image::open("src/assets/level_specs/environment.png")
            .expect("Failed to load sprite sheet")
            .into(),
        (32, 32),
    )
    .register((0, 0, 0), (0, 2))
    .register((255, 0, 0), (0, 1))
    .compile()
    .expect("Failed to compile level spec");

    level_layer
        .render(&tile_sheet)
        .expect("Failed to render test level")
        .save("src/assets/level_generated/test.png")
        .expect("Failed to save test level image");

    let collision_map = level_layer.value_where(|layer_val| layer_val == 1, 1);
    collision_map
        .dump_csv("src/assets/level_generated/collision.csv")
        .expect("Failed to dump collision map CSV");

    // Find the places where we should put front walls
    let wall_locations = level_layer.convolve(|neighborhood| {
        if neighborhood.get(0, 0) == Some(1)
            && neighborhood.get(0, -1) == Some(1)
            && neighborhood.get(0, 1) == Some(0)
        {
            1
        } else {
            0
        }
    });

    let ceiling_locations =
        level_layer.zip_with(
            &wall_locations,
            |original, wall| {
                if wall == 1 {
                    0
                } else {
                    original
                }
            },
        );

    let ceiling_autotile_sheet = tile_sheet.canonical_autotile((1, 5), (0, 2));
    let ceiling_autotile_layer = ceiling_locations.autotile_with(1, AbyssPolicy::PadWithSelf);

    let ceiling_image = ceiling_autotile_layer
        .render(&ceiling_autotile_sheet)
        .expect("Failed to render ceiling layer");

    let front_walls_image = wall_locations
        .render(&tile_sheet)
        .expect("Failed to render front walls layer");

    // Ambient occlusion
    let ao_locations = ceiling_locations.convolve(|neighborhood| {
        let top_value = neighborhood
            .get(0, -1)
            .unwrap_or(neighborhood.get(0, 0).unwrap());
        if top_value == 1 {
            0
        } else {
            1
        }
    });
    let ao_autotile_sheet = tile_sheet.canonical_autotile((1, 0), (0, 2));
    let ao_autotile_layer = ao_locations.autotile_with(1, AbyssPolicy::PadWithSelf);

    // We want the ao to be hidden by the ceiling, so we can replace the ceiling layer
    let ao_image = ao_autotile_layer
        .render(&ao_autotile_sheet)
        .expect("Failed to render AO layer");

    let ceiling_image = alpha_blend_new(&ceiling_image, &ao_image, 0, 0);

    // Merge the images
    let level_image = alpha_blend_new(&front_walls_image, &ceiling_image, 0, 0);

    level_image
        .save("src/assets/level_generated/test_with_walls.png")
        .expect("Failed to save level image with walls");

    let floor_tiles = tile_sheet.contiguous_tiles(&(0..=0), &(3..=6));

    let floor_layer = level_layer.ones_like().fill_with(|x, y| {
        let mut hasher = DefaultHasher::new();
        (x, y).hash(&mut hasher);
        let hash = hasher.finish();
        let mut rng = StdRng::seed_from_u64(hash);
        let tile_id = rng.random_range(1..(floor_tiles.count_registered_tiles() - 1));
        tile_id as u32
    });

    let floor_image = floor_layer
        .render(&floor_tiles)
        .expect("Failed to render floor layer");

    floor_image
        .save("src/assets/level_generated/floor.png")
        .expect("Failed to save floor image");
}
