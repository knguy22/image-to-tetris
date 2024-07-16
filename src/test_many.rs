use crate::approx::approximate;
use crate::draw;

use std::fs;
use std::time;
use imageproc::image::DynamicImage;
use dssim::Dssim;

// tests all image in the directory
pub fn run(dir: &str, board_width: u32) -> Result<(), Box<dyn std::error::Error>> {
    let start = time::Instant::now();
    let num_files = fs::read_dir(dir)?.count();
    let mut total_diff = 0.0;

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        
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
    }

    println!("Number of images={}", num_files);
    println!("Total Dssim diff={}", total_diff);
    println!("Average Dssim diff={}", total_diff / (num_files as f64));
    println!("Time Elapsed: {:?}", start.elapsed());
    Ok(())
}

fn diff_images_dssim(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let d = Dssim::new();

    let image1_buffer = image1.to_rgb8();
    let image2_buffer = image2.to_rgb8();

    let image1_rgb = rgb::FromSlice::as_rgb(image1_buffer.as_raw().as_slice());
    let image2_rgb = rgb::FromSlice::as_rgb(image2_buffer.as_raw().as_slice());

    let d_image1 = d.create_image_rgb(image1_rgb, image1.width() as usize, image1.height() as usize).unwrap();
    let d_image2 = d.create_image_rgb(image2_rgb, image2.width() as usize, image2.height() as usize).unwrap();

    let (diff, _) = d.compare(&d_image1, &d_image2);
    Ok(diff.into())
}