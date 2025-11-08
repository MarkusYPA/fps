use crate::{consts::CYAN_TRANSPARENT, textures::Texture};
use image::error::{ParameterError, ParameterErrorKind};
use image::{self, GenericImageView};

#[derive(Debug)]
pub struct SpriteSheet {
    pub idle: [Texture; 8],
    pub walk: [[Texture; 4]; 8],
}

impl SpriteSheet {
    pub fn new(path: &str) -> Result<Self, image::ImageError> {
        let img = image::open(path)?;

        // blob spritesheet frames are 276 x 338 pixels each with 4 vertical lines of pixels in between.
        let idle_frames_vec = Self::load_animation_frames(&img, 2, 2, 8, 8, 276, 338, 4, 2)?;

        let idle_frames: [Texture; 8] = idle_frames_vec.try_into().map_err(|_| {
            image::ImageError::Parameter(ParameterError::from_kind(ParameterErrorKind::Generic(
                "Incorrect number of idle frames".into(),
            )))
        })?;

        // blob spritesheet frames are 276 x 338 pixels each with 4 vertical and 2 horizontal lines of pixels in between.
        let walk_frames_vec = (0..8)
            .map(|i| -> Result<[Texture; 4], image::ImageError> {
                let frames =
                    Self::load_animation_frames(&img, 1, 342 + i * 340, 4, 4, 276, 338, 4, 2)?;
                frames.try_into().map_err(|_| {
                    image::ImageError::Parameter(ParameterError::from_kind(
                        ParameterErrorKind::Generic("Incorrect number of walk frames".into()),
                    ))
                })
            })
            .collect::<Result<Vec<[Texture; 4]>, _>>()?;

        let walk_frames: [[Texture; 4]; 8] = walk_frames_vec.try_into().map_err(|_| {
            image::ImageError::Parameter(ParameterError::from_kind(ParameterErrorKind::Generic(
                "Incorrect number of walk animation rows".into(),
            )))
        })?;

        Ok(SpriteSheet {
            idle: idle_frames,
            walk: walk_frames,
        })
    }

    fn load_animation_frames(
        img: &image::DynamicImage,
        start_x: u32,
        start_y: u32,
        num_frames: u32,
        frames_in_row: u32,
        frame_width: u32,
        frame_height: u32,
        gap_x: u32,
        gap_y: u32,
    ) -> Result<Vec<Texture>, image::ImageError> {
        let mut frames = Vec::new();
        for i in 0..num_frames {
            let x = start_x + (i % frames_in_row) * (frame_width + gap_x);
            let y = start_y + (i / frames_in_row) * (frame_height + gap_y);

            let frame_img = img.view(x, y, frame_width, frame_height);
            let mut pixels = Vec::with_capacity((frame_width * frame_height) as usize);
            for y_px in 0..frame_height {
                for x_px in 0..frame_width {
                    let pixel = frame_img.get_pixel(x_px, y_px);
                    let color = if pixel == CYAN_TRANSPARENT {
                        0 // Transparent
                    } else {
                        ((pixel[3] as u32) << 24)
                            | ((pixel[0] as u32) << 16)
                            | ((pixel[1] as u32) << 8)
                            | (pixel[2] as u32)
                    };
                    pixels.push(color);
                }
            }
            frames.push(Texture {
                pixels,
                width: frame_width,
                height: frame_height,
            });
        }
        Ok(frames)
    }
}
