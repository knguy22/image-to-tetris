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
        for orientation in Orientation::all() {
            for piece in Piece::all(cell, orientation) {
                // 2.1 check if the piece can be placed
                if board.can_place(&piece) {
                    // 2.2 select the best one
                    best_piece = Some(piece);
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