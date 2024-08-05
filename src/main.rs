mod approx_image;
mod approx_audio;
mod approx_video;
mod cli;
mod utils;

use approx_image::{Config, PrioritizeColor};
use approx_image::draw::create_skins;
use approx_image::integration_test;
use std::path::PathBuf;

use clap::Parser;
use imageproc::image;
use rayon;

fn main() {
    let cli = cli::Cli::parse();

    let threads = cli.threads.unwrap_or(4);
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().expect("failed to build thread pool");
    println!("Using {} threads", threads);

    let prioritize_tetrominos = match cli.prioritize_tetrominos {
        true => PrioritizeColor::Yes,
        false => PrioritizeColor::No,
    };
    println!("Prioritizing tetrominos: {}", cli.prioritize_tetrominos);

    // a global skins will be copied by each thread to prevent needing IO to recreate skins for each thread
    let skins = create_skins();

    match cli.command {
        cli::Commands::Integration {board_width} => {
            let config = Config {
                board_width: board_width.unwrap_or(100),
                board_height: 0, // height doesn't matter here since it will be auto-scaled
                prioritize_tetrominos,
                skins: &skins,
            };
            integration_test::run("sources", &config).expect("failed to run integration test");
        },
        cli::Commands::ApproxImage { source, output, board_width, board_height } => {
            let config = Config {
                board_width,
                board_height,
                prioritize_tetrominos,
                skins: &skins,
            };
            run_approx_image(&source, &output, &config)
        }
        cli::Commands::ApproxAudio { source, output } => {
            approx_audio::run(&source, &output).expect("failed to run approximation audio");
        }
        cli::Commands::ApproxVideo { source, output, board_width, board_height } => {
            let config = Config {
                board_width,
                board_height,
                prioritize_tetrominos,
                skins: &skins,
            };
            approx_video::run(&source, &output, &config).expect("failed to run approximation video");
        }
    }
}

fn run_approx_image(source: &PathBuf, output: &PathBuf, config: &Config) {
    println!("Approximating an image: {}", source.display());

    let mut source_img = image::open(source).expect("could not load source image");
    println!("Loaded {}x{} image", source_img.width(), source_img.height());

    let result_img = approx_image::run(&mut source_img, &config).expect("could not approximate image");
    result_img.save(output).expect("could not save output image");
}
