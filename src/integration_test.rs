use crate::{approx_image::approximate, draw};

use std::fs;
use std::path::PathBuf;
use std::time;

use imageproc::image::DynamicImage;
use dssim::Dssim;
use rayon::{prelude::*, current_num_threads};

// tests all image in the directory
pub fn run(dir: &str, board_width: u32) -> Result<(), Box<dyn std::error::Error>> {
    let start = time::Instant::now();
    let num_files = fs::read_dir(dir)?.count();
    let images: Vec<_> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .collect();

    println!("Running {} images in {} threads", num_files, current_num_threads());

    let total_diff: f64 = images
        .par_iter()
        .map(|image| {
            let path = image.path();
            run_thread(path, board_width).unwrap()
        })
        .sum();

    println!("Number of images={}", num_files);
    println!("Total Dssim diff={}", total_diff);
    println!("Average Dssim diff={}", total_diff / (num_files as f64));
    println!("Time Elapsed: {:?}", start.elapsed());
    Ok(())
}

fn run_thread(path: PathBuf, board_width: u32) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff = 0.0;
    let mut target_img = image::open(path.clone())?;
    
    // set the board height to scale to the image
    let board_height = target_img.width() * board_width / target_img.height();
    let config = draw::Config {
        board_width: board_width as usize,
        board_height: board_height as usize,
    };

    let approx_img = approximate(&mut target_img, &config)?;

    // handle scoring
    let dssim_diff = diff_images_dssim(&approx_img, &target_img)?;
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