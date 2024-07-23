use crate::board::EMPTY_CELL;
use crate::draw::{self, BlockSkin, SkinnedBoard};
use crate::piece::{Cell, Piece, Orientation};

use std::collections::BinaryHeap;

use image::Rgba;
use imageproc::image::{DynamicImage, GenericImageView};

#[derive(Copy, Clone)]
pub enum PrioritizeColor {
    Yes,
    No
}

enum UseGarbage {
    Yes,
    No
}

#[derive(Copy, Clone)]
pub struct Config {
    pub board_width: usize,
    pub board_height: usize,
    pub prioritize_tetrominos: PrioritizeColor,
}

// the target image will be changed in order to fit the scaling of the board
pub fn run(target_img: &mut DynamicImage, config: &Config) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    // initialize the board
    let mut board = SkinnedBoard::new(config.board_width, config.board_height);

    // resize the skins
    let (pixels_width, pixels_height) = target_img.dimensions();
    let skin_width = pixels_width / u32::try_from(board.board_width())?;
    let skin_height = pixels_height / u32::try_from(board.board_height())?;
    if skin_width == 0 || skin_height == 0 {
        return Err("Skin dimensions must be greater than 0".into());
    }
    board.resize_skins(skin_height, skin_width);

    // resize the target image to account for rounding errors
    *target_img = resize_img_from_board(&board, target_img)?;

    // initialize average pixels for context reasons during approximation
    let avg_pixel_grid = average_pixel_grid(&target_img, board.skins_width(), board.skins_height())?;

    // init the heap and push the first row of cells into it
    // the first row is the highest row in number because we are using a max heap
    let mut heap = BinaryHeap::new();
    for y in (0..board.board_height()).rev() {
        for x in 0..board.board_width() {
            heap.push(Cell { x: x, y: y });
        }
    }

    // perform the approximation
    match config.prioritize_tetrominos {
        PrioritizeColor::Yes => process_heap_prioritize(&mut heap, &mut board, target_img, &avg_pixel_grid)?,
        PrioritizeColor::No => process_heap(&mut heap, &mut board, target_img, &avg_pixel_grid, UseGarbage::Yes)?
    }

    // draw the board
    Ok(draw::draw_board(&board))
}

fn process_heap_prioritize(heap: &mut BinaryHeap<Cell>, board: &mut SkinnedBoard, target_img: &DynamicImage, avg_pixel_grid: &Vec<Rgba<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    // first try to not use garbage to avoid gray and black blocks
    process_heap(heap, board, target_img, &avg_pixel_grid, UseGarbage::No)?;

    // then use garbage with the remaining unfilled cells
    for y in (0..board.board_height()).rev() {
        for x in 0..board.board_width() {
            let cell = Cell { x: x, y: y };
            if board.empty_at(&cell) {
                heap.push(cell);
            }
        }
    }
    process_heap(heap, board, target_img, &avg_pixel_grid, UseGarbage::Yes)?;
    Ok(())
}

fn process_heap(heap: &mut BinaryHeap<Cell>, board: &mut SkinnedBoard, target_img: &DynamicImage, avg_pixel_grid: &Vec<Rgba<u8>>, use_garbage: UseGarbage) -> Result<(), Box<dyn std::error::Error>> {
    // for each cell at the top of the heap:
    while heap.len() > 0 {
        let cell = heap.pop().expect("heap should not be empty");

        // 1. check if the cell is unoccupied
        if !board.empty_at(&cell) {
            continue;
        }

        // 2. for each possible skin, piece, and orientation:
        let mut best_piece: Option<Piece> = None;
        let mut best_piece_diff = f64::MAX;
        let mut best_skin_id: Option<usize> = None;

        for skin in board.iter_skins() {
            match use_garbage {
                // try black or gray garbage
                UseGarbage::Yes => {
                    for piece in Piece::all_garbage(cell) {
                        let diff = avg_piece_pixel_diff(&piece, &board, skin, &target_img, &avg_pixel_grid)?;
                        if diff < best_piece_diff {
                            best_piece = Some(piece);
                            best_piece_diff = diff;
                            best_skin_id = Some(skin.id());
                        }
                    }
                }
                UseGarbage::No => (),
            };

            // try placing pieces
            for orientation in Orientation::all() {
                for piece in Piece::all_normal(cell, orientation) {
                    if board.board().can_place(&piece) {
                        let diff = avg_piece_pixel_diff(&piece, &board, &skin, &target_img, &avg_pixel_grid)?;
                        if diff < best_piece_diff {
                            best_piece = Some(piece);
                            best_piece_diff = diff;
                            best_skin_id = Some(skin.id());
                        }
                    }
                }
            }
        }

        // place the best piece
        match best_piece {
            Some(best_piece) => board.place(&best_piece, best_skin_id.expect("there must be a best skin"))?,
            None => (),
        }
    }

    Ok(())
}

