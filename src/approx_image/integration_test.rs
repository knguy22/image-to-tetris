use super::{Config, GlobalData, draw::resize_skins, resize_image};

use std::fs;
use std::path::PathBuf;
use std::time;

use image::GenericImageView;
use imageproc::image::DynamicImage;
use dssim::Dssim;
use rayon::prelude::*;

// tests all image in the directory
pub fn run(dir: &str, config: &Config, glob: &GlobalData) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running integration test on {}", dir);

    let start = time::Instant::now();
    let num_files = fs::read_dir(dir)?.count();
    let images: Vec<_> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .collect();

    println!("Approximating {} images", num_files);

    let total_diff: f64 = images
        .par_iter()
        .map(|image| {
            score_image(image.path(), config, glob).expect("failed to score image")
        })
        .sum();

    if num_files == 0 {
        panic!("No images found in directory");
    }

    println!("Number of images={}", num_files);
    println!("Total Dssim diff={}", total_diff);
    println!("Average Dssim diff={}", total_diff / (num_files as f64));
    println!("Time Elapsed: {:?}", start.elapsed());
    Ok(())
}

fn score_image(path: PathBuf, old_config: &Config, glob: &GlobalData) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff = 0.0;
    let mut source_img = image::open(path.clone())?;
    
    // set the board height to scale to the image
    let board_height = source_img.width() * (old_config.board_width as u32) / source_img.height();
    let config = Config {
        board_width: old_config.board_width as usize,
        board_height: board_height as usize,
        ..*old_config
    };

    // create a new glob for the local approximation since each image can contain different sizes
    // this means the block skin sizes should be tailored to the image
    let mut glob = glob.clone();

    // resize the source image and skins as necessary
    let (image_width, image_height) = source_img.dimensions();
    resize_skins(&mut glob.skins, image_width, image_height, config.board_width, config.board_height)?;
    resize_image(&mut source_img, glob.skin_width(), glob.skin_height(), config.board_width, config.board_height);

    // handle scoring
    let approx_img = super::approx(&mut source_img, &config, &glob)?;
    let dssim_diff = diff_images_dssim(&approx_img, &source_img)?;
    total_diff += dssim_diff;
    println!("Diff: {}, Source: {}", dssim_diff, path.display());

    Ok(total_diff)
}

fn diff_images_dssim(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let d = Dssim::new();

    let image1_buffer = image1.to_rgb8();
    let image2_buffer = image2.to_rgb8();

    let image1_rgb = rgb::FromSlice::as_rgb(image1_buffer.as_raw().as_slice());
    let image2_rgb = rgb::FromSlice::as_rgb(image2_buffer.as_raw().as_slice());

    let d_image1 = d.create_image_rgb(image1_rgb, image1.width() as usize, image1.height() as usize).expect("Failed to create dssim image");
    let d_image2 = d.create_image_rgb(image2_rgb, image2.width() as usize, image2.height() as usize).expect("Failed to create dssim image");

    let (diff, _) = d.compare(&d_image1, &d_image2);
    Ok(diff.into())
}