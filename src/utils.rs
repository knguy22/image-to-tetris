use indicatif::{ProgressBar, ProgressStyle};

pub fn check_command_result(result: &std::process::Output) -> Result<(), Box<dyn std::error::Error>> {
    match result.status.code() {
        Some(0) => Ok(()),
        _ => Err(format!("failed to execute command, command line: {result:?}").into()),
    }
}

pub fn progress_bar(pb_len: usize) -> Result<ProgressBar, Box<dyn std::error::Error>> {
    let spinner_style = ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")?
        .tick_chars("##-");
    let pb = ProgressBar::new(u64::try_from(pb_len)?);
    pb.set_style(spinner_style.clone());
    Ok(pb)
}