fn resize_img_from_board(board: &SkinnedBoard, target_img: &DynamicImage) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    // resize the target image to account for rounding errors
    let resized_target_width = board.skins_width() * u32::try_from(board.board_width())?;
    let resized_target_height = board.skins_height() * u32::try_from(board.board_height())?;
    let resized_target_buffer = image::imageops::resize(target_img, resized_target_width, resized_target_height, image::imageops::FilterType::Lanczos3);
    Ok(image::DynamicImage::from(resized_target_buffer))
}

fn average_pixel_grid(target_img: &DynamicImage, pixels_grid_width: u32, pixels_grid_height: u32) -> Result<Vec<Rgba<u8>>, Box<dyn std::error::Error>> {
    // check pixels are evenly divided into the grid
    let (pixels_w, pixels_h) = target_img.dimensions();
    if pixels_w % pixels_grid_width != 0 || pixels_h % pixels_grid_height != 0 {
        return Err("Pixels must be evenly divided into the grid".into());
    }

    // now divide pixels into the grid and compute the average pixel for each
    let pixels_per_grid = pixels_grid_width * pixels_grid_height;
    let mut avg_pixels = Vec::new();

    // for each grid in the image, calculate an average
    for pixels_y_range in (0..pixels_h).step_by(pixels_grid_height as usize) {
        for pixels_x_range in (0..pixels_w).step_by(pixels_grid_width as usize) {
            let mut pixel_sum: [u32; 4]= [0, 0, 0, 0];

            // calculate the sum using each pixel in the grid
            for y in 0..pixels_grid_height {
                for x in 0..pixels_grid_width {
                    let pixel = target_img.get_pixel(pixels_x_range + x, pixels_y_range + y);
                    pixel_sum[0] += pixel[0] as u32;
                    pixel_sum[1] += pixel[1] as u32;
                    pixel_sum[2] += pixel[2] as u32;
                    pixel_sum[3] += pixel[3] as u32;
                }
            }

            // divide by the number of pixels in the grid
            let pixel_avg: Rgba<u8> = [
                u8::try_from(pixel_sum[0] / pixels_per_grid)?,
                u8::try_from(pixel_sum[1] / pixels_per_grid)?,
                u8::try_from(pixel_sum[2] / pixels_per_grid)?,
                u8::try_from(pixel_sum[3] / pixels_per_grid)?,
            ].into();

            avg_pixels.push(pixel_avg);
        }
    }

    Ok(avg_pixels)
}

