use crate::draw::{self, BlockSkin, SkinnedBoard, Config};
use crate::piece::{Cell, Piece, Orientation};

use std::collections::BinaryHeap;

use imageproc::image::{DynamicImage, GenericImageView};

// the target image will be changed in order to fit the scaling of the board
pub fn approximate(target_img: &mut DynamicImage, config: &Config) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    // initialize the board
    let mut board = SkinnedBoard::new(config.board_width, config.board_height);

    // resize the skins
    let (img_width, img_height) = target_img.dimensions();
    let skin_width = img_width / u32::try_from(board.board_width())?;
    let skin_height = img_height / u32::try_from(board.board_height())?;
    if skin_width == 0 || skin_height == 0 {
        return Err("Skin dimensions must be greater than 0".into());
    }
    board.resize_skins(skin_height, skin_width);

    // resize the target image to account for rounding errors
    *target_img = resize_img_from_board(&board, target_img)?;

    // init the heap and push the first row of cells into it
    // the first row is the highest row in number because we are using a max heap
    let mut heap = BinaryHeap::new();
    for y in (0..board.board_height()).rev() {
        for x in 0..board.board_width() {
            heap.push(Cell { x: x, y: y });
        }
    }

    // for each cell at the top of the heap:
    while heap.len() > 0 {
        let cell = heap.pop().unwrap();

        // 1. check if the cell is unoccupied
        if !board.empty_at(&cell) {
            continue;
        }

        // 2. for each possible skin, piece, and orientation:
        let mut best_piece: Option<Piece> = None;
        let mut best_piece_diff = f64::MAX;
        let mut best_skin_id: Option<usize> = None;

        for skin in board.iter_skins() {
            // try black or gray garbage
            for piece in Piece::all_garbage(cell) {
                let diff = avg_grid_pixel_diff(&piece, skin, &target_img)?;
                if diff < best_piece_diff {
                    best_piece = Some(piece);
                    best_piece_diff = diff;
                    best_skin_id = Some(skin.id());
                }
            }

            // try placing pieces
            for orientation in Orientation::all() {
                for piece in Piece::all_normal(cell, orientation) {
                    if board.board.can_place(&piece) {
                        let diff = avg_piece_pixel_diff(&piece, &skin, &target_img)?;
                        if diff < best_piece_diff {
                            best_piece = Some(piece);
                            best_piece_diff = diff;
                            best_skin_id = Some(skin.id());
                        }
                    }
                }
            }
        }

        // place the best piece; there must be a best piece
        let best_piece = best_piece.unwrap();
        board.place(&best_piece, best_skin_id.unwrap())?;
    }

    // draw the board
    Ok(draw::draw_board(&board))
}

fn resize_img_from_board(board: &SkinnedBoard, target_img: &DynamicImage) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    // resize the target image to account for rounding errors
    let resized_target_width = board.skins_width() * u32::try_from(board.board_width())?;
    let resized_target_height = board.skins_height() * u32::try_from(board.board_height())?;
    let resized_target_buffer = image::imageops::resize(target_img, resized_target_width, resized_target_height, image::imageops::FilterType::Lanczos3);
    Ok(image::DynamicImage::from(resized_target_buffer))
}

fn avg_piece_pixel_diff(piece: &Piece, skin: &BlockSkin, target_img: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff: f64 = 0.0;
    let mut total_pixels: u32 = 0;
    let block_skin: &DynamicImage = match piece {
        Piece::I(_, _) => &skin.i_img,
        Piece::O(_, _) => &skin.o_img,
        Piece::T(_, _) => &skin.t_img,
        Piece::L(_, _) => &skin.l_img,
        Piece::J(_, _) => &skin.j_img,
        Piece::S(_, _) => &skin.s_img,
        Piece::Z(_, _) => &skin.z_img,
        _ => panic!("Garbage or black piece has no skin")
    };

    for cell in piece.get_occupancy()? {
        for y in 0..skin.height() {
            for x in 0..skin.width() {
                let target_pixel = target_img.get_pixel((cell.x as u32 * skin.width() + x) as u32, (cell.y as u32 * skin.height() + y) as u32);
                let skin_pixel = block_skin.get_pixel(x, y);
                total_diff += (target_pixel[0] as i32 - skin_pixel[0] as i32).pow(2) as f64;
                total_diff += (target_pixel[1] as i32 - skin_pixel[1] as i32).pow(2) as f64;
                total_diff += (target_pixel[2] as i32 - skin_pixel[2] as i32).pow(2) as f64;
                total_pixels += 3;
            }
        }
    }

    Ok(total_diff / total_pixels as f64)
}

fn avg_grid_pixel_diff(piece: &Piece, skin: &BlockSkin, target_img: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff: f64 = 0.0;
    let mut total_pixels: u32 = 0;

    let block_skin: &DynamicImage = match piece {
        Piece::Garbage(_) => &skin.gray_img,
        Piece::Black(_) => &skin.black_img,
        _ => panic!("Piece is not garbage or black"),
    };

    // searches a grid around this cell
    let cell = piece.get_cell();
    for y in 0..skin.height() {
        for x in 0..skin.width() {
            let target_pixel = target_img.get_pixel((cell.x as u32 * skin.width() + x) as u32, (cell.y as u32 * skin.height() + y) as u32);
            let skin_pixel = block_skin.get_pixel(x, y);
            total_diff += (target_pixel[0] as i32 - skin_pixel[0] as i32).pow(2) as f64;
            total_diff += (target_pixel[1] as i32 - skin_pixel[1] as i32).pow(2) as f64;
            total_diff += (target_pixel[2] as i32 - skin_pixel[2] as i32).pow(2) as f64;
            total_pixels += 3;
        }
    }

    Ok(total_diff / total_pixels as f64)
}