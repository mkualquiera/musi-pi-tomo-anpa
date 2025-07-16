mod adjacency;

use std::collections::{HashMap, HashSet};

use image::{GenericImage, GenericImageView, RgbImage, RgbaImage};
use ndarray::Array2;

use crate::level::adjacency::match_adjacency_rule;

pub fn alpha_blend_new(base: &RgbaImage, overlay: &RgbaImage, x: u32, y: u32) -> RgbaImage {
    let (base_width, base_height) = overlay.dimensions();
    let (overlay_width, overlay_height) = base.dimensions();

    // Create new image with same dimensions as base
    let mut result = overlay.clone();

    for dy in 0..overlay_height {
        for dx in 0..overlay_width {
            let base_x = x + dx;
            let base_y = y + dy;

            // Check bounds
            if base_x >= base_width || base_y >= base_height {
                continue;
            }

            let overlay_pixel = base.get_pixel(dx, dy);
            let base_pixel = result.get_pixel_mut(base_x, base_y);

            // Alpha blending formula: result = src * src_alpha + dst * (1 - src_alpha)
            let src_alpha = overlay_pixel[3] as f32 / 255.0;
            let inv_alpha = 1.0 - src_alpha;

            base_pixel[0] =
                ((overlay_pixel[0] as f32 * src_alpha) + (base_pixel[0] as f32 * inv_alpha)) as u8;
            base_pixel[1] =
                ((overlay_pixel[1] as f32 * src_alpha) + (base_pixel[1] as f32 * inv_alpha)) as u8;
            base_pixel[2] =
                ((overlay_pixel[2] as f32 * src_alpha) + (base_pixel[2] as f32 * inv_alpha)) as u8;

            // Blend alpha channels too
            base_pixel[3] =
                ((overlay_pixel[3] as f32 * src_alpha) + (base_pixel[3] as f32 * inv_alpha)) as u8;
        }
    }

    result
}

pub struct Neighborhood7x7 {
    data: [Option<u32>; 7 * 7],
}

impl Default for Neighborhood7x7 {
    fn default() -> Self {
        Self {
            data: [None; 7 * 7],
        }
    }
}

pub enum AbyssPolicy {
    PadWithSelf,
    PadWithAir,
}

impl Neighborhood7x7 {
    /// Get a value from the neighborhood using relative coordinates
    /// (0, 0) is the center, (-3, -3) is top-left, (3, 3) is bottom-right
    pub fn get(&self, dx: i32, dy: i32) -> Option<u32> {
        if !(-3..=3).contains(&dx) || !(-3..=3).contains(&dy) {
            return None;
        }
        let idx = ((dy + 3) * 7 + (dx + 3)) as usize;
        self.data[idx]
    }

    /// Set a value in the neighborhood using relative coordinates
    pub fn set(&mut self, dx: i32, dy: i32, value: Option<u32>) {
        if !(-3..=3).contains(&dx) || !(-3..=3).contains(&dy) {
            return;
        }
        let idx = ((dy + 3) * 7 + (dx + 3)) as usize;
        self.data[idx] = value;
    }

    /// Get the center value (equivalent to get(0, 0))
    pub fn center(&self) -> Option<u32> {
        self.get(0, 0)
    }

    /// Get all values in a specific row relative to center
    /// row -3 is the top row, row 3 is the bottom row
    pub fn row(&self, dy: i32) -> [Option<u32>; 7] {
        if !(-3..=3).contains(&dy) {
            return [None; 7];
        }
        let start_idx = ((dy + 3) * 7) as usize;
        let mut row = [None; 7];
        row.copy_from_slice(&self.data[start_idx..start_idx + 7]);
        row
    }

    /// Get all values in a specific column relative to center
    /// col -3 is the leftmost column, col 3 is the rightmost column
    pub fn col(&self, dx: i32) -> [Option<u32>; 7] {
        if !(-3..=3).contains(&dx) {
            return [None; 7];
        }
        let mut col = [None; 7];
        for dy in -3..=3 {
            let idx = ((dy + 3) * 7 + (dx + 3)) as usize;
            col[(dy + 3) as usize] = self.data[idx];
        }
        col
    }

