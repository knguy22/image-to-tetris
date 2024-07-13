use crate::board::Board;

use imageproc::{image, image::GenericImageView};
use std::path;

pub fn draw_board(board: &Board) {
    let i_img = image::open("assets/section_2.png").unwrap();
    let o_img = image::open("assets/section_3.png").unwrap();
    let t_img = image::open("assets/section_4.png").unwrap();
    let l_img = image::open("assets/section_5.png").unwrap();
    let j_img = image::open("assets/section_6.png").unwrap();
    let s_img = image::open("assets/section_7.png").unwrap();
    let z_img = image::open("assets/section_8.png").unwrap();

    let block_img_width = i_img.width();
    let block_img_height = i_img.height();

    let mut img = image::RgbaImage::new(board.width as u32 * block_img_width, board.height as u32 * block_img_height);

    for y in 0..board.height {
        for x in 0..board.width {
            let block = match board.cells[y * board.width + x] {
                'I' => &i_img,
                'O' => &o_img,
                'T' => &t_img,
                'L' => &l_img,
                'J' => &j_img,
                'S' => &s_img,
                'Z' => &z_img,
                _ => { continue; }
            };
            image::imageops::overlay(&mut img, block, (x as u32 * block_img_width).into(), (y as u32 * block_img_height).into());
        }
    }
    // flip the image
    img = image::imageops::flip_vertical(&img);
    img.save("results/board.png").unwrap();
}



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