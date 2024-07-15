use crate::board::Board;
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
    board.resize_skins(img_width / u32::try_from(board.board_width())?, img_height / u32::try_from(board.board_height())?);

    // resize the target image to account for rounding errors
    *target_img = resize_img_from_board(&board, target_img)?;

    // init the heap and push the first row of cells into it
    // the first row is the highest row in number because we are using a max heap
    let mut heap = BinaryHeap::new();
    for x in 0..board.board_width() {
        heap.push(Cell { x: x, y: board.board_height() - 1 });
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
            for id in 0..2 {
                let diff = avg_grid_pixel_diff(&cell, &board.board, skin, id, &target_img)?;
                if diff < best_piece_diff {
                    best_piece = None;
                    best_piece_diff = diff;
                    best_skin_id = Some(skin.id());
                }
            }
            
            // try placing pieces
            for orientation in Orientation::all() {
                for piece in Piece::all(cell, orientation.clone()) {
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

        // 3. if we found a piece, place it
        if best_piece.is_some() {
            let best_piece = best_piece.unwrap();
            board.place(&best_piece, best_skin_id.unwrap())?;

            // check cells above to push into heap
            for piece_cell in best_piece.get_occupancy()? {
                if piece_cell.y > 0 {
                    heap.push(Cell { x: piece_cell.x, y: piece_cell.y - 1 });
                }
            }
        } 
        // assign the empty/garbage block the skin
        else {
            board.place_cell(&cell, best_skin_id.unwrap())?;
            if cell.y > 0 {
                heap.push(Cell { x: cell.x, y: cell.y - 1 });
            }
        }
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
    for cell in piece.get_occupancy()? {
        for y in 0..skin.height() {
            for x in 0..skin.width() {
                let target_pixel = target_img.get_pixel((cell.x as u32 * skin.width() + x) as u32, (cell.y as u32 * skin.height() + y) as u32);
                let skin_pixel = match piece {
                    Piece::I(_, _) => skin.i_img.get_pixel(x, y),
                    Piece::O(_, _) => skin.o_img.get_pixel(x, y),
                    Piece::T(_, _) => skin.t_img.get_pixel(x, y),
                    Piece::L(_, _) => skin.l_img.get_pixel(x, y),
                    Piece::J(_, _) => skin.j_img.get_pixel(x, y),
                    Piece::S(_, _) => skin.s_img.get_pixel(x, y),
                    Piece::Z(_, _) => skin.z_img.get_pixel(x, y),
                };

                total_diff += (target_pixel[0] as i32 - skin_pixel[0] as i32).pow(2) as f64;
                total_diff += (target_pixel[1] as i32 - skin_pixel[1] as i32).pow(2) as f64;
                total_diff += (target_pixel[2] as i32 - skin_pixel[2] as i32).pow(2) as f64;
                total_pixels += 3;
            }
        }
    }

    Ok(total_diff / total_pixels as f64)
}

fn avg_grid_pixel_diff(cell: &Cell, board: &Board, skin: &BlockSkin, skin_id: usize, target_img: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff: f64 = 0.0;
    let mut total_pixels: u32 = 0;
    let skin_img = skin.as_array_ref()[skin_id];

    for cell_y in 0..2 {
        for cell_x in 0..2 {
            let curr_cell = Cell { x: cell.x + cell_x - 1, y: cell.y + cell_y - 1 };
            match board.get(&curr_cell) {
                Err(_) => continue,
                _ => (),
            }

            for y in 0..skin.height() {
                for x in 0..skin.width() {
                    let target_pixel = target_img.get_pixel((curr_cell.x as u32 * skin.width() + x) as u32, (curr_cell.y as u32 * skin.height() + y) as u32);
                    let skin_pixel = skin_img.get_pixel(x, y);
                    total_diff += (target_pixel[0] as i32 - skin_pixel[0] as i32).pow(2) as f64;
                    total_diff += (target_pixel[1] as i32 - skin_pixel[1] as i32).pow(2) as f64;
                    total_diff += (target_pixel[2] as i32 - skin_pixel[2] as i32).pow(2) as f64;
                    total_pixels += 3;
                }
            }
        }
    }

    Ok(total_diff / total_pixels as f64)
}