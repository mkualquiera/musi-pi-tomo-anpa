use game_build_tools::level::{alpha_blend_new, AbyssPolicy, LevelSpec};

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

    // Merge the images
    let level_image = alpha_blend_new(&front_walls_image, &ceiling_image, 0, 0);

    level_image
        .save("src/assets/level_generated/test_with_walls.png")
        .expect("Failed to save level image with walls");
}
