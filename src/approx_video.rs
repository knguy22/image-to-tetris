use crate::approx_image;
use crate::cli::{Config, GlobalData};
use crate::utils::{check_command_result, progress_bar};

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use ffmpeg_next::format;
use rayon::prelude::*;

const SOURCE_IMG_DIR: &str = "video_sources";
const APPROX_IMG_DIR: &str = "video_approx";
const AUDIO_PATH: &str = "video_approx/audio.wav";

pub fn run(source: &Path, output: &Path, config: &Config, glob: &GlobalData, video_config: &VideoConfig) -> Result<()> {
    let source_path = source.to_str().expect("failed to convert source path to string");
    let output_path = output.to_str().expect("failed to convert output path to string");

    println!("Approximating video with {}x{} dimensions using {}x{} board", video_config.image_width, video_config.image_height, config.board_width, config.board_height);
    println!("Using {} fps", video_config.fps);

    // use ffmpeg to generate a directory full of images
    // make sure those images correspond to the board dimenisions and blockskin dimensions
    println!("Generating source images from {source_path}...");
    let gen_image_command = Command::new("ffmpeg")
        .arg("-i")
        .arg(source_path)
        .arg("-vf")
        .arg(format!("fps={},scale={}x{}", video_config.fps, video_config.image_width, video_config.image_height))
        .arg("-start_number")
        .arg("0")
        .arg(format!("{SOURCE_IMG_DIR}/%d.png"))
        .output()?;
    check_command_result(&gen_image_command)?;

    // use ffmpeg to generate the audio file
    println!("Generating audio file from {source_path}...");
    let gen_audio_command = Command::new("ffmpeg")
        .arg("-i")
        .arg(source_path)
        .arg(AUDIO_PATH)
        .output()?;
    check_command_result(&gen_audio_command)?;

    // approximate the source images
    let images: Vec<_> = fs::read_dir(SOURCE_IMG_DIR)?
        .collect();
    let pb = progress_bar(images.len())?;
    pb.set_message("Approximating source images...");
    images
        .into_par_iter()
        .for_each(|image| {
            let source_path = image.expect("failed to read source image").path();
            let source_path_without_dir = source_path.file_name().expect("failed to get source image path without directory");
            let approx_path = format!("{}/{}", APPROX_IMG_DIR, source_path_without_dir.to_str().expect("failed to convert source image path to string"));

            let source_img = image::open(source_path).expect("failed to load source image");
            let approx_img = approx_image::approx(&source_img, config, glob).expect("failed to approximate image");
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
        .arg(format!("{APPROX_IMG_DIR}/%d.png"))
        .arg("-i")
        .arg(AUDIO_PATH)
        .arg("-c:v")
        .arg("libx264")
        .arg("-crf")
        .arg("10")
        .arg("-vf")
        .arg(format!("scale={}:{}", video_config.image_width, video_config.image_height))
        .arg("-c:a")
        .arg("aac")
        .arg("-shortest")
        .arg(output_path)
        .output()?;
    check_command_result(&combine_command)?;

    cleanup()?;

    println!("Done!");

    Ok(())
}

pub fn init(source: &Path, output: &Path, config: &Config, glob: &mut GlobalData) -> Result<VideoConfig> {
    ffmpeg_next::init()?;

    // make sure the prerequisite directories exist and are empty
    if Path::new(SOURCE_IMG_DIR).exists() {
        fs::remove_dir_all(SOURCE_IMG_DIR)?;
    }
    if Path::new(APPROX_IMG_DIR).exists() {
        fs::remove_dir_all(APPROX_IMG_DIR)?;
    }
    fs::create_dir(SOURCE_IMG_DIR)?;
    fs::create_dir(APPROX_IMG_DIR)?;

    // make sure the output file is not there
    assert!(!output.exists(), "output file already exists");

    // load config
    let mut video_config = VideoConfig::new(source)?;

    // modify the config based on resized skins
    approx_image::draw::resize_skins(&mut glob.skins, video_config.image_width, video_config.image_height, config.board_width, config.board_height).unwrap();
    video_config.image_width = glob.skin_width() * u32::try_from(config.board_width)?;
    video_config.image_height = glob.skin_height() * u32::try_from(config.board_height)?;

    Ok(video_config)
}

fn cleanup() -> Result<()> {
    fs::remove_dir_all(SOURCE_IMG_DIR)?;
    fs::remove_dir_all(APPROX_IMG_DIR)?;
    Ok(())
}

// contains important video metadata
#[derive(Debug, Clone, Copy)]
pub struct VideoConfig {
    pub image_width: u32,
    pub image_height: u32,
    fps: i32,
}

impl VideoConfig {
    // loads video metadata
    fn new(path: &Path) -> Result<VideoConfig> {
        let source = format::input(path)?;
        let input = source.streams().best(ffmpeg_next::media::Type::Video).expect("failed to find video stream");
        let fps = input.avg_frame_rate();
        let decoder = input.codec().decoder().video()?;

        Ok(VideoConfig {
            image_width: decoder.width(),
            image_height: decoder.height(),
            fps: fps.numerator() / fps.denominator(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx_image::PrioritizeColor;

    #[test]
    #[ignore]
    fn test_run() {
        let source = Path::new("test_videos/blank_video.mkv");
        let output = Path::new("test_results/blank_video.mp4");

        let config = Config {
            board_width: 63,
            board_height: 35,
            prioritize_tetrominos: PrioritizeColor::No,
        };

        let mut glob = GlobalData::new();
        let video_config = init(&source, &output, &config, &mut glob).unwrap();
        run(&source, &output, &config, &glob, &video_config).expect("failed to run video approximator");

        // remove output
        fs::remove_file(&output).unwrap();
    }
}