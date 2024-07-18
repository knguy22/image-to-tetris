mod approx;
mod board;
mod cli;
mod draw;
mod piece;
mod test_many;

use clap::Parser;
use imageproc::image;

fn main() {
    let cli = cli::Cli::parse();

    // run integration tests if possible; source and output are ignored
    if cli.integration_tests {
        test_many::run("sources", 100).unwrap();
        return
    }

    // otherwise approximate an image; these arguments must be set
    let source_img_path = cli.source_img.unwrap();
    let output_img_path = cli.output_img.unwrap();

    let board_width = match cli.width {
        Some(width) => width,
        None => 10,
    };
    let board_height = match cli.height {
        Some(height) => height,
        None => 20,
    };

    let config = draw::Config {
        board_width: board_width,
        board_height: board_height,
    };

    let mut source_img = image::open(source_img_path).unwrap();
    println!("Loaded {}x{} image", source_img.width(), source_img.height());

    let result_img = approx::approximate(&mut source_img, &config).unwrap();
    result_img.save(output_img_path).unwrap();
}

#[cfg(test)]
mod tests {
    use std::{fs, path};

    use draw::SkinnedBoard;

    use super::*;

    #[test]
    fn test_draw_all_pieces() {
        let width = 10;
        let height = 20;
        let skin_id = 0;
        let test_dir = "test_results";
        if !path::Path::new(&test_dir).exists() {
            fs::create_dir(test_dir).unwrap();
        }

        for orientation in piece::Orientation::all() {
            for piece in piece::Piece::all_normal(piece::Cell { x: 4, y: 4 }, orientation) {
                let mut board = SkinnedBoard::new(width, height);

                // place regular piece
                board.place(&piece, skin_id).unwrap();
                
                // fill the rest with black garbage
                for y in 0..height {
                    for x in 0..width {
                        let cell = piece::Cell { x: x, y: y };
                        if board.empty_at(&cell) {
                            board.place(&piece::Piece::Black(cell), skin_id).unwrap();
                        }
                    }
                }

                let img = draw::draw_board(&board);
                img.save(format!("{}/{:?} {:?}.png", test_dir, piece, piece.get_orientation())).unwrap();
            }
        }
    }
}