extern crate image;
use image::{ImageBuffer, Rgba};
extern crate regex;
use regex::Regex;
use std::io;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::f64::consts::PI;

const SOBELHORIZ: [f32; 9] = [-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0];
const SOBELVERTI: [f32; 9] = [-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0];
const IMGOUT: &str = "img_out/";
const VIDIN: &str = "vid_in/";
const VIDOUT: &str = "vid_out/";
const BMP: &str = ".bmp";
const PNG: &str = ".png";
const GSF0: f32 = 0.299;
const GSF1: f32 = 0.587;
const GSF2: f32 = 0.114;
const FDF0: f64 = 7.0 / 16.0;
const FDF1: f64 = 3.0 / 16.0;
const FDF2: f64 = 5.0 / 16.0;
const FDF3: f64 = 1.0 / 16.0;

fn clear_directory(dir_name: &str) -> io::Result<()> {
  let dir = Path::new(dir_name);
  if dir.exists() {
    println!("clean_directory(): Cleaning directory {}", dir_name);
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
    println!("clean_directory(): Creating directory {}", dir_name);
    fs::create_dir_all(dir)?;
  }
  println!("clean_directory(): Successfully created/cleaned directory {}", dir_name);
  Ok(())
}

enum Effect {
  ColorInvert,
  ColorGrayscale,
  ColorFilter(f32, f32, f32),
  EdgeDetect,
  DiscreteCosine(u32),
  DiscreteSine(u32),
  FsDither,
}

impl Effect {
  fn apply(&self, img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    match *self {
      Effect::ColorInvert => color_invert(img),
      Effect::ColorGrayscale => color_grayscale(img),
      Effect::ColorFilter(r, g, b) => color_filter(img, r, g, b),
      Effect::EdgeDetect => edge_detect(img),
      Effect::DiscreteCosine(block_size) => discrete_cosine(img, block_size),
      Effect::DiscreteSine(block_size) => discrete_sine(img, block_size),
      Effect::FsDither => fs_dither(img),
    }
  }
}

fn color_invert(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  for pixel in img.pixels_mut() {
    pixel.0 = [255-pixel[0], 255-pixel[2], 255-pixel[2], pixel[3]];
  }
}

fn color_grayscale(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  for pixel in img.pixels_mut() {
    let gray = (GSF0*pixel[0] as f32 + GSF1*pixel[1] as f32 + GSF2*pixel[2] as f32) as u8;
    pixel.0 = [gray, gray, gray, pixel[3]];
  }
}

fn color_filter(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, r: f32, g: f32, b: f32) {
  for pixel in img.pixels_mut() {
    pixel[0] = (pixel[0] as f32 * r) as u8;
    pixel[1] = (pixel[1] as f32 * g) as u8;
    pixel[2] = (pixel[2] as f32 * b) as u8;
  }
}

fn edge_detect(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  let (width, height) = img.dimensions();
  let mut sobel_img = ImageBuffer::new(width, height);
  for y in 1..(height-1) {
    for x in 1..(width-1) {
      let mut gx = 0.0;
      let mut gy = 0.0;
      for ky in 0..3 {
        for kx in 0..3 {
          let pixel = img.get_pixel(x + kx - 1, y + ky -1).0;
          let intensity = (pixel[0] as f32 + pixel[1] as f32 + pixel[2] as f32) / 3.0;
          gx += intensity * SOBELHORIZ[(ky * 3 + kx) as usize];
          gy += intensity * SOBELVERTI[(ky * 3 + kx) as usize];
        }
      }
      let magnitude = ((gx * gx + gy * gy).sqrt()) as u8;
      sobel_img.put_pixel(x, y, Rgba([magnitude, magnitude, magnitude, 255]));
    }
  }
  *img = sobel_img;
}

fn dct_step(block: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
  let n = block.len();
  let mut dct = vec![vec![0.0; n]; n];
  let sqrt2 = (2.0f64).sqrt();
  for u in 0..n {
    for v in 0..n {
      let mut sum = 0.0;
      for i in 0..n {
        for j in 0..n {
          let cu = if u == 0 { 1.0 / sqrt2 } else { 1.0 };
          let cv = if v == 0 { 1.0 / sqrt2 } else { 1.0 };
          sum += cu * cv * block[i][j]
            * ((2.0 * i as f64 + 1.0) * u as f64 * PI / (2.0 * n as f64)).cos()
            * ((2.0 * j as f64 + 1.0) * v as f64 * PI / (2.0 * n as f64)).cos();
        }
      }
      dct[u][v] = sum * (2.0 / (n as f64).sqrt());
    }
  }
  dct
}

