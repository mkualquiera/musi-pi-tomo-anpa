mod adjacency;

use std::collections::{HashMap, HashSet};

use image::{GenericImage, GenericImageView, RgbImage, RgbaImage};
use ndarray::Array2;

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

    pub fn one_where<F: Fn(u32) -> bool>(&self, predicate: F) -> LevelLayer {
        let mut new_layer = LevelLayer::new(self.data.ncols(), self.data.nrows());
        for (y, row) in self.data.outer_iter().enumerate() {
            for (x, &tile_id) in row.iter().enumerate() {
                if predicate(tile_id) {
                    new_layer.data[[y, x]] = 1;
                } else {
                    new_layer.data[[y, x]] = 0;
                }
            }
        }
        new_layer
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
