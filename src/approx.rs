use crate::board::EMPTY_CELL;
use crate::draw::{self, BlockSkin, Config, SkinnedBoard};
use crate::piece::{Cell, Piece, Orientation};

use std::collections::BinaryHeap;

use image::Rgba;
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
                let diff = avg_piece_pixel_diff(&piece, &board, skin, &target_img)?;
                if diff < best_piece_diff {
                    best_piece = Some(piece);
                    best_piece_diff = diff;
                    best_skin_id = Some(skin.id());
                }
            }

            // try placing pieces
            for orientation in Orientation::all() {
                for piece in Piece::all_normal(cell, orientation) {
                    if board.board().can_place(&piece) {
                        let diff = avg_piece_pixel_diff(&piece, &board, &skin, &target_img)?;
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

fn avg_piece_pixel_diff(piece: &Piece, board: &SkinnedBoard, skin: &BlockSkin, target_img: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff: f64 = 0.0;
    let mut total_pixels: u32 = 0;
    let block_image = skin.block_image_from_piece(piece);

    let center_cell = piece.get_cell();
    let occupancy = piece.get_occupancy()?;
    let mut context_cells: Vec<Cell> = Vec::new();

    // you want the context to be the opposite direction of the new cells, ie (dy, dx) > 0
    for dy in 0..2 {
        for dx in 0..2 {
            let context_cell = Cell { x: center_cell.x + dx + 1, y: center_cell.y + dy + 1 };
            let context_char = board.board().get(&context_cell);

            // only append contexts that are occupied with other pieces we already placed
            if context_char.is_ok() && *context_char.unwrap() != EMPTY_CELL && !occupancy.contains(&context_cell) {
                context_cells.push(context_cell);
            }
        }
    }

    for cell in occupancy {
        // first analyze the context
        for context_cell in &context_cells {
            let skin_id = board.get_cells_skin(&context_cell);
            let context_skin = board.get_skin(skin_id);
            let context_block_image = context_skin.block_image_from_char(board.board().get(&context_cell)?);

            for y in 0..skin.height() {
                for x in 0..skin.width() {
                    let target_context_pixel = target_img.get_pixel((context_cell.x as u32 * skin.width() + x) as u32, (context_cell.y as u32 * skin.height() + y) as u32);
                    let target_pixel = target_img.get_pixel((cell.x as u32 * skin.width() + x) as u32, (cell.y as u32 * skin.height() + y) as u32);
                    let approx_context_pixel = context_block_image.get_pixel(x, y);
                    let approx_pixel = block_image.get_pixel(x, y);

                    let target_delta: f64 = subtract_pixels(&target_pixel, &target_context_pixel)
                        .iter()
                        .fold(0.0, |acc, x| acc + (x.abs() as f64));
                    let approx_delta: f64 = subtract_pixels(&approx_pixel, &approx_context_pixel)
                        .iter()
                        .fold(0.0, |acc, x| acc + (x.abs() as f64));

                    total_diff += target_delta - approx_delta;
                    total_pixels += 3;
                }
            }
        }

        // then analyze the difference between the current cells
        for y in 0..skin.height() {
            for x in 0..skin.width() {
                let target_pixel = target_img.get_pixel((cell.x as u32 * skin.width() + x) as u32, (cell.y as u32 * skin.height() + y) as u32);
                let approx_pixel = block_image.get_pixel(x, y);
                total_diff += subtract_pixels(&target_pixel, &approx_pixel)
                    .iter()
                    .fold(0.0, |acc, x| acc + x.pow(2) as f64);
                total_pixels += 3;
            }
        }
    }

    Ok(total_diff / total_pixels as f64)
}

fn subtract_pixels(a: &Rgba<u8>, b: &Rgba<u8>) -> [i32; 3] {
    [
        a[0] as i32 - b[0] as i32,
        a[1] as i32 - b[1] as i32,
        a[2] as i32 - b[2] as i32,
    ]
}