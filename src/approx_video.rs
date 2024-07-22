use crate::{approx_image, draw};

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use ffmpeg_next::format;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

pub fn run(source: &PathBuf, output: &PathBuf, board_width: usize, board_height: usize) {
    const SOURCE_IMG_DIR: &str = "video_sources";
    const APPROX_IMG_DIR: &str = "video_approx";
    const AUDIO_PATH: &str = "video_approx/audio.wav";

    ffmpeg_next::init().expect("failed to initialize ffmpeg");
    let source_path = source.to_str().expect("failed to convert source path to string");
    let output_path = output.to_str().expect("failed to convert output path to string");

    // check for the prerequisite directories to exist
    if !PathBuf::from(SOURCE_IMG_DIR).exists() {
        fs::create_dir(SOURCE_IMG_DIR).expect("failed to create video_sources directory");
    }
    if !PathBuf::from(APPROX_IMG_DIR).exists() {
        fs::create_dir(APPROX_IMG_DIR).expect("failed to create video_sources directory");
    }

    // make sure the directories are empty; crash if not
    if fs::read_dir(SOURCE_IMG_DIR).unwrap().count() > 0 {
        panic!("video_sources directory is not empty");
    }
    if fs::read_dir(APPROX_IMG_DIR).unwrap().count() > 0 {
        panic!("video_approx directory is not empty");
    }

    // make sure the output file is not there
    if PathBuf::from(output_path).exists() {
        panic!("output file already exists");
    }

    // load config
    let video_config = VideoConfig::new(source).expect("failed to load video config");
    let draw_config = draw::Config {
        board_width: board_width,
        board_height: board_height,
    };
    println!("Approximating video with {}x{} dimensions using {}x{} board", video_config.width, video_config.height, board_width, board_height);
    println!("Using {} fps", video_config.fps);

    // use ffmpeg to generate a directory full of images
    println!("Generating source images from {}...", source_path);
    let gen_image_command = Command::new("ffmpeg")
        .arg("-i")
        .arg(source_path)
        .arg("-vf")
        .arg(format!("fps={}", video_config.fps))
        .arg("-start_number")
        .arg("0")
        .arg(format!("{}/%d.png", SOURCE_IMG_DIR))
        .output()
        .expect("failed to create source images");
    if !gen_image_command.status.success() {
        println!("ffmpeg error: {:?}", gen_image_command);
        panic!("failed to generate source images");
    }

    // use ffmpeg to generate the audio file
    println!("Generating audio file from {}...", source_path);
    let gen_audio_command = Command::new("ffmpeg")
        .arg("-i")
        .arg(source_path)
        .arg(AUDIO_PATH)
        .output()
        .expect("failed to create audio file");
    if !gen_audio_command.status.success() {
        panic!("failed to generate source images");
    }

    // approximate the source images
    let images: Vec<_> = fs::read_dir(SOURCE_IMG_DIR).expect("failed to read source images directory")
        .into_iter()
        .collect();
    let pb = progress_bar(images.len());
    pb.set_message("Approximating source images...");
    images
        .into_par_iter()
        .for_each(|image| {
            let source_path = image.expect("failed to read source image").path();
            let source_path_without_dir = source_path.file_name().expect("failed to get source image path without directory");
            let approx_path = format!("{}/{}", APPROX_IMG_DIR, source_path_without_dir.to_str().expect("failed to convert source image path to string"));

            let mut source_img = image::open(source_path).expect("failed to load source image");
            let approx_img = approx_image::approximate(&mut source_img, &draw_config).expect("failed to approximate image");
            approx_img.save(approx_path).expect("failed to save approx image");

            // make sure the progress bar is updated
            pb.inc(1);
        });
    pb.finish_with_message("Done approximating source images!");

    // combine the approximated images and audio for a final video
    println!("Combining approximated images and audio...");
    let combine_command = Command::new("ffmpeg")
        .arg("-framerate")
        .arg(format!("{}", video_config.fps))
        .arg("-i")
        .arg(format!("{}/%d.png", APPROX_IMG_DIR))
        .arg("-i")
        .arg(AUDIO_PATH)
        .arg("-c:v")
        .arg("libx264")
        .arg("-crf")
        .arg("10")
        .arg("-vf")
        .arg(format!("scale={}:{}", video_config.width, video_config.height))
        .arg("-c:a")
        .arg("aac")
        .arg("-shortest")
        .arg(output_path)
        .output()
        .expect("failed to combine images and audio");
    if !combine_command.status.success() {
        println!("ffmpeg error: {:?}", combine_command);
        panic!("failed to combine images and audio");
    }

    // clean up the directories
    fs::remove_dir_all(SOURCE_IMG_DIR).expect("failed to remove source images directory");
    fs::remove_dir_all(APPROX_IMG_DIR).expect("failed to remove approximated images directory");

    println!("Done!");
}

fn progress_bar(pb_len: usize) -> ProgressBar {
    let spinner_style = ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .tick_chars("##-");
    let pb = ProgressBar::new(u64::try_from(pb_len).expect("failed to convert usize to u64"));
    pb.set_style(spinner_style.clone());
    pb
}

// contains important video metadata
#[derive(Debug, Clone, Copy)]
struct VideoConfig {
    width: u32,
    height: u32,
    fps: i32,
}

impl VideoConfig {
    // loads video metadata
    fn new(path: &PathBuf) -> Result<VideoConfig, Box<dyn std::error::Error>> {
        let source = format::input(path)?;
        let input = source.streams().best(ffmpeg_next::media::Type::Video).ok_or("failed to find video stream")?;
        let fps = input.avg_frame_rate();
        let decoder = input.codec().decoder().video().expect("failed to create decoder");

        Ok(VideoConfig {
            width: decoder.width(),
            height: decoder.height(),
            fps: fps.numerator() / fps.denominator(),
        })
    }
}
