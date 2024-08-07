use crate::approx_image::PrioritizeColor;
use crate::approx_image::draw::{Skins, create_skins};

use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Clone)]
pub struct GlobalData {
    pub skins: Skins,
}

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub board_width: usize,
    pub board_height: usize,
    pub prioritize_tetrominos: PrioritizeColor,
    pub approx_audio: bool,
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// number of threads to use; default is 4
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// flag for whether to prioritize tetrominos or not; increases image color but reduces accuracy
    # [arg(short, long, default_value_t = false)]
    pub prioritize_tetrominos: bool,

    /// flag for whether to approximate audio or not, only used with video
    #[arg(short, long, default_value_t = false)]
    pub approx_audio: bool,

    #[command(subcommand)]
    pub command: Commands
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// runs approximation tests using images located in the `sources` directory; `board_width` is set to 100 if unspecified
    Integration{board_width: Option<usize>},

    /// approximates a single image using tetris blocks
    ApproxImage{source: PathBuf, output: PathBuf, board_width: usize, board_height: usize},

    /// approximates a single audio file using tetris sound clips
    ApproxAudio{source: PathBuf, output: PathBuf},

    /// approximates a single video using tetris blocks
    ApproxVideo{source: PathBuf, output: PathBuf, board_width: usize, board_height: usize},
}

impl GlobalData {
    pub fn new() -> GlobalData {
        GlobalData {
            skins: create_skins(),
        }
    }

    pub fn skin_width(&self) -> u32 {
        self.skins[0].width()
    }
    pub fn skin_height(&self) -> u32 {
        self.skins[0].height()
    }
}