use crate::board::Board;
use crate::draw::{self, Config};
use crate::piece::{Cell, Piece, Orientation};

use std::any::Any;
use std::collections::BinaryHeap;

use imageproc::image::{DynamicImage, SubImage, GenericImageView};
use image_compare::{self, CompareError, rgb_hybrid_compare};
use dssim::{self, Dssim};
use rgb;

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
    let mut pieces_tried: u32 = 0;
    let mut best_diff: f64 = diff_images(&draw::draw_board(&board, &skin), &target_img)?;
    println!("Initial diff: {}", best_diff);

    while heap.len() > 0 {
        let cell = heap.pop().unwrap();

        // 1. check if the cell is unoccupied
        if *board.get(&cell)? != ' ' {
            continue;
        }

        // 2. for each possible piece and orientation:
        let mut best_piece: Option<Piece> = None;
        for orientation in Orientation::all() {
            for piece in Piece::all(cell, orientation.clone()) {
                pieces_tried += 1;

            if pieces_tried % 500 == 0 {
                println!("pieces tried: {}", pieces_tried);
            }

                // 2.1 check if the piece can be placed
                match board.place(&piece) {
                    Ok(_) => {
                        // 2.2 if so, diff it
                        let diff = diff_images(&draw::draw_board(&board, &skin), &target_img)?;
                        if diff < best_diff {
                            best_piece = Some(piece);
                            best_diff = diff;
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

            println!("diff: {}, pieces placed: {}", best_diff, board.pieces.len());

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

pub fn diff_images(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(diff_images_image_compare(image1, image2)?)
    // Ok(diff_images_dssim(image1, image2)?)
}

pub fn diff_images_image_compare(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, CompareError> {
    Ok(rgb_hybrid_compare(&image1.clone().to_rgb8(), &image2.clone().into_rgb8())?.score)
}

pub fn diff_images_dssim(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    let d = Dssim::new();

    let image1_buffer = image1.to_rgb8();
    let image2_buffer = image2.to_rgb8();

    let image1_rgb = rgb::FromSlice::as_rgb(image1_buffer.as_raw().as_slice());
    let image2_rgb = rgb::FromSlice::as_rgb(image2_buffer.as_raw().as_slice());

    let d_image1 = d.create_image_rgb(image1_rgb, image1.width() as usize, image1.height() as usize).unwrap();
    let d_image2 = d.create_image_rgb(image2_rgb, image2.width() as usize, image2.height() as usize).unwrap();

    let (diff, _) = d.compare(&d_image1, &d_image2);
    Ok(diff.into())
}