fn extract_block(img: &ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32, block_size: u32) -> Vec<Vec<f64>> {
  let mut block = vec![vec![0.0; block_size as usize]; block_size as usize];
  let (width, height) = img.dimensions();
  for i in 0..block_size {
    for j in 0..block_size {
      let px = x + j;
      let py = y + i;
      if px < width && py < height {
        let pixel = img.get_pixel(px, py);
        let gray = pixel[0] as f64 / 255.0;
        block[i as usize][j as usize] = gray;
      }
    }
  }
  block
}

fn store_block(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, block: Vec<Vec<f64>>, x: u32, y: u32, block_size: u32) {
  let (width, height) = img.dimensions();
  for i in 0..block_size {
    for j in 0..block_size {
      if x + j < width && y + i < height {
        let val = block[i as usize][j as usize];
        let gray = (val.clamp(0.0, 1.0) * 255.0) as u8;
        img.put_pixel(x + j, y + i, Rgba([gray, gray, gray, 255]));
      }
    }
  }
}

fn discrete_cosine(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, block_size: u32) {
  let (width, height) = img.dimensions();
  let mut dct_img = img.clone();
  for y in (0..height).step_by(block_size as usize) {
    for x in (0..width).step_by(block_size as usize) {
      let block = extract_block(img, x, y, block_size);
      let dct_block = dct_step(&block);
      store_block(&mut dct_img, dct_block, x, y, block_size);
    }
  }
  *img = dct_img;
}

fn dst_step(block: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
  let n = block.len();
  let mut dst = vec![vec![0.0; n]; n];
  for u in 0..n {
    for v in 0..n {
      let mut sum = 0.0;
      for i in 0..n {
        for j in 0..n {
          sum += block[i][j]
            * ((i as f64 + 0.5) * u as f64 * PI / n as f64).sin()
            * ((j as f64 + 0.5) * v as f64 * PI / n as f64).sin();
        }
      }
      dst[u][v] = sum * (2.0 / (n as f64).sqrt());
    }
  }
  dst
}

fn discrete_sine(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, block_size: u32) {
  let (width, height) = img.dimensions();
  let mut dst_img = img.clone();
  for y in (0..height).step_by(block_size as usize) {
    for x in (0..width).step_by(block_size as usize) {
      let block = extract_block(img, x, y, block_size);
      let dst_block = dst_step(&block);
      store_block(&mut dst_img, dst_block, x, y, block_size);
    }
  }
  *img = dst_img;
}

fn nearest_pixel(src_pixel: Rgba<u8>) -> Rgba<u8> {
  Rgba([src_pixel[0], src_pixel[1], src_pixel[2], 255])
}

fn pixel_delta(p1: Rgba<u8>, p2: Rgba<u8>) -> [i32; 3] {
  [(p1[0] as i32) - (p2[0] as i32), (p1[1] as i32) - (p2[1] as i32), (p1[2] as i32) - (p2[2] as i32)]
}

fn distribute_err(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32, q_err: [i32; 3], factor: f64) {
  if x < img.width() && y < img.height() {
    let mut pixel = img.get_pixel(x, y).0;
    pixel[0] = (pixel[0] as f64 + (q_err[0] as f64 * factor)).clamp(0.0, 255.0) as u8;
    pixel[1] = (pixel[1] as f64 + (q_err[1] as f64 * factor)).clamp(0.0, 255.0) as u8;
    pixel[2] = (pixel[2] as f64 + (q_err[2] as f64 * factor)).clamp(0.0, 255.0) as u8;
    img.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], 255]));
  }
}

fn fs_dither(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  let (width, height) = img.dimensions();
  let mut dither_img = img.clone();
  for y in 0..height {
    for x in 0..width {
      let old_pixel = *dither_img.get_pixel(x, y);
      let new_pixel = nearest_pixel(old_pixel);
      dither_img.put_pixel(x, y, new_pixel);
      let q_err = pixel_delta(old_pixel, new_pixel);
      if x + 1 < width {
        distribute_err(&mut dither_img, x + 1, y, q_err, FDF0);
      }
      if x > 0 && y + 1 < height {
        distribute_err(&mut dither_img, x - 1, y + 1, q_err, FDF1);
      }
      if y + 1 < height {
        distribute_err(&mut dither_img, x, y + 1, q_err, FDF2);
      }
      if x + 1 < width && y + 1 < height {
        distribute_err(&mut dither_img, x + 1, y + 1, q_err, FDF3);
      }
    }
  }
  *img = dither_img;
}