    /// Iterator over all positions and their values
    pub fn iter(&self) -> impl Iterator<Item = ((i32, i32), Option<u32>)> + '_ {
        (-3..=3).flat_map(move |dy| {
            (-3..=3).map(move |dx| {
                let idx = ((dy + 3) * 7 + (dx + 3)) as usize;
                ((dx, dy), self.data[idx])
            })
        })
    }

    /// Get the raw data array (for compatibility with existing function signatures)
    pub fn raw_data(&self) -> [Option<u32>; 7 * 7] {
        self.data
    }
}

pub struct TileSheet {
    image: RgbaImage,
    num_tiles: (usize, usize),
    tile_mapping: HashMap<u32, (usize, usize)>,
    tile_inv_mapping: HashMap<(usize, usize), u32>,
}

impl TileSheet {
    pub fn new(image: RgbaImage, num_tiles: (usize, usize)) -> Self {
        Self {
            image,
            num_tiles,
            tile_mapping: HashMap::new(),
            tile_inv_mapping: HashMap::new(),
        }
    }

    pub fn new_with_tile_size(image: RgbaImage, tile_size: (u32, u32)) -> Self {
        let num_tiles = (
            image.width() as usize / tile_size.0 as usize,
            image.height() as usize / tile_size.1 as usize,
        );
        Self {
            image,
            num_tiles,
            tile_mapping: HashMap::new(),
            tile_inv_mapping: HashMap::new(),
        }
    }

    pub fn register(self, tile_id: u32, position: (usize, usize)) -> Self {
        let mut tile_sheet = self;
        if tile_sheet.tile_mapping.contains_key(&tile_id) {
            panic!("Tile ID {} already registered", tile_id);
        }
        if tile_sheet.tile_inv_mapping.contains_key(&position) {
            panic!(
                "Position {:?} already registered for another tile ID",
                position
            );
        }
        tile_sheet.tile_mapping.insert(tile_id, position);
        tile_sheet
    }

    pub fn allocate_tile_id(&mut self, position: (usize, usize)) -> u32 {
        if self.tile_inv_mapping.contains_key(&position) {
            return *self.tile_inv_mapping.get(&position).unwrap();
        }
        let tile_id = self.tile_inv_mapping.len() as u32;
        self.tile_mapping.insert(tile_id, position);
        self.tile_inv_mapping.insert(position, tile_id);
        tile_id
    }

    pub fn grab_tile(
        &self,
        tile_id: u32,
    ) -> std::option::Option<image::SubImage<&image::ImageBuffer<image::Rgba<u8>, std::vec::Vec<u8>>>>
    {
        if let Some(&(x, y)) = self.tile_mapping.get(&tile_id) {
            let tile_width = self.image.width() / self.num_tiles.0 as u32;
            let tile_height = self.image.height() / self.num_tiles.1 as u32;

            let x_start = x as u32 * tile_width;
            let y_start = y as u32 * tile_height;

            let result = self.image.view(x_start, y_start, tile_width, tile_height);
            Some(result)
        } else {
            None
        }
    }

    pub fn implied_tile_size(&self) -> (u32, u32) {
        let tile_width = self.image.width() / self.num_tiles.0 as u32;
        let tile_height = self.image.height() / self.num_tiles.1 as u32;
        (tile_width, tile_height)
    }

    pub fn clean_clone(&self) -> Self {
        Self {
            image: self.image.clone(),
            num_tiles: self.num_tiles,
            tile_mapping: HashMap::new(),
            tile_inv_mapping: HashMap::new(),
        }
    }

    pub fn canonical_autotile(&self, (start_x, start_y): (u32, u32), air: (u32, u32)) -> Self {
        // Canonical autotile defines an autotile region which is always 10 rows
        // and 5 cols rows, starting from the given position like so:
        // 0 5 10 ..
        // 1 6 11 ..
        // 2 7 12 ..
        // 3 8 13 ..
        // 4 9 14 ..
        let mut autotile = self.clean_clone();
        autotile.allocate_tile_id((air.0 as usize, air.1 as usize));
        for col in 0..10 {
            for row in 0..5 {
                let position = (start_x as usize + col, start_y as usize + row);
                autotile.allocate_tile_id(position);
            }
        }
        autotile
    }
}