fn avg_piece_pixel_diff(piece: &Piece, board: &SkinnedBoard, skin: &BlockSkin, target_img: &DynamicImage, avg_pixel_grid: &Vec<Rgba<u8>>) -> Result<f64, Box<dyn std::error::Error>> {
    let mut curr_pixel_diff: f64 = 0.0;
    let mut total_curr_pixels: u32 = 0;

    let mut context_pixel_diff: f64 = 0.0;
    let mut total_context_pixels: u32 = 0;

    let block_image = skin.block_image_from_piece(piece);

    let center_cell = piece.get_cell();
    let occupancy = piece.get_occupancy()?;
    let context_cells = find_context_cells(board, &occupancy, &center_cell);

    // used to weigh the importance of each diff
    const RED_WEIGHT: f64 = 1.0;
    const GREEN_WEIGHT: f64 = 1.7;
    const BLUE_WEIGHT: f64 = 0.8;

    let avg_board_cell_pixel = block_image.get_average_pixel();
    let avg_target_cell_pixel = find_average_target_cell_pixel(avg_pixel_grid, &occupancy, board);
    for cell in occupancy {
        // first analyze the context using average pixels
        for context_cell in &context_cells {
            let cell_char = board.board().get(&cell)?;
            let skin_id = board.get_cells_skin(&context_cell);

            let context_skin = board.get_skin(skin_id);
            let context_block_image = context_skin.block_image_from_char(cell_char);
            let avg_board_context_pixel = context_block_image.get_average_pixel();

            let avg_target_context_pixel = avg_pixel_grid[(context_cell.y * board.board_width() + context_cell.x) as usize];

            let board_context_diff = subtract_pixels(&avg_board_cell_pixel, &avg_board_context_pixel);
            let target_context_diff = subtract_pixels(&avg_target_cell_pixel, &avg_target_context_pixel);

            context_pixel_diff += f64::sqrt(
                (board_context_diff[0] - target_context_diff[0]).pow(2) as f64 * RED_WEIGHT +
                (board_context_diff[1] - target_context_diff[1]).pow(2) as f64 * GREEN_WEIGHT +
                (board_context_diff[2] - target_context_diff[2]).pow(2) as f64 * BLUE_WEIGHT
            );
            total_context_pixels += 1;
        }

        // then analyze the individual cell to find the pixel difference between the current cells
        for y in 0..skin.height() {
            for x in 0..skin.width() {
                let target_pixel = target_img.get_pixel((cell.x as u32 * skin.width() + x) as u32, (cell.y as u32 * skin.height() + y) as u32);
                let approx_pixel = block_image.get_pixel(x, y);
                let curr_diff = subtract_pixels(&target_pixel, &approx_pixel);
                curr_pixel_diff += 
                    curr_diff[0].pow(2) as f64 * RED_WEIGHT +
                    curr_diff[1].pow(2) as f64 * GREEN_WEIGHT +
                    curr_diff[2].pow(2) as f64 * BLUE_WEIGHT
                ;
                total_curr_pixels += 1;
            }
        }
    }

    // weight the context diff in comparison with the current diff
    let avg_pixel_diff = 
        if total_context_pixels != 0 {
            curr_pixel_diff / total_curr_pixels as f64 + context_pixel_diff / total_context_pixels as f64 
        } else {
            curr_pixel_diff / total_curr_pixels as f64
        };


    Ok(avg_pixel_diff)
}

fn find_context_cells(board: &SkinnedBoard, occupancy: &Vec<Cell>, center_cell: &Cell) -> Vec<Cell> {
    let mut context_cells: Vec<Cell> = Vec::new();

    // get the context cells
    const MIN_DX: i32 = 0;
    const MIN_DY: i32 = 0;
    const MAX_DX: i32 = 8;
    const MAX_DY: i32 = 8;

    let mut dy: i32 = MIN_DY;
    while dy < MAX_DY {
        // compute and check the new y coordinate
        let new_y = usize::try_from((center_cell.y as i32) + dy);
        let new_y = match new_y {
            Ok(y) => y,
            Err(_) => {
                dy += 1;
                continue
            }
        };

        let mut dx: i32 = MIN_DX;
        while dx < MAX_DX {
            // compute and check the new x coordinate
            let new_x = usize::try_from((center_cell.x as i32) + dx);
            let new_x = match new_x {
                Ok(x) => x,
                Err(_) => {
                    dx += 1;
                    continue
                }
            };

            // only append contexts that are occupied with other pieces we already placed
            let context_cell = Cell {x: new_x, y: new_y};
            let context_char = board.board().get(&context_cell);
            if context_char.is_ok() && *context_char.expect("there must be a context char") != EMPTY_CELL && !occupancy.contains(&context_cell) {
                context_cells.push(context_cell);
            }
            dx += 1;
        }

        dy += 1;
    }

    context_cells
}

fn find_average_target_cell_pixel(avg_pixel_grid: &Vec<Rgba<u8>>, occupancy: &Vec<Cell>, board: &SkinnedBoard) -> Rgba<u8> {
    let mut pixel_sum: [u32; 4] = [0, 0, 0, 0];

    for cell in occupancy {
        let pixel = &avg_pixel_grid[cell.y * board.board_width() + cell.x];
        pixel_sum[0] += pixel[0] as u32;
        pixel_sum[1] += pixel[1] as u32;
        pixel_sum[2] += pixel[2] as u32;
        pixel_sum[3] += pixel[3] as u32;
    }

    pixel_sum.map(|x| u8::try_from(x / occupancy.len() as u32).expect("pixel should be in range")).into()
}

fn subtract_pixels(a: &Rgba<u8>, b: &Rgba<u8>) -> [i32; 3] {
    [
        a[0] as i32 - b[0] as i32,
        a[1] as i32 - b[1] as i32,
        a[2] as i32 - b[2] as i32,
    ]
}