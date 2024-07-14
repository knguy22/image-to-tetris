use crate::board::Board;
use crate::draw::{self, BlockSkin, Config};
use crate::piece::{Cell, Piece, Orientation};

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
    while heap.len() > 0 {
        let cell = heap.pop().unwrap();

        // 1. check if the cell is unoccupied
        if *board.get(&cell)? != ' ' {
            continue;
        }

        // 2. for each possible piece and orientation:
        let mut best_piece: Option<Piece> = None;
        let mut best_piece_diff = std::f64::MAX;
        for orientation in Orientation::all() {
            for piece in Piece::all(cell, orientation.clone()) {
                if board.can_place(&piece) {
                    let diff = avg_piece_pixel_diff(&piece, &board, &skin, &target_img)?;
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

pub fn diff_images(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(diff_images_image_compare(image1, image2)?)
    // Ok(diff_images_dssim(image1, image2)?)
}

fn diff_images_image_compare(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, CompareError> {
    Ok(rgb_hybrid_compare(&image1.clone().to_rgb8(), &image2.clone().into_rgb8())?.score)
}

fn diff_images_dssim(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
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

fn avg_piece_pixel_diff(piece: &Piece, board: &Board, skin: &BlockSkin, target_img: &DynamicImage) -> Result<f64, Box<dyn std::error::Error>> {
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