pub struct LevelLayer {
    data: Array2<u32>,
}

impl LevelLayer {
    pub fn new(width: usize, height: usize) -> Self {
        let data = Array2::from_elem((height, width), 0);
        Self { data }
    }

    pub fn hardcoded(self, data: &[u32]) -> Self {
        let mut layer = self;
        assert!(
            data.len() == layer.data.len(),
            "Data length does not match layer size"
        );
        for (i, &value) in data.iter().enumerate() {
            let (row, col) = (i / layer.data.ncols(), i % layer.data.ncols());
            layer.data[[row, col]] = value;
        }
        layer
    }

    pub fn render(&self, tile_sheet: &TileSheet) -> Result<RgbaImage, String> {
        let (tile_width, tile_height) = tile_sheet.implied_tile_size();
        let mut image = RgbaImage::new(
            (self.data.ncols() * tile_width as usize) as u32,
            (self.data.nrows() * tile_height as usize) as u32,
        );

        for (y, row) in self.data.outer_iter().enumerate() {
            for (x, &tile_id) in row.iter().enumerate() {
                if let Some(tile_image) = tile_sheet.grab_tile(tile_id) {
                    let x_start = x as u32 * tile_width;
                    let y_start = y as u32 * tile_height;
                    image
                        .copy_from(&tile_image.to_image(), x_start, y_start)
                        .expect("Failed to copy tile image to level layer image");
                } else {
                    return Err(format!("Tile ID {} not found in tile sheet", tile_id));
                }
            }
        }

        Ok(image)
    }

    pub fn value_where<F: Fn(u32) -> bool>(&self, predicate: F, value: u32) -> LevelLayer {
        let mut new_layer = LevelLayer::new(self.data.ncols(), self.data.nrows());
        for (y, row) in self.data.outer_iter().enumerate() {
            for (x, &tile_id) in row.iter().enumerate() {
                if predicate(tile_id) {
                    new_layer.data[[y, x]] = value;
                } else {
                    new_layer.data[[y, x]] = 0;
                }
            }
        }
        new_layer
    }

    pub fn convolve<F: Fn(&Neighborhood7x7) -> u32>(&self, func: F) -> LevelLayer {
        let mut new_layer = LevelLayer::new(self.data.ncols(), self.data.nrows());
        let (rows, cols) = (self.data.nrows(), self.data.ncols());

        for y in 0..rows {
            for x in 0..cols {
                let mut neighborhood = Neighborhood7x7::default();

                // Fill the 7x7 neighborhood
                for dy in -3..=3 {
                    for dx in -3..=3 {
                        let ny = y as isize + dy;
                        let nx = x as isize + dx;

                        // Check if the neighbor position is within bounds
                        if ny >= 0 && ny < rows as isize && nx >= 0 && nx < cols as isize {
                            neighborhood.set(
                                dx as i32,
                                dy as i32,
                                Some(self.data[[ny as usize, nx as usize]]),
                            );
                        }
                    }
                }

                // Apply the convolution function with the clean interface
                let result = func(&neighborhood);
                new_layer.data[[y, x]] = result;
            }
        }

        new_layer
    }

    pub fn zip_with<F: Fn(u32, u32) -> u32>(&self, other: &LevelLayer, func: F) -> LevelLayer {
        assert_eq!(
            self.data.shape(),
            other.data.shape(),
            "Layers must have the same shape"
        );
        let mut new_layer = LevelLayer::new(self.data.ncols(), self.data.nrows());

        for (y, row) in self.data.outer_iter().enumerate() {
            for (x, &tile_id) in row.iter().enumerate() {
                let other_tile_id = other.data[[y, x]];
                new_layer.data[[y, x]] = func(tile_id, other_tile_id);
            }
        }

        new_layer
    }

