use crate::approx_image;
use crate::approx_audio;
use crate::cli::{Config, GlobalData};
use crate::utils::check_command_result;

use std::fs;
use std::path::Path;
use std::process::Command;

use ffmpeg_next::format;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

const SOURCE_IMG_DIR: &str = "video_sources";
const APPROX_IMG_DIR: &str = "video_approx";
const AUDIO_PATH: &str = "video_approx/audio.wav";

pub fn run(source: &Path, output: &Path, config: &Config, glob: &GlobalData, video_config: &VideoConfig) -> Result<(), Box<dyn std::error::Error>> {
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

    // approximate the audio file if wanted
    if video_config.approx_audio {
        approx_audio::run(Path::new(AUDIO_PATH), Path::new(AUDIO_PATH))?;
    } 
    else {
        println!("Skipping audio approximation");
    }

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

pub fn init(source: &Path, output: &Path, config: &Config, glob: &mut GlobalData) -> Result<VideoConfig, Box<dyn std::error::Error>> {
    ffmpeg_next::init()?;

    // check for the prerequisite directories to exist
    if !Path::new(SOURCE_IMG_DIR).exists() {
        fs::create_dir(SOURCE_IMG_DIR)?;
    }
    if !Path::new(APPROX_IMG_DIR).exists() {
        fs::create_dir(APPROX_IMG_DIR)?;
    }

    // make sure the directories are empty; crash if not
    if fs::read_dir(SOURCE_IMG_DIR)?.count() > 0 {
        return Err("video_sources directory is not empty".into());
    }
    if fs::read_dir(APPROX_IMG_DIR)?.count() > 0 {
        return Err("video_approx directory is not empty".into());
    }

    // make sure the output file is not there
    if output.exists() {
        return Err("output file already exists".into());
    }

    // load config
    let mut video_config = VideoConfig::new(source, config)?;

    // modify the config based on resized skins
    approx_image::draw::resize_skins(&mut glob.skins, video_config.image_width, video_config.image_height, config.board_width, config.board_height).unwrap();
    video_config.image_width = glob.skin_width() * u32::try_from(config.board_width)?;
    video_config.image_height = glob.skin_height() * u32::try_from(config.board_height)?;

    Ok(video_config)
}

fn cleanup() -> Result<(), Box<dyn std::error::Error>> {
    fs::remove_dir_all(SOURCE_IMG_DIR)?;
    fs::remove_dir_all(APPROX_IMG_DIR)?;
    Ok(())
}

fn progress_bar(pb_len: usize) -> Result<ProgressBar, Box<dyn std::error::Error>> {
    let spinner_style = ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")?
        .tick_chars("##-");
    let pb = ProgressBar::new(u64::try_from(pb_len)?);
    pb.set_style(spinner_style.clone());
    Ok(pb)
}

// contains important video metadata
#[derive(Debug, Clone, Copy)]
pub struct VideoConfig {
    pub image_width: u32,
    pub image_height: u32,
    fps: i32,
    approx_audio: bool,
}

impl VideoConfig {
    // loads video metadata
    fn new(path: &Path, config: &Config) -> Result<VideoConfig, Box<dyn std::error::Error>> {
        let source = format::input(path)?;
        let input = source.streams().best(ffmpeg_next::media::Type::Video).ok_or("failed to find video stream")?;
        let fps = input.avg_frame_rate();
        let decoder = input.codec().decoder().video()?;

        Ok(VideoConfig {
            image_width: decoder.width(),
            image_height: decoder.height(),
            fps: fps.numerator() / fps.denominator(),
            approx_audio: config.approx_audio,
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
            approx_audio: false,
        };

        let mut glob = GlobalData::new();
        let video_config = init(&source, &output, &config, &mut glob).unwrap();
        run(&source, &output, &config, &glob, &video_config).expect("failed to run video approximator");

        // remove output
        fs::remove_file(&output).unwrap();
    }
}