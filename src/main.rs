mod board;
mod draw;
mod piece;
mod score;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let board = tki();
    let skin = draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?;
    draw::draw_board(&board, &skin, "results/board.png");

    let source_img = image::open("sources/rick-astley-890513150.jpg")?;
    let source_img2 = image::open("sources/rick-astley2.jpg")?;
    let result_img = image::open("results/board.png")?;

    // rescale source_img to fit result_img
    let resized_source_buffer = image::imageops::resize(&source_img, result_img.width(), result_img.height(), image::imageops::FilterType::Lanczos3);
    let source_img = image::DynamicImage::from(resized_source_buffer);

    let resized_source_buffer = image::imageops::resize(&source_img2, result_img.width(), result_img.height(), image::imageops::FilterType::Lanczos3);
    let source_img2 = image::DynamicImage::from(resized_source_buffer);

    let diff = score::compare_images(&source_img, &source_img)?;
    println!("Source vs Source difference: {:?}", diff.score);

    let diff = score::compare_images(&source_img, &result_img)?;
    println!("Source vs Result difference: {:?}", diff.score);

    let diff = score::compare_images(&source_img, &source_img2)?;
    println!("Source vs Source2 difference: {:?}", diff.score);

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
