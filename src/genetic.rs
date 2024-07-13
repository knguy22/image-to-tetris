use crate::board::Board;
use crate::piece::{Piece, Cell, Orientation};
use crate::draw::{draw_board, BlockSkin};

use imageproc::image::DynamicImage;
use image_compare::{self, CompareError, rgb_hybrid_compare};
use rand::{self, Rng};
use rand::distributions::Distribution;

pub struct Config {
    pub max_iterations: usize,
    pub population_size: usize,
    pub max_breed_attempts: usize,
    pub skin: BlockSkin,
    pub board_width: usize,
    pub board_height: usize,
}

#[derive(Clone)]
struct Individual {
    board: Board,
    score: f64,
}

pub fn genetic_algorithm(target_img: &DynamicImage, config: Config) -> DynamicImage {
    let mut best_image: DynamicImage = Default::default();
    let mut rng = rand::thread_rng();

    // initial population
    println!("Initial population size: {}", config.population_size);
    let mut population: Vec<Individual> = Vec::new();

    let mut attempts = 0;
    while population.len() < config.population_size {
        assert!(attempts < config.max_breed_attempts);

        let board = Board::new(config.board_width, config.board_height);
        let individual = Individual::new(board, &config, target_img);
        match individual {
            Ok(individual) => population.push(individual),
            Err(e) => {println!("Error: {}", e);}
        }
        attempts += 1;
    }
    println!("Initial population score: {}", population[0].score);

    let num_survive= config.population_size / 10;
    for i in 0..config.max_iterations {
        // sort the population based on fitness score and find the best
        population.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let best = &population[0];
        if best.score > 0.0 {
            best_image = draw_board(&best.board, &config.skin);
        }
        println!("Iteration: {}, Best score: {}, Population size: {}", i, best.score, population.len());

        // let the best 10% survive
        let mut new_population: Vec<Individual> = Vec::new();
        for i in 0..num_survive {
            new_population.push(population[i].clone());
        }

        // let the best 50% breed randomly to fill up the rest of the next generation
        let mut attempts = 0;
        while new_population.len() < config.population_size && attempts < config.max_breed_attempts {
            attempts += 1;

            let parent1 = &population[rng.gen_range(0..num_survive)];
            let parent2 = &population[rng.gen_range(0..num_survive)];
            let breed_res = breed(&parent1.board, &parent2.board);
            let child = match breed_res {
                Ok(child) => child,
                Err(_) => continue
            };
            match Individual::new(child, &config, target_img) {
                Ok(individual) => new_population.push(individual),
                Err(_) => {}
            }
        }

        // replace the old population
        population = new_population;
    }

    best_image
}

fn breed(board1: &Board, board2: &Board) -> Result<Board, Box<dyn std::error::Error>> {
    let mut board = Board::new(board1.width, board1.height);

    // share genetics
    // place pieces if possible in order from both boards choosing randomly using 2 pointers
    // do this until all pieces are parsed
    let mut i = 0;
    let mut j = 0;
    while i < board1.pieces.len() && j < board2.pieces.len() {
        let piece1 = &board1.pieces[i];
        let piece2 = &board2.pieces[j];
        if rand::random() {
            let _ = board.place(piece1);
            i += 1;
        } else {
            let _ = board.place(piece2);
            j += 1;
        }
    }
    while i < board1.pieces.len() {
        let _ = board.place(&board1.pieces[i]);
        i += 1;
    }
    while j < board2.pieces.len() {
        let _ = board.place(&board2.pieces[j]);
        j += 1;
    }

    match mutate(&mut board) {
        Ok(_) => Ok(board),
        Err(e) => Err(e)
    }
}

// mutations
fn mutate(board: &mut Board) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    let delete_piece: bool = rng.gen() && board.pieces.len() > 0;

    // delete a piece
    if delete_piece {
        let piece_between = rand::distributions::Uniform::from(0..board.pieces.len());
        let piece = board.pieces[piece_between.sample(&mut rng)].clone();
        board.remove_piece(&piece).expect(format!("Failed to remove piece {:?}", piece).as_str());
        return Ok(());
    }

    // add a piece
    let piece_between = rand::distributions::Uniform::from(0..7);
    let orientation_between = rand::distributions::Uniform::from(0..4);
    let x_between = rand::distributions::Uniform::from(0..board.width);
    let y_between = rand::distributions::Uniform::from(0..board.height);

    let cell = Cell{
        x: x_between.sample(&mut rng),
        y: y_between.sample(&mut rng)
    };

    let orientation = match orientation_between.sample(&mut rng) {
        0 => Orientation::NORTH,
        1 => Orientation::EAST,
        2 => Orientation::SOUTH,
        3 => Orientation::WEST,
        _ => unreachable!()
    };

    let piece = match piece_between.sample(&mut rng) {
        0 => Piece::I(cell, orientation),
        1 => Piece::J(cell, orientation),
        2 => Piece::L(cell, orientation),
        3 => Piece::O(cell, orientation),
        4 => Piece::S(cell, orientation),
        5 => Piece::T(cell, orientation),
        6 => Piece::Z(cell, orientation),
        _ => unreachable!()
    };

    if board.place(&piece).is_ok() {
        return Ok(());
    }
    Err("Failed to generate new board".into())
}

pub fn score(image1: &DynamicImage, image2: &DynamicImage) -> Result<f64, CompareError> {
    Ok(rgb_hybrid_compare(&image1.clone().to_rgb8(), &image2.clone().into_rgb8())?.score)
}

impl Individual {
    pub fn new(board: Board, config: &Config, target_img: &DynamicImage) -> Result<Individual, Box<dyn std::error::Error>> {
        let score = score(&draw_board(&board, &config.skin), target_img)?;
        Ok(Individual { board, score })
    }
}