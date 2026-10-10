//! Bounded subprocess bridge to Apple's on-device Foundation Models framework.
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

pub(crate) fn invoke(
    helper: &Path,
    input: Option<Value>,
    timeout: Duration,
) -> Result<Value, String> {
    let mut command = Command::new(helper);
    if input.is_none() {
        command.arg("--check");
    }
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|_| "Apple Intelligence helper could not start. Requires macOS 26 or later; check Apple Intelligence in System Settings or reinstall Tuck.")?;
    let stdout = child.stdout.take().ok_or("Missing Local model output")?;
    // Concurrent pipe handling keeps the deadline effective even if the helper stalls.
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(128 * 1024)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let mut stdin = child.stdin.take().ok_or("Missing Local model input")?;
    let writer = std::thread::spawn(move || {
        if let Some(input) = input {
            stdin.write_all(input.to_string().as_bytes())?;
        }
        Ok::<_, std::io::Error>(())
    });
    let status = child.wait_timeout(timeout);
    let complete = matches!(&status, Ok(Some(_)));
    if !complete {
        let _ = child.kill();
        let _ = child.wait();
    }
    let written = writer.join().map_err(|_| "Local model input failed")?;
    let output = reader
        .join()
        .map_err(|_| "Local model output failed")?
        .map_err(|_| "Local model output failed")?;
    if !complete {
        return Err("The local model reached its time limit or stopped unexpectedly".into());
    }
    written.map_err(|_| "Local model could not read the request")?;
    if !status.unwrap().unwrap().success() {
        return Err("The local model helper stopped unexpectedly".into());
    }
    let value: Value =
        serde_json::from_slice(&output).map_err(|_| "Invalid response from the local model")?;
    if let Some(error) = value["error"].as_str() {
        return Err(error.chars().take(400).collect());
    }
    Ok(value)
}

#[cfg(test)]
pub fn check(helper: &Path) -> Result<String, String> {
    invoke(helper, None, Duration::from_secs(15))?["status"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "Invalid Apple availability response".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stalled_and_invalid_helpers_fail_safely() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("fake-helper");
        std::fs::write(&helper, "#!/bin/sh\nexec /bin/sleep 5\n").unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let start = std::time::Instant::now();
        let error = invoke(
            &helper,
            Some(json!({"input":"x".repeat(200_000)})),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert!(error.contains("time limit"));
        assert!(start.elapsed() < Duration::from_secs(3));
        std::fs::write(&helper, "#!/bin/sh\nprintf 'invalid JSON'\n").unwrap();
        assert!(check(&helper).unwrap_err().contains("Invalid response"));
        std::fs::write(
            &helper,
            r#"#!/bin/sh
printf '{"error":"Enable Apple Intelligence in System Settings"}'
"#,
        )
        .unwrap();
        assert!(check(&helper).unwrap_err().contains("System Settings"));
    }
    #[test]
    fn missing_helper_returns_actionable_error() {
        assert!(check(Path::new("/no-such-tuck-helper"))
            .unwrap_err()
            .contains("System Settings"));
    }
}
