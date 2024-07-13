use std::error::Error;
use crate::piece::{Cell, Piece, Orientation};

pub struct Board {
    pub cells: [[char; 10]; 20]
}

impl Board {
    pub fn new() -> Board {
        Board { cells: [[' '; 10]; 20] }
    }

    pub fn print(&self) {
        for row in self.cells.iter() {
            println!("{}", row.iter().collect::<String>());
        }
    }

    pub fn place(&mut self, piece: &Piece) -> Result<(), Box<dyn Error>> {
        let to_occupy = piece.get_occupancy()?;
        for i in to_occupy.iter() {
            if !self.check_cell(i) {
                return Err(format!("Cell ({}, {}) is out of bounds", i.x, i.y).into());
            }
            else if self.cells[i.y][i.x] != ' ' {
                return Err(format!("Cell ({}, {}) is not empty", i.x, i.y).into());
            }
            self.cells[i.y][i.x] = 'X';
        }
        Ok(())
    }

    fn check_cell(&self, cell: &Cell) -> bool {
        cell.x < 10 && cell.y < 20
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_empty_board() {
        let mut board = Board::new();
        let piece = Piece::I(Cell { x: 1, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_ok());
    }

    #[test]
    fn test_place_out_of_bounds_low() {
        let mut board = Board::new();
        let piece = Piece::I(Cell { x: 0, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_out_of_bounds_high() {
        let mut board = Board::new();
        let piece = Piece::I(Cell { x: 8, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_overlap() {
        let mut board = Board::new();
        let piece = Piece::I(Cell { x: 2, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_ok());
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_overlap_2() {
        let mut board = Board::new();
        let piece = Piece::I(Cell { x: 2, y: 0 }, Orientation::NORTH);
        let piece2 = Piece::T(Cell { x: 2, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_ok());
        assert!(board.place(&piece2).is_err());
    }
}