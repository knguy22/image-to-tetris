use super::board::{Board, EMPTY_CELL, BLOCKED_CELL};
use super::piece::{Cell, Piece};

use image::Rgba;
use imageproc::{image, image::GenericImageView, image::DynamicImage, image::imageops::resize};

const INVALID_SKIN_ID: usize = usize::MAX;

pub type Skins = Vec<BlockSkin>;

pub struct SkinnedBoard<'a> {
    board: Board,
    cells_skin: Vec<usize>,
    skins: &'a Skins,
}

#[derive(Clone)]
pub struct BlockSkin {
    black_img: BlockImage,
    gray_img: BlockImage,

    i_img: BlockImage,
    o_img: BlockImage,
    t_img: BlockImage,
    l_img: BlockImage,
    j_img: BlockImage,
    s_img: BlockImage,
    z_img: BlockImage,

    width: u32,
    height: u32,
    id: usize,
}

#[derive(Clone)]
pub struct BlockImage {
    img: image::DynamicImage,
    avg_pixel: Rgba<u8>,
}

impl<'a> SkinnedBoard<'a> {
    pub fn new(width: usize, height: usize, skins: &'a Skins) -> SkinnedBoard {
        // cells skin must have the same dimensions as board
        SkinnedBoard {
            board: Board::new(width, height),
            cells_skin: vec![INVALID_SKIN_ID; width * height],
            skins
        }
    }

    pub fn iter_skins(&self) -> std::slice::Iter<BlockSkin> {
        self.skins.iter()
    }

    pub fn get_skin(&self, index: usize) -> &BlockSkin {
        &self.skins[index]
    }

    pub fn skins_width(&self) -> u32 {
        self.skins[0].width
    }

