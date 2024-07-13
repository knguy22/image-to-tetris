mod board;
mod draw;
mod piece;
mod genetic;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_img = image::open("sources/rick-astley-890513150.jpg")?;
    let config = genetic::Config {
        population_size: 50,
        max_iterations: 200,
        max_breed_attempts: 400,
        skin: draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?,
        board_width: 30,
        board_height: 30,
    };

    let example_result = draw::draw_board(&tki(config.board_width, config.board_height), &config.skin);
    let resized_source_buffer = image::imageops::resize(&source_img, example_result.width(), example_result.height(), image::imageops::FilterType::Lanczos3);
    let source_img = image::DynamicImage::from(resized_source_buffer);

    let result_img = genetic::genetic_algorithm(&source_img, config);
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
