use crate::board::{Board, EMPTY_CELL, BLOCKED_CELL};
use crate::piece::{Cell, Piece};

use imageproc::{image, image::GenericImageView, image::DynamicImage, image::imageops::resize};

#[derive(Clone)]
pub struct Config {
    pub board_width: usize,
    pub board_height: usize,
}

pub struct SkinnedBoard {
    pub board: Board,
    cells_skin: Vec<usize>,
    skins: Vec<BlockSkin>,
}

#[derive(Clone)]
pub struct BlockSkin {
    pub black_garbage: image::DynamicImage,
    pub gray_garbage: image::DynamicImage,

    pub i_img: image::DynamicImage,
    pub o_img: image::DynamicImage,
    pub t_img: image::DynamicImage,
    pub l_img: image::DynamicImage,
    pub j_img: image::DynamicImage,
    pub s_img: image::DynamicImage,
    pub z_img: image::DynamicImage,

    width: u32,
    height: u32,
    id: usize,
}

impl SkinnedBoard {
    pub fn new(width: usize, height: usize) -> SkinnedBoard {
        let mut skins = Vec::new();
        for file in std::fs::read_dir("assets").unwrap() {
            let path = file.unwrap().path();
            if path.is_file() && path.extension().unwrap() == "png" {
                skins.push(BlockSkin::new(path.to_str().unwrap(), skins.len()).unwrap());
            }
        }

        // cells skin must have the same dimensions as board
        SkinnedBoard {
            board: Board::new(width, height),
            cells_skin: vec![0; width * height],
            skins: skins
        }
    }

    pub fn iter_skins(&self) -> std::slice::Iter<BlockSkin> {
        self.skins.iter()
    }

    pub fn skins_width(&self) -> u32 {
        self.skins[0].width
    }

    pub fn skins_height(&self) -> u32 {
        self.skins[0].height
    }

    pub fn board_width(&self) -> usize {
        self.board.width
    }

    pub fn board_height(&self) -> usize {
        self.board.height
    }

    pub fn resize_skins(&mut self, width: u32, height: u32) {
        for skin in self.skins.iter_mut() {
            skin.resize(width, height);
        }
    }

    pub fn empty_at(&self, cell: &Cell) -> bool {
        *self.board.get(cell).unwrap_or(&BLOCKED_CELL) == EMPTY_CELL
    }

    pub fn place(&mut self, piece: &Piece, skin_id: usize) -> Result<(), Box<dyn std::error::Error>>{
        let board_width = self.board_width();

        // place the piece for both the skin and the boardj
        self.board.place(piece)?;
        for cell in piece.get_occupancy()? {
            self.cells_skin[cell.y * board_width + cell.x] = skin_id;
        }

        Ok(())
    }

    // assigns a cell to a skin and blocks it
    pub fn place_cell(&mut self, cell: &Cell, skin_id: usize) -> Result<(), Box<dyn std::error::Error>> {
        let board_width = self.board_width();
        match *self.board.get(cell)? {
            ' ' => {
                self.cells_skin[cell.y * board_width + cell.x] = skin_id;
                *self.board.get_mut(cell)? = BLOCKED_CELL;
                Ok(())
            },
            _ => Err("Cell is occupied".into())
        }
    }
}


impl BlockSkin {
    pub fn new(skin_path: &str, id: usize) -> Result<BlockSkin, Box<dyn std::error::Error>> {
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
            black_garbage: new_images[0].clone(),
            gray_garbage: new_images[1].clone(),
            i_img: new_images[6].clone(),
            o_img: new_images[4].clone(),
            t_img: new_images[8].clone(),
            l_img: new_images[3].clone(),
            j_img: new_images[7].clone(),
            s_img: new_images[5].clone(),
            z_img: new_images[2].clone(),
            width: section_width,
            height: height,
            id: id,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.black_garbage = DynamicImage::from(resize(&self.black_garbage, width, height, image::imageops::FilterType::Lanczos3));
        self.gray_garbage = DynamicImage::from(resize(&self.gray_garbage, width, height, image::imageops::FilterType::Lanczos3));
        self.i_img = DynamicImage::from(resize(&self.i_img, width, height, image::imageops::FilterType::Lanczos3));
        self.o_img = DynamicImage::from(resize(&self.o_img, width, height, image::imageops::FilterType::Lanczos3));
        self.t_img = DynamicImage::from(resize(&self.t_img, width, height, image::imageops::FilterType::Lanczos3));
        self.l_img = DynamicImage::from(resize(&self.l_img, width, height, image::imageops::FilterType::Lanczos3));
        self.j_img = DynamicImage::from(resize(&self.j_img, width, height, image::imageops::FilterType::Lanczos3));
        self.s_img = DynamicImage::from(resize(&self.s_img, width, height, image::imageops::FilterType::Lanczos3));
        self.z_img = DynamicImage::from(resize(&self.z_img, width, height, image::imageops::FilterType::Lanczos3));
        self.width = width;
        self.height = height;
    }

    #[allow(dead_code)]
    pub fn as_array_ref(&self) -> [&DynamicImage; 9] {
        [&self.black_garbage, &self.gray_garbage, &self.i_img, &self.o_img, &self.t_img, &self.l_img, &self.j_img, &self.s_img, &self.z_img]
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn id(&self) -> usize {
        self.id
    }
}


pub fn draw_board(skin_board: &SkinnedBoard) -> DynamicImage {
    let board = &skin_board.board;
    let skins = &skin_board.skins;
    let cells_skin = &skin_board.cells_skin;

    let mut img = image::RgbaImage::new(board.width as u32 * skins[0].width, board.height as u32 * skins[0].height);
    for y in 0..board.height {
        for x in 0..board.width {
            let skin = &skins[cells_skin[y * board.width + x]];
            let block = match board.cells[y * board.width + x] {
                'I' => &skin.i_img,
                'O' => &skin.o_img,
                'T' => &skin.t_img,
                'L' => &skin.l_img,
                'J' => &skin.j_img,
                'S' => &skin.s_img,
                'Z' => &skin.z_img,
                'G' => &skin.gray_garbage,
                _ => &skin.black_garbage,
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
        let skin = BlockSkin::new("assets/HqGYC5G - Imgur.png", 0).unwrap();
        assert_eq!(skin.width, 36);
        assert_eq!(skin.height, 36);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_resize_larger() {
        let mut skin = BlockSkin::new("assets/HqGYC5G - Imgur.png", 0).unwrap();
        skin.resize(64, 64);
        assert_eq!(skin.width, 64);
        assert_eq!(skin.height, 64);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_resize_smaller() {
        let mut skin = BlockSkin::new("assets/HqGYC5G - Imgur.png", 0).unwrap();
        skin.resize(16, 16);
        assert_eq!(skin.width, 16);
        assert_eq!(skin.height, 16);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_skinned_board_resize() {
        let mut board = SkinnedBoard::new(36, 36);
        board.resize_skins(64, 64);

        for skin in board.skins.iter() {
            assert_eq!(skin.width, 64);
            assert_eq!(skin.height, 64);
        }
    }

}