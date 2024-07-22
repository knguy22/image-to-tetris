use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// number of threads to use; default is 4
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// flag for whether to prioritize tetrominos or not; increases image color but reduces accuracy
    # [arg(short, long, default_value_t = false)]
    pub prioritize_tetrominos: bool,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    /// runs approximation tests using images located in the `sources` directory
    Integration,

    /// approximates a single image using tetris blocks
    ApproxImage{source: PathBuf, output: PathBuf, width: usize, height: usize},

    /// approximates a single video using tetris blocks
    ApproxVideo{source: PathBuf, output: PathBuf, width: usize, height: usize},
}