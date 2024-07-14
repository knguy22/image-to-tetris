use crate::board::Board;
use crate::draw::{self, BlockSkin, Config};
use crate::piece::{Cell, Piece, Orientation};

use std::collections::BinaryHeap;

use imageproc::image::{DynamicImage, GenericImageView};

pub fn approximate(target_img: &DynamicImage, config: &Config) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    // resize the skin
    let (width, height) = target_img.dimensions();
    let skin = config.skin.resize(width / u32::try_from(config.board_width)?, height / u32::try_from(config.board_height)?);

    // initialize the board
    let mut board = Board::new(config.board_width, config.board_height);

    // resize the target image to account for rounding errors
    let resized_target_width = skin.width() * u32::try_from(board.width)?;
    let resized_target_height = skin.height() * u32::try_from(board.height)?;
    let resized_target_buffer = image::imageops::resize(target_img, resized_target_width, resized_target_height, image::imageops::FilterType::Lanczos3);
    let target_img = image::DynamicImage::from(resized_target_buffer);
    target_img.save("results/tmp_target.png")?;

    // init the heap and push the first row of cells into it
    // the first row is the highest row in number because we are using a max heap
    let mut heap = BinaryHeap::new();
    for x in 0..config.board_width {
        heap.push(Cell { x: x, y: config.board_height - 1 });
    }

    // for each cell at the top of the heap:
    while heap.len() > 0 {
        let cell = heap.pop().unwrap();

        // 1. check if the cell is unoccupied
        if *board.get(&cell)? != ' ' {
            continue;
        }

        // 2. for each possible piece and orientation:
        let mut best_piece: Option<Piece> = None;
        let mut best_piece_diff = avg_grid_pixel_diff(&cell, &board, &skin, &target_img)?;
        for orientation in Orientation::all() {
            for piece in Piece::all(cell, orientation.clone()) {
                if board.can_place(&piece) {
                    let diff = avg_piece_pixel_diff(&piece, &skin, &target_img)?;
                    if diff < best_piece_diff {
                        best_piece = Some(piece);
                        best_piece_diff = diff;
                    }
                }
            }
        }

        // 3. if we found a piece, place it
        if best_piece.is_some() {
            let best_piece = best_piece.unwrap();
            board.place(&best_piece)?;

            // check cells above to push into heap
            for piece_cell in best_piece.get_occupancy()? {
                if piece_cell.y > 0 {
                    heap.push(Cell { x: piece_cell.x, y: piece_cell.y - 1 });
                }
            }
        }
    }

    // draw the board
    Ok(draw::draw_board(&board, &skin))
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

                total_diff += (target_pixel[0] as i32 - skin_pixel[0] as i32).abs() as f64;
                total_diff += (target_pixel[1] as i32 - skin_pixel[1] as i32).abs() as f64;
                total_diff += (target_pixel[2] as i32 - skin_pixel[2] as i32).abs() as f64;
                total_pixels += 3;
            }
        }
    }

    Ok(total_diff / total_pixels as f64)
}


// 3 x 3 grid centered around the cell
fn avg_grid_pixel_diff(cell: &Cell, board: &Board, skin: &BlockSkin, target_img: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let mut total_diff: f64 = 0.0;
    let mut total_pixels: u32 = 0;

    for cell_y in 0..3 {
        for cell_x in 0..3 {
            let curr_cell = Cell { x: cell.x + cell_x - 1, y: cell.y + cell_y - 1 };
            match board.get(&curr_cell) {
                Err(_) => continue,
                _ => (),
            }

            for y in 0..skin.height() {
                for x in 0..skin.width() {
                    let target_pixel = target_img.get_pixel((curr_cell.x as u32 * skin.width() + x) as u32, (curr_cell.y as u32 * skin.height() + y) as u32);
                    let skin_pixel = skin.i_img.get_pixel(x, y);
                    total_diff += (target_pixel[0] as i32 - skin_pixel[0] as i32).abs() as f64;
                    total_diff += (target_pixel[1] as i32 - skin_pixel[1] as i32).abs() as f64;
                    total_diff += (target_pixel[2] as i32 - skin_pixel[2] as i32).abs() as f64;
                    total_pixels += 3;
                }
            }
        }
    }

    Ok(total_diff / total_pixels as f64)
}