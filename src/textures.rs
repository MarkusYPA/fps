use image::{self, GenericImageView};
use std::collections::HashMap;

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl Texture {
    pub fn from_file(path: &str) -> Result<Self, image::ImageError> {
        let img = image::open(path)?;
        let (width, height) = img.dimensions();
        let mut pixels = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                // Convert RGBA to u32 (0xAARRGGBB)
                let color = ((pixel[3] as u32) << 24)
                    | ((pixel[0] as u32) << 16)
                    | ((pixel[1] as u32) << 8)
                    | (pixel[2] as u32);
                pixels.push(color);
            }
        }

        Ok(Texture { width, height, pixels })
    }
}

pub struct TextureManager {
    textures: Vec<Texture>,
    paths: HashMap<String, usize>,
}

impl TextureManager {
    pub fn new() -> Self {
        TextureManager {
            textures: Vec::new(),
            paths: HashMap::new(),
        }
    }

    pub fn load_texture(&mut self, path: &str) -> Result<usize, image::ImageError> {
        if let Some(idx) = self.paths.get(path) {
            return Ok(*idx);
        }

        let texture = Texture::from_file(path)?;
        let idx = self.textures.len();
        self.textures.push(texture);
        self.paths.insert(path.to_string(), idx);
        Ok(idx)
    }

    pub fn get_texture(&self, index: usize) -> Option<&Texture> {
        self.textures.get(index)
    }
}