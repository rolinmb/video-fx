extern crate image;
use image::{GenericImage, ImageBuffer, Rgba};
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
  GenInterp(
    f64,
    f64,
    f64,
    f64,
    Box<dyn Fn(f64, f64) -> f64 + Send + Sync>,
    Box<dyn Fn(f64, f64) -> f64 + Send + Sync>,
    Box<dyn Fn(f64, f64) -> f64 + Send + Sync>,
    Box<dyn Fn(f64, f64) -> f64 + Send + Sync>,
    Box<dyn Fn(f64, f64) -> f64 + Send + Sync>,
    Box<dyn Fn(f64, f64) -> f64 + Send + Sync>,
  ),
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
      Effect::GenInterp(iratio, rscale, gscale, bscale, ref f_r, ref f_g, ref f_b, ref fr_theta, ref fg_theta, ref fb_theta) => {
        gen_interp(img, iratio, rscale, gscale, bscale, f_r, f_g, f_b, fr_theta, fg_theta, fb_theta);
      }
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

fn fs_dither(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
  let (width, height) = img.dimensions();
  let mut img_buffer = img.clone();
  for y in 0..height {
    for x in 0..width {
      let old_pixel = img_buffer.get_pixel(x, y).0;
      let gray = (GSF0 * old_pixel[0] as f32 + GSF1 * old_pixel[1] as f32 + GSF2 * old_pixel[2] as f32) as u8;
      let new_pixel = if gray > 128 { 255 } else { 0 };
      let error = gray as f64 - new_pixel as f64;
      img_buffer.put_pixel(x, y, Rgba([new_pixel, new_pixel, new_pixel, old_pixel[3]]));
      if x + 1 < width {
        let pixel = img_buffer.get_pixel_mut(x + 1, y).0;
        let corrected = (pixel[0] as f64 + error * FDF0).clamp(0.0, 255.0) as u8;
        img_buffer.put_pixel(x + 1, y, Rgba([corrected, corrected, corrected, pixel[3]]));
      }
      if y + 1 < height {
        if x > 0 {
          let pixel = img_buffer.get_pixel_mut(x - 1, y + 1).0;
          let corrected = (pixel[0] as f64 + error * FDF1).clamp(0.0, 255.0) as u8;
          img_buffer.put_pixel(x - 1, y + 1, Rgba([corrected, corrected, corrected, pixel[3]]));
        }
        let pixel = img_buffer.get_pixel_mut(x, y + 1).0;
        let corrected = (pixel[0] as f64 + error * FDF2).clamp(0.0, 255.0) as u8;
        img_buffer.put_pixel(x, y + 1, Rgba([corrected, corrected, corrected, pixel[3]]));
        if x + 1 < width {
          let pixel = img_buffer.get_pixel_mut(x + 1, y + 1).0;
          let corrected = (pixel[0] as f64 + error * FDF3).clamp(0.0, 255.0) as u8;
          img_buffer.put_pixel(x + 1, y + 1, Rgba([corrected, corrected, corrected, pixel[3]]));
        }
      }
    }
  }
  *img = img_buffer;
}

fn gen_interp(
  img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
  iratio: f64, rscale: f64, gscale: f64, bscale: f64,
  f_r: impl Fn(f64, f64) -> f64 + Clone,
  f_g: impl Fn(f64, f64) -> f64 + Clone,
  f_b: impl Fn(f64, f64) -> f64 + Clone,
  fr_theta: impl Fn(f64, f64) -> f64 + Clone,
  fg_theta: impl Fn(f64, f64) -> f64 + Clone,
  fb_theta: impl Fn(f64, f64) -> f64 + Clone,
) {
  for (x, y, pixel) in img.enumerate_pixels_mut() {
    let gen_r = (fr_theta(x as f64, y as f64) * rscale * f_r.clone()(x as f64, y as f64)).max(0.0).min(255.0).round() as u8;
    let gen_g = (fg_theta(x as f64, y as f64) * gscale * f_g.clone()(x as f64, y as f64)).max(0.0).min(255.0).round() as u8;
    let gen_b = (fb_theta(x as f64, y as f64) * bscale * f_b.clone()(x as f64, y as f64)).max(0.0).min(255.0).round() as u8;
    pixel.0 = [
      (pixel.0[0] as f64 * iratio + (1.0 - iratio) * gen_r as f64).round() as u8,
      (pixel.0[1] as f64 * iratio + (1.0 - iratio) * gen_g as f64).round() as u8,
      (pixel.0[2] as f64 * iratio + (1.0 - iratio) * gen_b as f64).round() as u8,
      255,
    ];
  }
}

fn apply_effects(vid_in_name: &str, frames_dir: &str, vid_out_name: &str, img_type: &str, effects: &[Effect], iratio_init: f64, iratio_adj: f64) -> Result<(), Box<dyn std::error::Error>> {
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
  let frame_count = fs::read_dir(frames_dir)?.count() as f64;
  for entry in fs::read_dir(frames_dir)? {
    let entry = entry?;
    let src_path = entry.path();
    if let Some(extension) = src_path.extension().and_then(|s| s.to_str()) {
      if extension == ffmpeg_imgtype.strip_prefix('.').unwrap() {
        let mut img = image::open(&src_path)?.to_rgba8();
        let src_framename = src_path.file_name().unwrap().to_string_lossy();
        if let Some(captures) =re.captures(&src_framename) {
          let frame_num = captures["number"].parse::<u32>().unwrap_or(0);
          let mut adjusted_iratio = iratio_init + (iratio_adj * ((frame_num as f64) / frame_count));
          if adjusted_iratio > 0.999 {
            adjusted_iratio = 0.999;
          }
          if adjusted_iratio < 0.0 {
            adjusted_iratio = 0.0;
          }
          for fx in effects {
            if let Effect::GenInterp(_iratio, rscale, gscale, bscale, f_r, f_g, f_b, fr_theta, fg_theta, fb_theta) = fx {
              gen_interp(&mut img, adjusted_iratio, *rscale, *gscale, *bscale, f_r, f_g, f_b, fr_theta, fg_theta, fb_theta);
            } else {
              fx.apply(&mut img);
            }
          }
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
  let vid_in_name = VIDIN.to_owned()+"morning_07122024.mp4";
  let frames_dir = IMGOUT.to_owned()+"morning07122024";
  let vid_out_name = VIDOUT.to_owned()+"morning07122024_1.mp4";
  let image_type = "png";
  let interp_ratio = 0.5;
  let interp_ratio_adj = 0.45;
  let r_scale = 1.0;
  let g_scale = 1.0;
  let b_scale = 1.0;
  let fx: Vec<Effect> = vec![
    Effect::GenInterp(
      interp_ratio, // iratio
      r_scale, // rscale
      g_scale, // gscale
      b_scale, // bscale
      Box::new(|x, y| x * y), // f_r
      Box::new(|x, y| x + y), // f_g
      Box::new(|x, y| x - y), // f_b
      Box::new(|x, _y| x.sin()), // fr_theta 
      Box::new(|_x, y| y.cos()), // fg_theta
      Box::new(|x, y| (x * y).tan()), // fb_theta
    ),
  ];
  let _ = apply_effects(&vid_in_name, &frames_dir, &vid_out_name, &image_type, &fx, interp_ratio, interp_ratio_adj);
  Ok(())
}
