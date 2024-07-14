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

#[derive(Clone, Debug, PartialEq)]
pub enum Orientation {
    NORTH,
    EAST,
    SOUTH,
    WEST
}

#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    I(Cell, Orientation),
    O(Cell, Orientation),
    T(Cell, Orientation),
    L(Cell, Orientation),
    J(Cell, Orientation),
    S(Cell, Orientation),
    Z(Cell, Orientation)
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
    [Dir{ x: -1, y: 0 }, Dir{ x: -2, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 1 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }],
    [Dir{ x: -1, y: -1 }, Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }],
    [Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -2 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: 0 }],
];

const L_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: -2, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: 0 }],
    [Dir{ x: 0, y: -2 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: -2 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }, Dir{ x: -2, y: -1 }],
    [Dir{ x: -1, y: 0 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: -2 }],
];

const J_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: -2, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }, Dir{ x: -2, y: 0 }],
    [Dir{ x: -1, y: -2 }, Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: -2 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }, Dir{ x: 0, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: -2 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: 0 }],
];

const S_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: -2, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: 0 }, Dir{ x: 0, y: 0 }],
    [Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: -2 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }, Dir{ x: -2, y: -1 }],
    [Dir{ x: 0, y: -2 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: 0 }],
];

const Z_SHAPE: [[Dir; 4]; 4] = [
    [Dir{ x: 0, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: 0 }, Dir{ x: -2, y: 0 }],
    [Dir{ x: -1, y: -2 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }, Dir{ x: 0, y: 0 }],
    [Dir{ x: -2, y: 0 }, Dir{ x: -1, y: 0 }, Dir{ x: -1, y: -1 }, Dir{ x: 0, y: -1 }],
    [Dir{ x: 0, y: 0 }, Dir{ x: 0, y: -1 }, Dir{ x: -1, y: -1 }, Dir{ x: -1, y: -2 }],
];

impl Orientation {
    pub fn all() -> [Orientation; 4] {
        [Orientation::NORTH, Orientation::EAST, Orientation::SOUTH, Orientation::WEST]
    }
}

impl Piece {
    pub fn all(cell: Cell, orientation: Orientation) -> Vec<Piece> {
        let mut pieces = Vec::new();
        pieces.push(Piece::I(cell.clone(), orientation.clone()));
        pieces.push(Piece::O(cell.clone(), orientation.clone()));
        pieces.push(Piece::T(cell.clone(), orientation.clone()));
        pieces.push(Piece::L(cell.clone(), orientation.clone()));
        pieces.push(Piece::J(cell.clone(), orientation.clone()));
        pieces.push(Piece::S(cell.clone(), orientation.clone()));
        pieces.push(Piece::Z(cell, orientation));
        pieces
    }

    pub fn get_char(&self) -> char {
        match self {
            Piece::I(_, _) => 'I',
            Piece::O(_, _) => 'O',
            Piece::T(_, _) => 'T',
            Piece::L(_, _) => 'L',
            Piece::J(_, _) => 'J',
            Piece::S(_, _) => 'S',
            Piece::Z(_, _) => 'Z'
        }
    }

    pub fn get_orientation(&self) -> Orientation {
        match self {
            Piece::I(_, o) => o.clone(),
            Piece::O(_, o) => o.clone(),
            Piece::T(_, o) => o.clone(),
            Piece::L(_, o) => o.clone(),
            Piece::J(_, o) => o.clone(),
            Piece::S(_, o) => o.clone(),
            Piece::Z(_, o) => o.clone()
        }
    }

    pub fn get_cell(&self) -> Cell {
        match self {
            Piece::I(c, _) => c.clone(),
            Piece::O(c, _) => c.clone(),
            Piece::T(c, _) => c.clone(),
            Piece::L(c, _) => c.clone(),
            Piece::J(c, _) => c.clone(),
            Piece::S(c, _) => c.clone(),
            Piece::Z(c, _) => c.clone()
        }
    }

    pub fn get_occupancy(&self) -> Result<[Cell; 4], Box<dyn Error>> {
        let shape: &[[Dir; 4]; 4] = match self {
            Piece::I(_, _) => &I_SHAPE,
            Piece::O(_, _) => &O_SHAPE,
            Piece::T(_, _) => &T_SHAPE,
            Piece::L(_, _) => &L_SHAPE,
            Piece::J(_, _) => &J_SHAPE,
            Piece::S(_, _) => &S_SHAPE,
            Piece::Z(_, _) => &Z_SHAPE
        };

        let orien = self.get_orientation();
        let dirs = match orien {
            Orientation::NORTH => shape[0].clone(),
            Orientation::EAST => shape[1].clone(),
            Orientation::SOUTH => shape[2].clone(),
            Orientation::WEST => shape[3].clone()
        };

        
        let mut occupancy: [Cell; 4] = [Cell { x: 0, y: 0 }; 4];
        for i in 0..4 {
            let x = self.get_cell().x as i32 + dirs[i].x;
            let y = self.get_cell().y as i32 + dirs[i].y;
            if x < 0 || y < 0 {
                return Err(format!("Cell ({}, {}) contains negative values", x, y).into());
            }

            occupancy[i] = Cell { x: x as usize, y: y as usize };
        }
        Ok(occupancy)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_orientation() {
        let piece = Piece::I(Cell { x: 0, y: 0 }, Orientation::NORTH);
        assert_eq!(piece.get_orientation(), Orientation::NORTH);
    }

    #[test]
    fn test_get_cell() {
        let piece = Piece::I(Cell { x: 1, y: 1 }, Orientation::NORTH);
        assert_eq!(piece.get_cell(), Cell { x: 1, y: 1 });
    }

    #[test]
    fn test_get_occupancy() {
        let piece = Piece::I(Cell { x: 2, y: 2 }, Orientation::NORTH);
        assert!(piece.get_occupancy().is_ok());
    }
    

    #[test]
    fn test_get_occupancy_negative() {
        let piece = Piece::I(Cell { x: 0, y: 0 }, Orientation::NORTH);
        assert!(piece.get_occupancy().is_err());
    }
}