use crate::board::Board;

use imageproc::{image, image::GenericImageView, image::DynamicImage};

pub struct BlockSkin {
    empty_img: image::DynamicImage,
    i_img: image::DynamicImage,
    o_img: image::DynamicImage,
    t_img: image::DynamicImage,
    l_img: image::DynamicImage,
    j_img: image::DynamicImage,
    s_img: image::DynamicImage,
    z_img: image::DynamicImage,

    width: u32,
    height: u32
}

impl BlockSkin {
    pub fn new(skin_path: &str) -> Result<BlockSkin, Box<dyn std::error::Error>> {
        let img = imageproc::image::open(skin_path)?;

        const NUM_SECTIONS: usize = 9;
        let (width, height) = img.dimensions();
        let section_width = width / NUM_SECTIONS as u32;
        let img_buffer = img.into_rgb8();

        // split the skin into sections
        let mut new_images: [DynamicImage; NUM_SECTIONS] = Default::default();
        for i in 0..NUM_SECTIONS as u32 {
            let section = image::SubImage::new(&img_buffer, i * section_width, 0, section_width, height);
            new_images[i as usize] = section.to_image().into();
        }
        
        // return the skin
        Ok(BlockSkin {
            empty_img: new_images[0].clone(),
            i_img: new_images[6].clone(),
            o_img: new_images[4].clone(),
            t_img: new_images[8].clone(),
            l_img: new_images[3].clone(),
            j_img: new_images[7].clone(),
            s_img: new_images[5].clone(),
            z_img: new_images[2].clone(),
            width: section_width,
            height
        })
    }
}


pub fn draw_board(board: &Board, skin: &BlockSkin) -> DynamicImage {
    let mut img = image::RgbaImage::new(board.width as u32 * skin.width, board.height as u32 * skin.height);

    for y in 0..board.height {
        for x in 0..board.width {
            let block = match board.cells[y * board.width + x] {
                'I' => &skin.i_img,
                'O' => &skin.o_img,
                'T' => &skin.t_img,
                'L' => &skin.l_img,
                'J' => &skin.j_img,
                'S' => &skin.s_img,
                'Z' => &skin.z_img,
                _ => &skin.empty_img,
            };
            image::imageops::overlay(&mut img, block, (x as u32 * skin.width).into(), (y as u32 * skin.height).into());
        }
    }
    // flip the image due to how the board is represented
    img = image::imageops::flip_vertical(&img);
    DynamicImage::from(img)
}
