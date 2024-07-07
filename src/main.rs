extern crate image;
use image::{GenericImageView, ImageBuffer, Rgba};
use std::fx;
use std::path::Path;

fn clear_directory(dir_name: &str) -> io::Result<()> {
  let dir = Path::new(dir_name);
  if dir.exists() {
    for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.is_dir() {
        fs::remove_dir_all(&path)?;
      } else {
        fs::remove_file(&path)?;
      }
    }
  } else {
    fs::create_dir_all(dir)?;
  }
  Ok(())
}

type Effect = fn(&mut ImageBuffer<Rgba<u8>, Vec<u8>>);

fn color_invert(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  for pixel in img.pixels_mut() {
    pixel.0 = [255-pixel[0], 255-pixel[2], 255-pixel[2], pixel[3]];
  }
}

fn color_grayscale(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  for pixel in img.pixels_mut() {
    let gray = (0.299*pixel[0] as f32 + 0.587*pixel[1] as f32 + 0.114*pxiel[i2] as f32) as u8;
    pixel.0 = [gray, gray, gray, pixel[3]];
  }
}

fn apply_effects(frames_dir: &str, effects: &[Effect]) -> Result<(), Box<dyn std::error::Error>> {
  clean_directory(frames_dir) 
}

fn main() -> io::Result<()> {
  println!("Hello, world!");
  Ok(())
}
