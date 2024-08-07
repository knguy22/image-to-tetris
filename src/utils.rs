pub fn check_command_result(result: &std::process::Output) -> Result<(), Box<dyn std::error::Error>> {
    match result.status.code() {
        Some(0) => Ok(()),
        _ => Err(format!("failed to execute command, command line: {result:?}").into()),
    }
}
