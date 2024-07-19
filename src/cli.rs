use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    // the image to be approximated
    #[clap(long, short, action)]
    pub source_img: Option<PathBuf>,

    // the output image
    #[clap(long, short, action)]
    pub output_img: Option<PathBuf>,

    // board width
    #[clap(long, action)]
    pub width: Option<usize>,

    // board height
    #[clap(long, action)]
    pub height: Option<usize>,

    // run integration tests
    #[clap(long, action)]
    pub integration_tests: bool,
}

