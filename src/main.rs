mod approx;
mod board;
mod draw;
mod piece;

use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    // the image to be approximated
    #[clap(long, short, action)]
    source_img: PathBuf,

    // the output image
    #[clap(long, short, action)]
    output_img: PathBuf,

    // board width
    #[clap(long, short, action)]
    width: Option<usize>,

    // board height
    #[clap(long, short, action)]
    height: Option<usize>,
}


fn main() {
    let cli = Cli::parse();

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

    let source_img = image::open(cli.source_img).unwrap();
    println!("Loaded {}x{} image", source_img.width(), source_img.height());

    let result_img = approx::approximate(&source_img, config).unwrap();
    result_img.save(cli.output_img).unwrap();
}

#[cfg(test)]
mod tests {
    use draw::SkinnedBoard;

    use super::*;

    #[test]
    fn test_draw_all_pieces() {
        for orientation in piece::Orientation::all() {
            for piece in piece::Piece::all(piece::Cell { x: 4, y: 4 }, orientation) {
                let mut board = SkinnedBoard::new(10, 20);

                board.place(&piece, 0).unwrap();
                let img = draw::draw_board(&board);
                img.save(format!("results/{:?} {:?}.png", piece, piece.get_orientation())).unwrap();
            }
        }
    }
}