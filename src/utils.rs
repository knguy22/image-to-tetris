use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("failed to execute command, command line: {0}")]
    CommandError(String),
}

pub fn check_command_result(result: &std::process::Output) -> Result<()> {
    match result.status.code() {
        Some(0) => Ok(()),
        _ => Err(UtilsError::CommandError(String::from_utf8_lossy(&result.stderr).to_string()).into()),
    }
}

pub fn progress_bar(pb_len: usize) -> Result<ProgressBar> {
    let spinner_style = ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")?
        .tick_chars("##-");
    let pb = ProgressBar::new(u64::try_from(pb_len)?);
    pb.set_style(spinner_style.clone());
    Ok(pb)
}