    pub fn canonical_adjacency(&self, pad_with_adjacent: bool) -> LevelLayer {
        self.convolve(|neighborhood| {
            let get_at = |dx, dy| -> bool {
                if let Some(value) = neighborhood.get(dx, dy) {
                    value == 1
                } else {
                    pad_with_adjacent
                }
            };

            let neighborhood_adjacency = [
                get_at(-1, -1),
                get_at(0, -1),
                get_at(1, -1),
                get_at(-1, 0),
                get_at(0, 0),
                get_at(1, 0),
                get_at(-1, 1),
                get_at(0, 1),
                get_at(1, 1),
            ];
            let maybe_rule = match_adjacency_rule(&neighborhood_adjacency);
            if let Some(rule_index) = maybe_rule {
                (rule_index as u32) + 1
            } else {
                0 // Default to 0 if no rule matches
            }
        })
    }

    pub fn autotile_with(&self, value: u32, abyss: AbyssPolicy) -> LevelLayer {
        let mask = self.value_where(|tile_id| tile_id == value, 1);
        mask.canonical_adjacency(match abyss {
            AbyssPolicy::PadWithSelf => true,
            AbyssPolicy::PadWithAir => false,
        })
    }
}

type Color = (u8, u8, u8);
type TilePosition = (u32, u32);
type ColorMapEntry = (Color, TilePosition);

pub struct LevelSpec {
    layout: RgbImage,
    color_map: Vec<ColorMapEntry>,
    tile_size: (u32, u32),
    tileset: RgbaImage,
}

impl LevelSpec {
    pub fn new(layout: RgbImage, tileset: RgbaImage, tile_size: (u32, u32)) -> Self {
        Self {
            layout,
            color_map: Vec::new(),
            tile_size,
            tileset,
        }
    }

    pub fn register(self, color: (u8, u8, u8), tile_id: (u32, u32)) -> Self {
        let mut spec = self;
        if spec.color_map.iter().any(|&(c, _)| c == color) {
            panic!("Color {:?} already registered", color);
        }
        if spec.color_map.iter().any(|&(_, t)| t == tile_id) {
            panic!("Tile ID {:?} already registered for another color", tile_id);
        }
        spec.color_map.push((color, tile_id));
        spec
    }

    pub fn compile(self) -> Result<(TileSheet, LevelLayer), String> {
        let LevelSpec {
            layout,
            color_map,
            tile_size,
            tileset,
        } = self;

        let mut tile_sheet = TileSheet::new_with_tile_size(tileset, tile_size);
        let mut layer = LevelLayer::new(layout.width() as usize, layout.height() as usize);

        for &(_, tile_id) in &color_map {
            tile_sheet.allocate_tile_id((tile_id.0 as usize, tile_id.1 as usize));
        }

        let mut used_colors = HashSet::new();

        for (y, row) in layout.rows().enumerate() {
            for (x, pixel) in row.enumerate() {
                if let Some(tile_id) = {
                    color_map.iter().find_map(|&(c, t)| {
                        if c == (pixel[0], pixel[1], pixel[2]) {
                            Some(t)
                        } else {
                            None
                        }
                    })
                } {
                    let tile_id_u32 =
                        tile_sheet.allocate_tile_id((tile_id.0 as usize, tile_id.1 as usize));
                    layer.data[[y, x]] = tile_id_u32;
                    used_colors.insert((pixel[0], pixel[1], pixel[2]));
                } else {
                    return Err(format!(
                        "Color {:?} at ({}, {}) not registered in color map",
                        pixel, x, y
                    ));
                }
            }
        }

        // Ensure there are no missing colors
        for color in color_map.iter().map(|&(c, _)| c) {
            if !used_colors.contains(&color) {
                return Err(format!("Color {:?} was not used in the layout", color));
            }
        }
        for color in used_colors {
            if !color_map.iter().any(|&(c, _)| c == color) {
                return Err(format!("Color {:?} was used but not registered", color));
            }
        }

        Ok((tile_sheet, layer))
    }
}
