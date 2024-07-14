use crate::board::Board;
use crate::draw::{self, Config};
use crate::piece::{Cell, Piece, Orientation};

use std::collections::BinaryHeap;

use imageproc::image::{DynamicImage, SubImage, GenericImageView};
use image_compare::{self, CompareError, rgb_hybrid_compare};

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
    let mut pieces_placed: u32 = 0;
    while heap.len() > 0 {
        let cell = heap.pop().unwrap();

        // 1. check if the cell is unoccupied
        if *board.get(&cell)? != ' ' {
            continue;
        }

        // 2. for each possible piece and orientation:
        let mut best_piece: Option<Piece> = None;
        let mut best_score: f64 = 0.0;
        for orientation in Orientation::all() {
            for piece in Piece::all(cell, orientation) {
                // 2.1 check if the piece can be placed
                match board.place(&piece) {
                    Ok(_) => {
                        // 2.2 if so, score it
                        let score = score_board(&draw::draw_board(&board, &skin), &target_img)?;
                        if score > best_score {
                            best_piece = Some(piece);
                            best_score = score;
                        }
                        board.undo_last_move()?;
                    }
                    Err(_) => {}
                }
            }
        }

        // 3. if we found a piece, place it
        if best_piece.is_some() {
            let best_piece = best_piece.unwrap();
            board.place(&best_piece)?;

            pieces_placed += 1;
            if pieces_placed % 100 == 0 {
                println!("{} pieces placed", pieces_placed);
                println!("Best score: {}", best_score);
                let tmp = draw::draw_board(&board, &skin);
                tmp.save("results/tmp_board.png").unwrap();
            }

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

pub fn score_board(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, CompareError> {
    Ok(rgb_hybrid_compare(&image1.clone().to_rgb8(), &image2.clone().into_rgb8())?.score)
}