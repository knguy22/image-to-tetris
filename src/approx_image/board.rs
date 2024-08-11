use super::piece::{Cell, Piece};

use anyhow::Result;
use thiserror::Error;

#[derive(Clone)]
pub struct Board {
    cells: Vec<char>,
    pieces: Vec<Piece>,
    pub width: usize,
    pub height: usize
}

#[derive(Debug, Error)]
pub enum BoardError {
    #[error("Invalid cell: {0:?}")]
    InvalidCell(Cell),

    #[error("Occupied cell: {0:?}")]
    OccupiedCell(Cell),
}

pub const BLOCKED_CELL: char = 'B';
pub const EMPTY_CELL: char = ' ';

impl Board {
    pub fn new(width: usize, height: usize) -> Board {
        Board {
            cells: vec![EMPTY_CELL; width * height],
            pieces: Vec::new(),
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        println!("+{}+", "-".repeat(self.width));
        for row in self.cells.chunks(self.width).rev() {
            println!("|{}|", row.iter().collect::<String>());
        }
        println!("+{}+", "-".repeat(self.width));
    }

    pub fn can_place(&self, piece: &Piece) -> bool {
        let Ok(to_occupy) = piece.get_occupancy() else {return false;};
        for cell in &to_occupy {
            let Ok(curr) = self.get(cell) else {return false};
            if curr != EMPTY_CELL {
                return false;
            }
        }
        true
    }

    pub fn place(&mut self, piece: &Piece) -> Result<()> {
        let to_occupy = piece.get_occupancy()?;

        // check if cells are empty
        for cell in &to_occupy {
            let curr = self.get(cell)?;
            if curr != EMPTY_CELL {
                return Err(BoardError::OccupiedCell(*cell))?;
            }
        }

        // if so, place
        for cell in &to_occupy {
            *self.get_mut(cell)? = piece.get_char();
        }
        self.pieces.push(piece.clone());

        Ok(())
    }

    #[allow(dead_code)]
    pub fn undo_last_move(&mut self) -> Result<()> {
        assert!(!self.pieces.is_empty());

        let piece = self.pieces.pop().expect("pieces should not be empty");
        for cell in piece.get_occupancy()? {
            *self.get_mut(&cell)? = EMPTY_CELL;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_piece(&mut self, piece: &Piece) -> Result<()> {
        let to_occupy = piece.get_occupancy()?;
        for cell in &to_occupy {
            *self.get_mut(cell)? = EMPTY_CELL;
        }
        self.pieces.retain(|p| p != piece);
        Ok(())
    }

    pub fn get(&self, cell: &Cell) -> Result<char> {
        if !(cell.x < self.width && cell.y < self.height) {
            return Err(BoardError::InvalidCell(*cell))?;
        }
        Ok(self.cells[cell.y * self.width + cell.x])
    }

    pub fn get_mut(&mut self, cell: &Cell) -> Result<&mut char> {
        if !(cell.x < self.width && cell.y < self.height) {
            return Err(BoardError::InvalidCell(*cell))?;
        }
        Ok(&mut self.cells[cell.y * self.width + cell.x])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approx_image::piece::Orientation;

    #[test]
    fn test_place_empty_board() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 1, y: 0 }, Orientation::North);
        assert!(board.place(&piece).is_ok());
    }

    #[test]
    fn test_place_out_of_bounds_high() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 8, y: 0 }, Orientation::North);
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_overlap() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 2, y: 0 }, Orientation::North);
        assert!(board.place(&piece).is_ok());
        assert!(board.place(&piece).is_err());
    }

    #[test]
    fn test_place_overlap_2() {
        let mut board = Board::new(10, 20);
        let piece = Piece::I(Cell { x: 2, y: 0 }, Orientation::North);
        let piece2 = Piece::T(Cell { x: 2, y: 0 }, Orientation::North);
        assert!(board.place(&piece).is_ok());
        assert!(board.place(&piece2).is_err());
    }
}