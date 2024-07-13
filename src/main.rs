mod board;
mod draw;
mod piece;
mod genetic;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let board = tki();
    // let skin = draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?;
    // draw::draw_board_to_file(&board, &skin, "results/board.png");

    // let source_img = image::open("sources/rick-astley-890513150.jpg")?;
    // let source_img2 = image::open("sources/rick-astley2.jpg")?;
    // let result_img = image::open("results/board.png")?;

    // // rescale source_img to fit result_img
    // let resized_source_buffer = image::imageops::resize(&source_img, result_img.width(), result_img.height(), image::imageops::FilterType::Lanczos3);
    // let source_img = image::DynamicImage::from(resized_source_buffer);

    // let resized_source_buffer = image::imageops::resize(&source_img2, result_img.width(), result_img.height(), image::imageops::FilterType::Lanczos3);
    // let source_img2 = image::DynamicImage::from(resized_source_buffer);

    // let score = genetic::score(&source_img, &source_img)?;
    // println!("Source vs Source difference: {:?}", score);

    // let score = genetic::score(&source_img, &result_img)?;
    // println!("Source vs Result difference: {:?}", score);

    // let score = genetic::score(&source_img, &source_img2)?;
    // println!("Source vs Source2 difference: {:?}", score);

    let source_img = image::open("sources/rick-astley-890513150.jpg")?;
    let config = genetic::Config {
        population_size: 100,
        max_iterations: 100,
        max_breed_attempts: 200,
        skin: draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?,
        board_width: 10,
        board_height: 10,
    };

    let example_result = draw::draw_board(&tki(), &config.skin);
    let resized_source_buffer = image::imageops::resize(&source_img, example_result.width(), example_result.height(), image::imageops::FilterType::Lanczos3);
    let source_img = image::DynamicImage::from(resized_source_buffer);

    let result_img = genetic::genetic_algorithm(&source_img, config);
    result_img.save("results/board.png")?;

    Ok(())
}

fn tki() -> board::Board {
    let mut board = board::Board::new(10, 10);
    board.place(&piece::Piece::O(piece::Cell { x: 0, y: 0 }, piece::Orientation::NORTH)).unwrap();
    board.place(&piece::Piece::I(piece::Cell { x: 4, y: 0 }, piece::Orientation::NORTH)).unwrap();
    board.place(&piece::Piece::Z(piece::Cell { x: 2, y: 1 }, piece::Orientation::EAST)).unwrap();
    board.place(&piece::Piece::S(piece::Cell { x: 5, y: 2 }, piece::Orientation::SOUTH)).unwrap();
    board.place(&piece::Piece::J(piece::Cell { x: 9, y: 1 }, piece::Orientation::EAST)).unwrap();
    board.place(&piece::Piece::L(piece::Cell { x: 1, y: 2 }, piece::Orientation::NORTH)).unwrap();
    board.print();

    board
}
