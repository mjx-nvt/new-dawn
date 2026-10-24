//! DOCX → PDF with an app already on the Mac: Microsoft Word → LibreOffice → Pages, the first
//! that works (D-024). Each conversion runs in a fresh folder and is killed after 60 s.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::process::Command;

use crate::error::AppError;

const TIMEOUT: Duration = Duration::from_secs(60);

/// One conversion at a time: two scripts driving Word at once could save or close each other's
/// document (or the user's).
static CONVERTING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Converter {
    Word,
    Libreoffice,
    Pages,
}

impl Converter {
    /// Preference order.
    const ALL: [Converter; 3] = [Converter::Word, Converter::Libreoffice, Converter::Pages];

    fn name(self) -> &'static str {
        match self {
            Converter::Word => "Microsoft Word",
            Converter::Libreoffice => "LibreOffice",
            Converter::Pages => "Pages",
        }
    }

    fn bundle(self) -> &'static str {
        match self {
            Converter::Word => "Microsoft Word.app",
            Converter::Libreoffice => "LibreOffice.app",
            Converter::Pages => "Pages.app",
        }
    }

    /// Sandboxed apps read files in their own container without a "grant access" prompt.
    fn container(self) -> Option<&'static str> {
        match self {
            Converter::Word => Some("com.microsoft.Word"),
            Converter::Pages => Some("com.apple.iWork.Pages"),
            Converter::Libreoffice => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ConverterInfo {
    pub converter: Converter,
    pub name: String,
    /// Where the app was found; `None` when it isn't installed.
    pub path: Option<String>,
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

fn find_app(bundle: &str) -> Option<PathBuf> {
    [PathBuf::from("/Applications"), home().join("Applications")]
        .into_iter()
        .map(|dir| dir.join(bundle))
        .find(|path| path.is_dir())
}

/// Every converter in preference order, with where it was found.
pub fn list() -> Vec<ConverterInfo> {
    Converter::ALL
        .iter()
        .map(|&converter| ConverterInfo {
            converter,
            name: converter.name().to_owned(),
            path: find_app(converter.bundle()).map(|p| p.display().to_string()),
        })
        .collect()
}

/// Writes `pdf` from `docx` with the first converter that succeeds; returns which one.
pub async fn convert(docx: &Path, pdf: &Path) -> Result<Converter> {
    let _one_at_a_time = CONVERTING.lock().await;
    let mut failures = Vec::new();
    for info in list() {
        let Some(app) = info.path else { continue };
        match convert_with(info.converter, Path::new(&app), docx, pdf).await {
            Ok(()) => return Ok(info.converter),
            Err(err) => {
                tracing::warn!("{} could not convert: {err:#}", info.name);
                failures.push(format!("{}: {err:#}", info.name));
            }
        }
    }
    if failures.is_empty() {
        bail!(AppError::new(
            "no_converter",
            "No PDF converter found. Install Microsoft Word, LibreOffice or Pages."
        ));
    }
    bail!(AppError::new(
        "pdf_failed",
        format!("Could not make the PDF. {}", failures.join(" · "))
    ))
}

async fn convert_with(converter: Converter, app: &Path, docx: &Path, pdf: &Path) -> Result<()> {
    let base = converter
        .container()
        .map(|id| home().join("Library/Containers").join(id).join("Data/Documents"))
        .filter(|dir| dir.is_dir()) // the app has run at least once
        .unwrap_or_else(std::env::temp_dir);
    let work = base.join(format!("NewDawn-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&work).with_context(|| format!("creating {}", work.display()))?;
    let result = async {
        // A unique name: the Word script finds its document by name, never "the active one".
        let name = format!("minutes-{}", uuid::Uuid::now_v7());
        let input = work.join(format!("{name}.docx"));
        let output = work.join(format!("{name}.pdf"));
        std::fs::copy(docx, &input).context("copying the DOCX")?;
        let command = match converter {
            Converter::Word => applescript(WORD_SCRIPT, &input, &output),
            Converter::Pages => applescript(PAGES_SCRIPT, &input, &output),
            Converter::Libreoffice => {
                let mut cmd = Command::new(app.join("Contents/MacOS/soffice"));
                // A private profile, so a LibreOffice window already open doesn't swallow the job.
                // ponytail: the profile URL isn't percent-encoded; the temp dir has no spaces.
                cmd.arg(format!(
                    "-env:UserInstallation=file://{}",
                    work.join("profile").display()
                ))
                .args(["--headless", "--convert-to", "pdf", "--outdir"])
                .arg(&work)
                .arg(&input);
                cmd
            }
        };
        run(command).await?;
        if !output.is_file() {
            bail!("no PDF was written");
        }
        // Copy beside the target, then rename: a failure never leaves half a PDF in place.
        let partial = pdf.with_extension("pdf.tmp");
        std::fs::copy(&output, &partial).context("saving the PDF")?;
        std::fs::rename(&partial, pdf).context("saving the PDF")?;
        Ok(())
    }
    .await;
    if let Err(err) = std::fs::remove_dir_all(&work) {
        tracing::warn!("could not remove {}: {err}", work.display());
    }
    result
}

/// Paths travel as arguments (`argv`: input, output, input file name), never inside the script.
fn applescript(script: &str, input: &Path, output: &Path) -> Command {
    let mut cmd = Command::new("/usr/bin/osascript");
    cmd.arg("-e")
        .arg(script)
        .arg(input)
        .arg(output)
        .arg(input.file_name().unwrap_or_default());
    cmd
}

/// Quits Word afterwards only if this conversion launched it.
const WORD_SCRIPT: &str = r#"on run argv
    set wasRunning to application "Microsoft Word" is running
    with timeout of 55 seconds
        tell application "Microsoft Word"
            open file name (item 1 of argv)
            set d to document (item 3 of argv)
            save as d file name (item 2 of argv) file format format PDF
            close d saving no
            if not wasRunning then quit saving no
        end tell
    end timeout
end run"#;

const PAGES_SCRIPT: &str = r#"on run argv
    set wasRunning to application "Pages" is running
    with timeout of 55 seconds
        tell application "Pages"
            set d to open (POSIX file (item 1 of argv))
            export d to (POSIX file (item 2 of argv)) as PDF
            close d saving no
            if not wasRunning then quit
        end tell
    end timeout
end run"#;

async fn run(mut command: Command) -> Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    const HINT: &str = "is it showing a dialog?";
    let output = tokio::time::timeout(TIMEOUT, command.output())
        .await
        .map_err(|_| anyhow!("didn't finish in {} s — {HINT}", TIMEOUT.as_secs()))?
        .context("starting the converter")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // -1712: the app didn't answer the Apple Event in time (usually a modal dialog).
        if stderr.contains("(-1712)") {
            bail!("didn't answer — {HINT}");
        }
        bail!("exited with {}: {}", output.status, stderr.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_converters_in_preference_order() {
        let found: Vec<Converter> = list().iter().map(|c| c.converter).collect();
        assert_eq!(found, Converter::ALL);
    }

    /// A real conversion with whatever is installed
    /// (`cargo test converts_the_vifc_sample -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn converts_the_vifc_sample() {
        let docx = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../brands/vifc/templates/meeting-minutes/original.docx");
        let pdf = std::env::temp_dir().join(format!("newdawn-pdf-{}.pdf", uuid::Uuid::now_v7()));
        let started = std::time::Instant::now();
        let used = tauri::async_runtime::block_on(convert(&docx, &pdf)).unwrap();
        eprintln!("{used:?} in {:?}", started.elapsed());
        let head = std::fs::read(&pdf).unwrap();
        assert!(head.starts_with(b"%PDF"));
        std::fs::remove_file(pdf).unwrap();
    }
}
