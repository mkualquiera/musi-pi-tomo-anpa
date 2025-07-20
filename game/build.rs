use std::hash::{DefaultHasher, Hash, Hasher};

use game_build_tools::level::{alpha_blend_new, AbyssPolicy, LevelSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn build_level_basic(level_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (tile_sheet, level_layer) = LevelSpec::new(
        image::open(format!("src/assets/level_specs/{}_layout.png", level_name))
            .expect("Failed to load level layout")
            .into(),
        image::open("src/assets/level_specs/environment.png")
            .expect("Failed to load sprite sheet")
            .into(),
        (32, 32),
    )
    .register((0, 0, 0), (0, 2)) // air
    .register((255, 0, 0), (0, 1)) // wall
    .register((255, 255, 0), (0, 7)) // door
    .compile()?;

    level_layer
        .render(&tile_sheet)?
        .save(format!("src/assets/level_generated/{}.png", level_name))?;

    // Find the places where we should put front walls
    let wall_locations = level_layer.convolve(|neighborhood| {
        if neighborhood.get(0, 0) == Some(1)
            && neighborhood.get(0, -1) == Some(1)
            && neighborhood.get(0, 1) != Some(1)
            && neighborhood.get(0, 1).is_some()
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

    let ceiling_image = ceiling_autotile_layer.render(&ceiling_autotile_sheet)?;

    let front_walls_image = wall_locations.render(&tile_sheet)?;

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

    let door_shadow_tiles = tile_sheet
        .clean_clone()
        .register(0, (0, 2))
        .contiguous_tiles(&(0..=0), &(7..=10), false);

    let door_shadow_layer = level_layer.convolve(|neighborhood| {
        if neighborhood.get(0, 0) == Some(2) {
            if neighborhood.get(0, 1).is_none() {
                1
            } else if neighborhood.get(1, 0).is_none() {
                2
            } else if neighborhood.get(0, -1).is_none() {
                3
            } else if neighborhood.get(-1, 0).is_none() {
                4
            } else {
                0
            }
        } else {
            0
        }
    });

    let door_shadow_image = door_shadow_layer
        .render(&door_shadow_tiles)
        .expect("Failed to render door shadow layer");

    let ao_image = alpha_blend_new(&ao_image, &door_shadow_image, 0, 0);

    let ceiling_image = alpha_blend_new(&ao_image, &ceiling_image, 0, 0);

    // Merge the images
    let level_image = alpha_blend_new(&front_walls_image, &ceiling_image, 0, 0);

    level_image.save(format!(
        "src/assets/level_generated/{}_with_walls.png",
        level_name
    ))?;

    let floor_tiles = tile_sheet.contiguous_tiles(&(0..=0), &(3..=6), true);

    let floor_layer = level_layer.ones_like().fill_with(|x, y| {
        let mut hasher = DefaultHasher::new();
        (x, y).hash(&mut hasher);
        let hash = hasher.finish();
        let mut rng = StdRng::seed_from_u64(hash);
        let tile_id = rng.random_range(1..(floor_tiles.count_registered_tiles() - 1));
        tile_id as u32
    });

    let floor_image = floor_layer.render(&floor_tiles)?;

    floor_image.save(format!(
        "src/assets/level_generated/{}_floor.png",
        level_name
    ))?;

    let collision_layer = level_layer.zip_with(&door_shadow_layer, |original, door_shadow| {
        if door_shadow > 0 {
            door_shadow + 1
        } else {
            original
        }
    });

    collision_layer.dump_csv(&format!(
        "src/assets/level_generated/{}_collision.csv",
        level_name
    ))?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/assets/level_specs");

    build_level_basic("spawn")?;
    build_level_basic("base_0")?;

    Ok(())
}
