use image::{self, GenericImageView};
use std::collections::HashMap;

#[derive(Debug)]
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

        Ok(Texture {
            width,
            height,
            pixels,
        })
    }
}

pub struct TextureManager {
    textures: HashMap<String, Texture>,
}

impl TextureManager {
    pub fn new() -> Self {
        TextureManager {
            textures: HashMap::new(),
        }
    }

    pub fn load_texture(&mut self, name: String, path: &str) -> Result<(), image::ImageError> {
        let texture = Texture::from_file(path)?;
        self.textures.insert(name, texture);
        Ok(())
    }

    pub fn get_texture(&self, name: &str) -> Option<&Texture> {
        self.textures.get(name)
    }
}

pub fn load_game_textures(texture_manager: &mut TextureManager) -> Result<(), image::ImageError> {
    texture_manager.load_texture("character2".to_string(), "assets/character2.png")?;
    texture_manager.load_texture("character3".to_string(), "assets/character3.png")?;
    texture_manager.load_texture("character4".to_string(), "assets/character4.png")?;
    // navigator icon used for the minimap player indicator
    texture_manager.load_texture("navigator".to_string(), "assets/navigator.png")?;
    Ok(())
}