fn apply_effects(vid_in_name: &str, frames_dir: &str, vid_out_name: &str, img_type: &str, effects: &[Effect]) -> Result<(), Box<dyn std::error::Error>> {
  if !Path::new(vid_in_name).exists() {
    return Err(format!("apply_effects(): The video file {} does not exist.", vid_in_name).into());
  }
  clear_directory(frames_dir)?;
  let out_parts: Vec<&str> = vid_out_name.split("/").collect();
  let frame_outpart = out_parts.last().unwrap_or(&"").replace(".mp4", "");
  let ffmpeg_imgtype = match img_type {
    "bmp" | ".bmp" => BMP,
    "png" | ".png" => PNG,
    _ => PNG,
  };
  let _teardowncmd = if cfg!(target_os = "windows") {
    Command::new("cmd")
    .args(["/C", &format!("ffmpeg -i {} -vf fps=30 {}/{}_%04d{}", vid_in_name, frames_dir, frame_outpart, ffmpeg_imgtype)])
    .output()
    .expect("apply_effects(): Failed to execute _teardowncmd")
  } else {
    Command::new("sh")
    .arg("-c")
    .arg("echo hello")
    .output()
    .expect("apply_effects(): Failed to execute _teardowncmd (not implemented for linux/macOS)")
  };
  println!("apply_effects(): Successfully executed _teardowncmd to create {} source frames in {}", vid_in_name, frames_dir);
  let re = Regex::new(r"(?P<name>.+)_(?P<number>\d+)\.\w+").unwrap();
  for entry in fs::read_dir(frames_dir)? {
    let entry = entry?;
    let src_path = entry.path();
    if let Some(extension) = src_path.extension().and_then(|s| s.to_str()) {
      if extension == ffmpeg_imgtype.strip_prefix('.').unwrap() {
        let mut img = image::open(&src_path)?.to_rgba8();
        for fx in effects {
          fx.apply(&mut img);
        }
        let src_framename = src_path.file_name().unwrap().to_string_lossy();
        if let Some(captures) = re.captures(&src_framename) {
          let frame_num = captures["number"].parse::<u32>().unwrap_or(0);
          let new_framename = format!("{}/{}_fx_{:04}{}", frames_dir, frame_outpart, frame_num, ffmpeg_imgtype);
          img.save(&new_framename)?;
        } else {
          println!("Warning: Failed to extract frame number from filename: {}", src_framename);
        }
      }
    }
  }
  let _rebuildcmd = if cfg!(target_os = "windows") {
    Command::new("cmd")
    .args(["/C", &format!("ffmpeg -y -framerate 30 -i {}/{}_fx_%04d{} -c:v libx264 -pix_fmt yuv420p {}", frames_dir, frame_outpart, ffmpeg_imgtype, vid_out_name)])
    .output()
    .expect("apply_effects(): Failed to execute _rebuildcmd")
  } else {
    Command::new("sh")
    .arg("-c")
    .arg("echo hello")
    .output()
    .expect("apply_effects(): Failed to execute _rebuildcmd (not implemented for linux/macOS)")
  };
  println!("apply_effects(): Successfully executed _rebuildcmd to generate {} from {} source frames in {} with effects list applied", vid_out_name, vid_in_name, frames_dir);
  for entry in fs::read_dir(frames_dir)? {
    let entry = entry?;
    let img_path = entry.path();
    let img_name = img_path.file_name().unwrap().to_string_lossy();
    if !img_name.contains("_fx_") {
      fs::remove_file(img_path)?;
    }
  }
  Ok(())
}

fn main() -> io::Result<()> {
  let vid_in_name = VIDIN.to_owned()+"ants.mp4";
  let frames_dir = IMGOUT.to_owned()+"ants5";
  let vid_out_name = VIDOUT.to_owned()+"ants5.mp4";
  let image_type = "png";
  let fx: Vec<Effect> = vec![Effect::FsDither];
  let _ = apply_effects(&vid_in_name, &frames_dir, &vid_out_name, &image_type, &fx);
  Ok(())
}
