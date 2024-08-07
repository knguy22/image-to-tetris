mod approx_image;
mod approx_audio;
mod approx_video;
mod cli;
mod utils;

use approx_image::PrioritizeColor;
use approx_image::integration_test;
use cli::{Config, GlobalData};

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    let threads = cli.threads.unwrap_or(4);
    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().expect("failed to build thread pool");
    println!("Using {threads} threads");

    let prioritize_tetrominos = if cli.prioritize_tetrominos {PrioritizeColor::Yes} else {PrioritizeColor::No};
    println!("Prioritizing tetrominos: {}", cli.prioritize_tetrominos);

    // a global skins will be copied by each thread to prevent needing IO to recreate skins for each thread
    let mut glob = GlobalData::new();

    match cli.command {
        cli::Commands::Integration {board_width} => {
            let config = Config {
                board_width: board_width.unwrap_or(100),
                board_height: 0, // height doesn't matter here since it will be auto-scaled
                prioritize_tetrominos,
                approx_audio: false,
            };
            integration_test::run("sources", &config, &glob).expect("failed to run integration test");
        },
        cli::Commands::ApproxImage { source, output, board_width, board_height } => {
            let config = Config {
                board_width,
                board_height,
                prioritize_tetrominos,
                approx_audio: false,
            };
            approx_image::run(&source, &output, &config, &mut glob);
        }
        cli::Commands::ApproxAudio { source, output } => {
            approx_audio::run(&source, &output).expect("failed to run approximation audio");
        }
        cli::Commands::ApproxVideo { source, output, board_width, board_height} => {
            let config = Config {
                board_width,
                board_height,
                prioritize_tetrominos,
                approx_audio: cli.approx_audio,
            };
            let video_config = approx_video::init(&source, &output, &config, &mut glob).unwrap();
            approx_video::run(&source, &output, &config, &glob, &video_config).expect("failed to run approximation video");
        }
    }
}
