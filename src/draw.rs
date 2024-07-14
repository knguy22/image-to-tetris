use crate::board::Board;

use imageproc::{image, image::GenericImageView, image::DynamicImage, image::imageops::resize};

#[derive(Clone)]
pub struct Config {
    pub skin: BlockSkin,
    pub board_width: usize,
    pub board_height: usize,
}

#[derive(Clone)]
pub struct BlockSkin {
    pub empty_img: image::DynamicImage,
    pub i_img: image::DynamicImage,
    pub o_img: image::DynamicImage,
    pub t_img: image::DynamicImage,
    pub l_img: image::DynamicImage,
    pub j_img: image::DynamicImage,
    pub s_img: image::DynamicImage,
    pub z_img: image::DynamicImage,

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

    pub fn resize(&self, width: u32, height: u32) -> BlockSkin {
        BlockSkin {
            empty_img: DynamicImage::from(resize(&self.empty_img, width, height, image::imageops::FilterType::Lanczos3)),
            i_img: DynamicImage::from(resize(&self.i_img, width, height, image::imageops::FilterType::Lanczos3)),
            o_img: DynamicImage::from(resize(&self.o_img, width, height, image::imageops::FilterType::Lanczos3)),
            t_img: DynamicImage::from(resize(&self.t_img, width, height, image::imageops::FilterType::Lanczos3)),
            l_img: DynamicImage::from(resize(&self.l_img, width, height, image::imageops::FilterType::Lanczos3)),
            j_img: DynamicImage::from(resize(&self.j_img, width, height, image::imageops::FilterType::Lanczos3)),
            s_img: DynamicImage::from(resize(&self.s_img, width, height, image::imageops::FilterType::Lanczos3)),
            z_img: DynamicImage::from(resize(&self.z_img, width, height, image::imageops::FilterType::Lanczos3)),
            width,
            height
        }
    }

    pub fn as_array_ref(&self) -> [&DynamicImage; 8] {
        [&self.empty_img, &self.i_img, &self.o_img, &self.t_img, &self.l_img, &self.j_img, &self.s_img, &self.z_img]
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
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
    DynamicImage::from(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let skin = BlockSkin::new("assets/HqGYC5G - Imgur.png").unwrap();
        assert_eq!(skin.width, 36);
        assert_eq!(skin.height, 36);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_resize_larger() {
        let skin = BlockSkin::new("assets/HqGYC5G - Imgur.png").unwrap();
        let resized = skin.resize(64, 64);
        assert_eq!(resized.width, 64);
        assert_eq!(resized.height, 64);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_resize_smaller() {
        let skin = BlockSkin::new("assets/HqGYC5G - Imgur.png").unwrap();
        let resized = skin.resize(16, 16);
        assert_eq!(resized.width, 16);
        assert_eq!(resized.height, 16);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

}