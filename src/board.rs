use std::error::Error;
use crate::piece::{Cell, Piece, Orientation};

pub struct Board {
    pub cells: Vec<char>,
    width: usize,
    height: usize
}

impl Board {
    pub fn new(width: usize, height: usize) -> Board {
        Board {
            cells: vec![' '; width * height],
            width,
            height
        }
    }

    pub fn print(&self) {
        println!("+{}+", "-".repeat(self.width));
        for row in self.cells.chunks(self.width).rev() {
            println!("|{}|", row.iter().collect::<String>());
        }
        println!("+{}+", "-".repeat(self.width));
    }

    pub fn place(&mut self, piece: &Piece) -> Result<(), Box<dyn Error>> {
        let to_occupy = piece.get_occupancy()?;

        // check if cells are empty
        for cell in to_occupy.iter() {
            let curr = self.get(cell)?;
            if *curr != ' ' {
                return Err(format!("{:?} is not empty at {:?}", piece, cell).into());
            }
        }

        // if so, place
        for cell in to_occupy.iter() {
            *self.get(cell)? = piece.get_char();
        }

        Ok(())
    }

    fn get(&mut self, cell: &Cell) -> Result<&mut char, Box<dyn Error>> {
        if !(cell.x < self.width && cell.y < self.height) {
            return Err(format!("Cell ({}, {}) is out of bounds", cell.x, cell.y).into());
        }
        Ok(&mut self.cells[cell.y * self.width + cell.x])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_empty_board() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 1, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_ok());
    }

    #[test]
    fn test_place_out_of_bounds_low() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 0, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_out_of_bounds_high() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 8, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_overlap() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 2, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_ok());
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_overlap_2() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 2, y: 0 }, Orientation::NORTH);
        let piece2 = Piece::T(Cell { x: 2, y: 0 }, Orientation::NORTH);
        assert!(board.place(&piece).is_ok());
        assert!(board.place(&piece2).is_err());
    }
}