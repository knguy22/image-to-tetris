use crate::utils::check_command_result;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

// resamples the audio to the specified sample rate using ffmpeg
pub fn run(source: &PathBuf, output: &PathBuf, sample_rate: f64) -> Result<(), Box<dyn std::error::Error>> {
    // use .wav for output
    let output = output.with_extension("wav");

    // replace the file
    if output.exists() {
        fs::remove_file(output.clone())?;
    }

    let gen_audio_command = Command::new("ffmpeg")
        .arg("-i")
        .arg(source)
        .arg("-ar")
        .arg(sample_rate.to_string())
        .arg(output)
        .output()?;
    check_command_result(gen_audio_command)?;
    Ok(())
}

// the same as run but for an entire directory
pub fn run_dir(source: &PathBuf, output: &PathBuf, sample_rate: f64) -> Result<(), Box<dyn std::error::Error>> {
    // makes sure the output directory exists
    if !output.exists() {
        fs::create_dir_all(output)?;
    }

    for path in source.read_dir()? {
        let source_path = path.expect("failed to read source image").path();
        let file_name = source_path.file_name().expect("failed to get source image path without directory");
        let output_path = output.join(file_name);
        run(&source_path, &output_path, sample_rate)?;
    }
    Ok(())
}