use game_build_tools::level::LevelSpec;

fn main() {
    println!("cargo:rerun-if-changed=src/assets/level_specs");

    let (tile_sheet, level) = LevelSpec::new(
        image::open("src/assets/level_specs/test_layout.png")
            .expect("Failed to load level layout")
            .into(),
        image::open("src/assets/level_specs/environment.png")
            .expect("Failed to load sprite sheet")
            .into(),
        (32, 32),
    )
    .register((255, 0, 0), (0, 0))
    .register((0, 0, 0), (0, 2))
    .compile()
    .expect("Failed to compile level spec");

    level
        .render(&tile_sheet)
        .expect("Failed to render test level")
        .save("src/assets/level_generated/test.png")
        .expect("Failed to save test level image");
}
