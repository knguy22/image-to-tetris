use std::error::Error;

#[derive(Clone)]
pub struct Dir {
    pub x: i32,
    pub y: i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell {
    pub x: usize,
    pub y: usize
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Orientation {
    North,
    East,
    South,
    West
}

#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    I(Cell, Orientation),
    O(Cell, Orientation),
    T(Cell, Orientation),
    L(Cell, Orientation),
    J(Cell, Orientation),
    S(Cell, Orientation),
    Z(Cell, Orientation),
    Gray(Cell),
    Black(Cell),
}

// constants modified from https://github.com/freyhoe/ditzy22/blob/main/common.h

const I_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: 0, y: 0 }, Dir{ x: 1, y: 0 }, Dir{ x: 2, y: 0 }, Dir{ x: 3, y: 0 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: -2 }, Dir{ x: 0, y: -3 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }, Dir{ x: -3, y: 0 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: 0, y: 1 }, Dir{ x: 0, y: 2 }, Dir{ x: 0, y: 3 }]
];

const O_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }],
];

const T_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: 0, y: 1 }, Dir{ x: -1, y: 0 }, Dir{ x: 0, y: 0 }, Dir{ x: 1, y: 0 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 1 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }],
    [Dir{ x: -1, y: -1 }, Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }],
    [Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -2 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: 0 }],
];

const L_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: -2, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: 0 }],
    [Dir{ x: 1, y: -1 }, Dir{ x: 0, y: 1 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }, Dir{ x: -2, y: -1 }],
    [Dir{ x: -1, y: 0 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: -2 }],
];

const J_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: -1, y: 0 }, Dir{ x: 0, y: 0 }, Dir{ x: 1, y: 0 }, Dir{ x: -1, y: -1 }],
    [Dir{ x: -1, y: -2 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: -2 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }, Dir{ x: 0, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: -2 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: 0 }],
];

const S_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: -2, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: 0 }, Dir{ x: 0, y: 0 }],
    [Dir{ x: 0, y: 1 }, Dir{ x: 0, y: 0 }, Dir{ x: 1, y: 0 }, Dir{ x: 1, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }, Dir{ x: -2, y: -1 }],
    [Dir{ x: 1, y: -1 }, Dir{ x: 1, y: 0 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: 1 }],
];

const Z_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: 1, y: 0 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: 1 }, Dir{ x: -1, y: 1 }],
    [Dir{ x: -1, y: -2 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: 0 }],
    [Dir{ x: -1, y: 1 }, Dir{ x: 0, y: 1 }, Dir{ x: 0, y: 0 }, Dir{ x: 1, y: 0 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: -2 }],
];

impl Orientation {
    pub fn all() -> [Orientation; 4] {
        [Orientation::North, Orientation::East, Orientation::South, Orientation::West]
    }
}

impl Piece {
    pub fn all_normal(cell: Cell, orientation: Orientation) -> Vec<Piece> {
        vec![
            Piece::I(cell, orientation),
            Piece::O(cell, orientation),
            Piece::T(cell, orientation),
            Piece::L(cell, orientation),
            Piece::J(cell, orientation),
            Piece::S(cell, orientation),
            Piece::Z(cell, orientation),
        ]
    }

    pub fn all_garbage(cell: Cell) -> Vec<Piece> {
        vec![Piece::Gray(cell), Piece::Black(cell)]
    }

    pub fn get_char(&self) -> char {
        match self {
            Piece::I(_, _) => 'I',
            Piece::O(_, _) => 'O',
            Piece::T(_, _) => 'T',
            Piece::L(_, _) => 'L',
            Piece::J(_, _) => 'J',
            Piece::S(_, _) => 'S',
            Piece::Z(_, _) => 'Z',
            Piece::Gray(_) => 'G',
            Piece::Black(_) => 'B'
        }
    }

    pub fn get_orientation(&self) -> Orientation {
        match self {
            Piece::I(_, o) => *o,
            Piece::O(_, o) => *o,
            Piece::T(_, o) => *o,
            Piece::L(_, o) => *o,
            Piece::J(_, o) => *o,
            Piece::S(_, o) => *o,
            Piece::Z(_, o) => *o,
            _ => panic!("Garbage or black piece has no orientation")
        }
    }

    pub fn get_cell(&self) -> Cell {
        match self {
            Piece::I(c, _) => *c,
            Piece::O(c, _) => *c,
            Piece::T(c, _) => *c,
            Piece::L(c, _) => *c,
            Piece::J(c, _) => *c,
            Piece::S(c, _) => *c,
            Piece::Z(c, _) => *c,
            Piece::Gray(c) => *c,
            Piece::Black(c) => *c
        }
    }

    pub fn get_occupancy(&self) -> Result<Vec<Cell>, Box<dyn Error>> {
        // only non-garbage pieces should have a shape
        let shape: &[[Dir; 4]; 4] = match self {
            Piece::I(_, _) => &I_SHAPE,
            Piece::O(_, _) => &O_SHAPE,
            Piece::T(_, _) => &T_SHAPE,
            Piece::L(_, _) => &L_SHAPE,
            Piece::J(_, _) => &J_SHAPE,
            Piece::S(_, _) => &S_SHAPE,
            Piece::Z(_, _) => &Z_SHAPE,
            Piece::Gray(c) | Piece::Black(c) => return Ok(vec![*c]),
        };

        let orien = self.get_orientation();
        let dirs = match orien {
            Orientation::North => shape[0].clone(),
            Orientation::East => shape[1].clone(),
            Orientation::South => shape[2].clone(),
            Orientation::West => shape[3].clone()
        };

        let mut occupancy = Vec::new();
        for dir in dirs {
            let x = self.get_cell().x as i32 + dir.x;
            let y = self.get_cell().y as i32 + dir.y;
            if x < 0 || y < 0 {
                return Err(format!("Cell ({}, {}) contains negative values", x, y).into());
            }

            occupancy.push(Cell { x: x as usize, y: y as usize });
        }
        Ok(occupancy)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_orientation() {
        let piece = Piece::I(Cell { x: 0, y: 0 }, Orientation::North);
        assert_eq!(piece.get_orientation(), Orientation::North);
    }

    #[test]
    fn test_get_cell() {
        let piece = Piece::I(Cell { x: 1, y: 1 }, Orientation::North);
        assert_eq!(piece.get_cell(), Cell { x: 1, y: 1 });
    }

    #[test]
    fn test_get_occupancy() {
        let piece = Piece::I(Cell { x: 2, y: 2 }, Orientation::North);
        assert!(piece.get_occupancy().is_ok());
    }

}