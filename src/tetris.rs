pub struct Cell{
    x: i32,
    y: i32
}

pub enum Orientation {
    NORTH,
    EAST,
    SOUTH,
    WEST
}

pub enum Piece {
    I(Cell, Orientation),
    O(Cell, Orientation),
    T(Cell, Orientation),
    L(Cell, Orientation),
    J(Cell, Orientation),
    S(Cell, Orientation),
    Z(Cell, Orientation)
}

const I_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: -1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: 2, y: 0 }],
    [Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: -1 }, Cell{ x: 0, y: -2 }],
    [Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: -1, y: 0 }, Cell{ x: -2, y: 0 }],
    [Cell{ x: 0, y: -1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 2 }]
];

const O_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 1, y: 1 }],
    [Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 1, y: 0 }, Cell{ x: 1, y: 1 }],
    [Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 1, y: 1 }],
    [Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 1, y: 0 }, Cell{ x: 1, y: 1 }],
];

const T_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: 0, y: 1 }, Cell{ x: -1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }],
    [Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: -1 }],
    [Cell{ x: 0, y: -1 }, Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: -1, y: 0 }],
    [Cell{ x: -1, y: 0 }, Cell{ x: 0, y: -1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }],
];

const L_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: -1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: -1, y: 1 }],
    [Cell{ x: -1, y: -1 }, Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: -1 }],
    [Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: -1, y: 0 }, Cell{ x: 1, y: -1 }],
    [Cell{ x: 1, y: 1 }, Cell{ x: 0, y: -1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }],
];


const J_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: -1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: 1, y: 1 }],
    [Cell{ x: 1, y: -1 }, Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: -1 }],
    [Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: -1, y: 0 }, Cell{ x: -1, y: -1 }],
    [Cell{ x: -1, y: 1 }, Cell{ x: 0, y: -1 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }],
];

const S_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: -1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: 1, y: 1 }],
    [Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: 1, y: -1 }],
    [Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: -1 }, Cell{ x: -1, y: -1 }],
    [Cell{ x: 0, y: -1 }, Cell{ x: 0, y: 0 }, Cell{ x: -1, y: 0 }, Cell{ x: -1, y: 1 }],
];

const Z_SHAPE: [[Cell; 4]; 4] = [
    [Cell{ x: 1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: 1 }, Cell{ x: -1, y: 1 }],
    [Cell{ x: 0, y: -1 }, Cell{ x: 0, y: 0 }, Cell{ x: 1, y: 0 }, Cell{ x: 1, y: 1 }],
    [Cell{ x: -1, y: 0 }, Cell{ x: 0, y: 0 }, Cell{ x: 0, y: -1 }, Cell{ x: 1, y: -1 }],
    [Cell{ x: 0, y: 1 }, Cell{ x: 0, y: 0 }, Cell{ x: -1, y: 0 }, Cell{ x: -1, y: -1 }],
];

// constants modified from https://github.com/freyhoe/ditzy22/blob/main/common.h
pub const SHAPES: [[[Cell; 4]; 4]; 7] = [I_SHAPE, O_SHAPE, T_SHAPE, L_SHAPE, J_SHAPE, S_SHAPE, Z_SHAPE];

pub fn print_all_shapes() {
    for shape in SHAPES.iter() {
        for piece in shape.iter() {
            print_piece(piece);
        }
    }
}

fn print_piece(piece: &[Cell; 4]) {
    let mut canvas: [[char; 5]; 5] = [[' '; 5]; 5];
    for i in 0..4 {
        // array can include negative numbers; adjust
        canvas[(piece[i].y + 2) as usize][(piece[i].x + 2) as usize] = 'X';
    }

    for row in canvas.iter() {
        println!("{}", row.iter().collect::<String>());
    }
}