mod approx;
mod board;
mod draw;
mod piece;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_img = image::open("sources/rick-astley-890513150.jpg")?;
    // let source_img = image::open("sources/only_z.png")?;

    let config = draw::Config {
        skin: draw::BlockSkin::new("assets/HqGYC5G - Imgur.png")?,
        board_width: 192,
        board_height: 108,
    };

    let result_img = approx::approximate(&source_img, &config)?;
    result_img.save("results/board.png")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
}