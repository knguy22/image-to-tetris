mod approx;
mod board;
mod draw;
mod piece;
mod genetic;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_img = image::open("sources/rick-astley-890513150.jpg")?;
    // let source_img = image::open("sources/only_z.png")?;

    let draw_config = draw::Config {
        skin: draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?,
        board_width: 192,
        board_height: 108,
    };

    let config = genetic::Config {
        population_size: 25,
        max_iterations: 300,
        max_breed_attempts: 600,
        mutation_rate: 5,
        draw_config: draw_config,
    };

    let result_img = approx::approximate(&source_img, &config.draw_config)?;
    result_img.save("results/board.png")?;

    Ok(())
}

fn tki(width: usize, height: usize) -> board::Board {
    let mut board = board::Board::new(width, height);
    board.place(&piece::Piece::O(piece::Cell { x: 0, y: 0 }, piece::Orientation::NORTH)).unwrap();
    board.place(&piece::Piece::I(piece::Cell { x: 4, y: 0 }, piece::Orientation::NORTH)).unwrap();
    board.place(&piece::Piece::Z(piece::Cell { x: 2, y: 1 }, piece::Orientation::EAST)).unwrap();
    board.place(&piece::Piece::S(piece::Cell { x: 5, y: 2 }, piece::Orientation::SOUTH)).unwrap();
    board.place(&piece::Piece::J(piece::Cell { x: 9, y: 1 }, piece::Orientation::EAST)).unwrap();
    board.place(&piece::Piece::L(piece::Cell { x: 1, y: 2 }, piece::Orientation::NORTH)).unwrap();
    board.print();

    board
}

fn test_draw_all_pieces() {
    let draw_config = draw::Config {
        skin: draw::BlockSkin::new("assets/HqGYC5G - Imgur.png").unwrap(),
        board_width: 120,
        board_height: 67,
    };

    for orientation in piece::Orientation::all() {
        for piece in piece::Piece::all(piece::Cell { x: 4, y: 4 }, orientation) {
            let mut board = board::Board::new(10, 20);
            board.place(&piece).unwrap();
            let img = draw::draw_board(&board, &draw_config.skin);
            img.save(format!("results/{:?} {:?}.png", piece, piece.get_orientation())).unwrap();
        }
    }
}