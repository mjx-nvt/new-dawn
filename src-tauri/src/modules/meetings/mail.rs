//! A new message in Apple Mail with the cover note and the minutes attached. Apple Events via
//! `osascript`, with the subject, body and file path passed as arguments (never inside the
//! script), like the PDF converters. Local only: Mail is the one that sends.

use std::process::Stdio;

use anyhow::{Context, Result, bail};
use tokio::process::Command;

const SCRIPT: &str = r#"on run argv
    with timeout of 30 seconds
        tell application "Mail"
            set m to make new outgoing message with properties {subject:(item 1 of argv), content:(item 2 of argv), visible:true}
            if (count of argv) > 2 then
                tell m to make new attachment with properties {file name:(POSIX file (item 3 of argv))} at after the last paragraph
            end if
            activate
        end tell
    end timeout
end run"#;

pub async fn compose(subject: &str, body: &str, attachment: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("/usr/bin/osascript");
    cmd.arg("-e").arg(SCRIPT).arg(subject).arg(body);
    if let Some(path) = attachment {
        cmd.arg(path);
    }
    let output = cmd
        .stdin(Stdio::null())
        .output()
        .await
        .context("running osascript")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(())
}