    pub fn skins_height(&self) -> u32 {
        self.skins[0].height
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn board_width(&self) -> usize {
        self.board.width
    }

    pub fn board_height(&self) -> usize {
        self.board.height
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

    pub fn get_cells_skin(&self, cell: &Cell) -> usize {
        self.cells_skin[cell.y * self.board_width() + cell.x]
    }
}

pub fn resize_skins(skins: &mut Skins, image_width: u32, image_height: u32, board_width: usize, board_height: usize) -> Result<(), Box<dyn std::error::Error>> {
    let skin_width = image_width / u32::try_from(board_width)?;
    let skin_height = image_height / u32::try_from(board_height)?;
    if skin_width == 0 || skin_height == 0 {
        return Err("Skin dimensions must be greater than 0".into());
    }
    for skin in skins.iter_mut() {
        skin.resize(skin_width, skin_height);
    }
    Ok(())
}

impl BlockSkin {
    pub fn new(skin_path: &str, id: usize) -> Result<BlockSkin, Box<dyn std::error::Error>> {
        let img = imageproc::image::open(skin_path)?;

        const NUM_SECTIONS: usize = 9;
        let (width, height) = img.dimensions();
        let section_width = width / NUM_SECTIONS as u32;
        let img_buffer = img.into_rgb8();

        // split the skin into sections
        let mut new_images = Vec::new();
        for i in 0..NUM_SECTIONS as u32 {
            let section = image::SubImage::new(&img_buffer, i * section_width, 0, section_width, height);
            new_images.push(BlockImage::new(section.to_image().into())?);
        }
        
        // return the skin
        Ok(BlockSkin {
            black_img: new_images[0].clone(),
            gray_img: new_images[1].clone(),
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
        for block in self.as_array_ref_mut() {
            block.resize(width, height);
        }
        self.width = width;
        self.height = height;
    }

    #[allow(dead_code)]
    pub fn as_array_ref(&self) -> [&BlockImage; 9] {
        [&self.black_img, &self.gray_img, &self.i_img, &self.o_img, &self.t_img, &self.l_img, &self.j_img, &self.s_img, &self.z_img]
    }

    pub fn as_array_ref_mut(&mut self) -> [&mut BlockImage; 9] {
        [&mut self.black_img, &mut self.gray_img, &mut self.i_img, &mut self.o_img, &mut self.t_img, &mut self.l_img, &mut self.j_img, &mut self.s_img, &mut self.z_img]
    }

    pub fn block_image_from_piece(&self, piece: &Piece) -> &BlockImage {
        match piece {
            Piece::I(_, _) => &self.i_img,
            Piece::O(_, _) => &self.o_img,
            Piece::T(_, _) => &self.t_img,
            Piece::L(_, _) => &self.l_img,
            Piece::J(_, _) => &self.j_img,
            Piece::S(_, _) => &self.s_img,
            Piece::Z(_, _) => &self.z_img,
            Piece::Gray(_) => &self.gray_img,
            Piece::Black(_) => &self.black_img,
        }
    }

    pub fn block_image_from_char(&self, cell_val: &char) -> &BlockImage {
        match *cell_val {
            'I' => &self.i_img,
            'O' => &self.o_img,
            'T' => &self.t_img,
            'L' => &self.l_img,
            'J' => &self.j_img,
            'S' => &self.s_img,
            'Z' => &self.z_img,
            'G' => &self.gray_img,
            _ => &self.black_img,
        }
        
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

impl BlockImage {
    pub fn new(img: DynamicImage) -> Result<BlockImage, Box<dyn std::error::Error>> {
        let num_pixels: u32 = (img.width() * img.height()) as u32;
        let avg_pixel: Rgba<u8> = img
            .pixels()
            // use u32 for summation instead of u8 to prevent overflow
            .fold([0, 0, 0, 0],
                |acc, (_x, _y, p) |
                [acc[0] + p[0] as u32, acc[1] + p[1] as u32, acc[2] + p[2] as u32, acc[3] + p[3] as u32])
            // divide by number of pixels
            .map(|x| u8::try_from(x / num_pixels).expect("could not convert pixel sum to u8"))
            .into();

        Ok(BlockImage {
            img,
            avg_pixel,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.img.width() != width || self.img.height() != height {
            self.img = DynamicImage::from(resize(&self.img, width, height, image::imageops::FilterType::Lanczos3));
        }
    }

    #[allow(dead_code)]
    pub fn width(&self) -> u32 {
        self.img.width()
    }

    #[allow(dead_code)]
    pub fn height(&self) -> u32 {
        self.img.height()
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Rgba<u8> {
        self.img.get_pixel(x, y)
    }

    pub fn get_average_pixel(&self) -> Rgba<u8> {
        self.avg_pixel
    }
}

pub fn draw_board(skin_board: &SkinnedBoard) -> DynamicImage {
    let board = &skin_board.board;
    let skins = skin_board.skins;
    let cells_skin = &skin_board.cells_skin;

    let mut img = image::RgbaImage::new(board.width as u32 * skins[0].width, board.height as u32 * skins[0].height);
    for y in 0..board.height {
        for x in 0..board.width {
            let skin_id = cells_skin[y * board.width + x];
            let skin = skin_board.get_skin(skin_id);
            let block = match board.cells[y * board.width + x] {
                'I' => &skin.i_img,
                'O' => &skin.o_img,
                'T' => &skin.t_img,
                'L' => &skin.l_img,
                'J' => &skin.j_img,
                'S' => &skin.s_img,
                'Z' => &skin.z_img,
                'G' => &skin.gray_img,
                'B' => &skin.black_img,
                _ => panic!("Invalid cell value: {}", board.cells[y * board.width + x]),
            };
            image::imageops::overlay(&mut img, &block.img, (x as u32 * skin.width).into(), (y as u32 * skin.height).into());
        }
    }
    DynamicImage::from(img)
}

pub fn create_skins() -> Skins {
    let mut skins = Vec::new();
    for file in std::fs::read_dir("assets").expect("assets directory not found") {
        let path = file.expect("failed to read file").path();
        if path.is_file() && path.extension().expect("no file extension found") == "png" {
            skins.push(BlockSkin::new(path.to_str().expect("failed to convert path to string"), skins.len()).expect("failed to load skin"));
        }
    }

    skins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        let skin = BlockSkin::new("test_images/HqGYC5G - Imgur.png", 0).expect("could not load skin");
        assert_eq!(skin.width, 36);
        assert_eq!(skin.height, 36);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_resize_larger() {
        let mut skin = BlockSkin::new("test_images/HqGYC5G - Imgur.png", 0).expect("could not load skin");
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
        let mut skin = BlockSkin::new("test_images/HqGYC5G - Imgur.png", 0).expect("could not load skin");
        skin.resize(16, 16);
        assert_eq!(skin.width, 16);
        assert_eq!(skin.height, 16);

        for i in skin.as_array_ref() {
            assert_eq!(i.width(), skin.width);
            assert_eq!(i.height(), skin.height);
        }
    }

    #[test]
    fn test_save_skinned_board() {
        let mut skin = BlockSkin::new("test_images/HqGYC5G - Imgur.png", 0).expect("could not load skin");
        skin.resize(16, 16);
        let skins = vec![skin];

        // board should have all cells be set to INVALID by default
        let board_width = 4;
        let board_height = 4;
        let mut board = SkinnedBoard::new(4, 4, &skins);
        for cell in board.cells_skin.iter() {
            assert_eq!(*cell, INVALID_SKIN_ID);
        }

        // replace the invalid
        for y in 0..board_height {
            for x in 0..board_width {
                board.place(&Piece::Black(Cell { x, y }), 0).expect("failed to place piece");
            }
        }

        let image = draw_board(&board);

        image.save("test_results/test_save_skinned_board.png").expect("failed to save image");
    }
}