mod approx_image;
mod approx_video;
mod board;
mod cli;
mod draw;
mod piece;
mod integration_test;

use std::path::PathBuf;

use clap::Parser;
use imageproc::image;
use rayon;

fn main() {
    let cli = cli::Cli::parse();

    let threads = cli.threads.unwrap_or(4);
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();
    println!("Using {} threads", threads);

    let prioritize_tetrominos = match cli.prioritize_tetrominos {
        true => approx_image::PrioritizeColor::Yes,
        false => approx_image::PrioritizeColor::No,
    };
    println!("Prioritizing tetrominos: {}", cli.prioritize_tetrominos);

    match cli.command {
        cli::Commands::Integration {board_width} => {
            let config = approx_image::Config {
                board_width: board_width.unwrap_or(100),
                board_height: 0, // height doesn't matter here since it will be auto-scaled
                prioritize_tetrominos
            };
            integration_test::run("sources", &config).unwrap()
        },
        cli::Commands::ApproxImage { source, output, board_width, board_height } => {
            run_approx_image(&source, &output, board_width, board_height, prioritize_tetrominos)
        }
        cli::Commands::ApproxVideo { source, output, board_width, board_height } => {
            let config = approx_image::Config {
                board_width,
                board_height,
                prioritize_tetrominos
            };
            approx_video::run(&source, &output, &config)
        }
    }
}

fn run_approx_image(source: &PathBuf, output: &PathBuf, board_width: usize, board_height: usize, prioritize_tetrominos: approx_image::PrioritizeColor) {
    println!("Approximating an image: {}", source.display());
    let config = approx_image::Config {
        board_width: board_width,
        board_height: board_height,
        prioritize_tetrominos
    };

    let mut source_img = image::open(source).unwrap();
    println!("Loaded {}x{} image", source_img.width(), source_img.height());

    let result_img = approx_image::run(&mut source_img, &config).unwrap();
    result_img.save(output).expect("could not save output image");
}

#[cfg(test)]
mod tests {
    use std::{fs, path};

    use draw::SkinnedBoard;
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    use super::*;

    #[test]
    fn test_draw_all_pieces() {
        rayon::ThreadPoolBuilder::new().num_threads(8).build_global().unwrap();

        let width = 10;
        let height = 20;
        let skin_id = 0;
        let test_dir = "test_results";
        if !path::Path::new(&test_dir).exists() {
            fs::create_dir(test_dir).unwrap();
        }

        let all_piece_types: Vec<_> = piece::Orientation::all()
            .into_iter()
            .flat_map(|o| piece::Piece::all_normal(piece::Cell { x: 4, y: 4 }, o))
            .collect();

        all_piece_types
            .into_par_iter()
            .for_each(|piece| {
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
            });
    }
}