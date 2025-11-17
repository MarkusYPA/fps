use rusttype::{point, Font, Scale};
use crate::consts::{HEIGHT, WIDTH};

pub fn draw_text(
    frame: &mut [u8],
    font: &Font,
    text: &str,
    x: usize,
    y: usize,
    color: [u8; 4],
) {
    let scale = Scale::uniform(24.0);
    let v_metrics = font.v_metrics(scale);
    let layout = font.layout(text, scale, point(x as f32, y as f32 + v_metrics.ascent));

    for g in layout {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|gx, gy, gv| {
                let gx = gx as i32 + bb.min.x;
                let gy = gy as i32 + bb.min.y;

                if gx >= 0 && gx < WIDTH as i32 && gy >= 0 && gy < HEIGHT as i32 {
                    let idx = (gy as usize * WIDTH + gx as usize) * 4;
                    if idx + 3 < frame.len() {
                        let pixel_alpha = (gv * 255.0) as u8;
                        let bg_r = frame[idx];
                        let bg_g = frame[idx + 1];
                        let bg_b = frame[idx + 2];
                        let bg_a = frame[idx + 3];

                        let r = (color[0] as u16 * pixel_alpha as u16 + bg_r as u16 * (255 - pixel_alpha) as u16) / 255;
                        let g = (color[1] as u16 * pixel_alpha as u16 + bg_g as u16 * (255 - pixel_alpha) as u16) / 255;
                        let b = (color[2] as u16 * pixel_alpha as u16 + bg_b as u16 * (255 - pixel_alpha) as u16) / 255;
                        let a = (color[3] as u16 * pixel_alpha as u16 + bg_a as u16 * (255 - pixel_alpha) as u16) / 255;

                        frame[idx] = r as u8;
                        frame[idx + 1] = g as u8;
                        frame[idx + 2] = b as u8;
                        frame[idx + 3] = a as u8;
                    }
                }
            });
        }
    }
}
