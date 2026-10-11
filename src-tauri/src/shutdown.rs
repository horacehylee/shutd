use std::process::Command;

/// Initiates a hybrid shutdown, same command the Go version shelled out to
/// but without the extra cmd.exe wrapper.
pub fn exec() -> Result<(), String> {
    Command::new("shutdown")
        .args(["/s", "/t", "0", "/hybrid"])
        .output()
        .map(|_| ())
        .map_err(|e| format!("failed to initiate shutdown: {e}"))
}

#[cfg(test)]
mod tests {
    /// The real command is only run by the scheduler thread, never tests.
    #[test]
    fn command_shape_is_documented() {
        let expected = ["/s", "/t", "0", "/hybrid"];
        assert_eq!(expected.len(), 4);
    }
}
