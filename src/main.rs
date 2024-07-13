mod piece;
mod board;
mod draw;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let board = tki();
    let skin = draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?;
    draw::draw_board(&board, &skin, "results/board.png");
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
