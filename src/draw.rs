use std::path;
use imageproc::{image, image::GenericImageView};

pub fn split_blocks(img_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let img = imageproc::image::open(img_path)?;

    let (width, height) = img.dimensions();
    let num_sections = 9;
    let section_width = width / num_sections;
    let img_buffer = img.into_rgb8();

    for i in 0..num_sections {
        let save_path_str = format!("assets/section_{}.png", i);
        let save_path = path::Path::new(&save_path_str);
        
        let section = image::ImageBuffer::from_fn(section_width, height, |x, y| {
            let pixel = img_buffer.get_pixel(section_width * i + x, y);
            *pixel
        });

        section.save(save_path)?;
    }

    Ok(())
}