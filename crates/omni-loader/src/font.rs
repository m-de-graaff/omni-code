//! Nerd Font auto-installer.
//!
//! On first launch, checks whether a Nerd Font is available. If not,
//! downloads JetBrainsMono Nerd Font from GitHub, installs it to the
//! user font directory, and configures Windows Terminal to use it.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use thiserror::Error;

/// The Nerd Font family we install.
pub const FONT_FAMILY: &str = "JetBrainsMono Nerd Font";

/// Nerd Font version to download.
const NERD_FONT_VERSION: &str = "v3.3.0";

/// GitHub download URL template.
const DOWNLOAD_URL: &str = "https://github.com/ryanoasis/nerd-fonts/releases/download/{VERSION}/JetBrainsMono.zip";

/// Marker file written after successful install.
const MARKER_FILE: &str = "nerd-font-installed";

#[derive(Debug, Error)]
pub enum FontError {
    #[error("failed to resolve data directory: {0}")]
    Path(#[from] super::paths::PathError),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP extraction failed: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("font directory not found")]
    NoFontDir,
}

/// Check whether the Nerd Font has already been installed by us.
///
/// We use a marker file in the data directory rather than scanning system
/// fonts, which avoids platform-specific font enumeration.
pub fn is_installed() -> bool {
    marker_path().is_ok_and(|p| p.exists())
}

/// Run the full install flow: download, extract, install, configure terminal.
///
/// This is idempotent — if the marker file exists it returns immediately.
///
/// # Errors
/// Returns `FontError` if any step fails.
pub async fn ensure_installed() -> Result<(), FontError> {
    if is_installed() {
        tracing::debug!("Nerd Font already installed (marker exists)");
        return Ok(());
    }

    tracing::info!("Nerd Font not found — installing {FONT_FAMILY} {NERD_FONT_VERSION}");

    // 1. Download the zip
    let zip_bytes = download_font_zip().await?;
    tracing::info!(bytes = zip_bytes.len(), "downloaded font archive");

    // 2. Extract .ttf files to the user font directory
    let font_dir = user_font_dir()?;
    std::fs::create_dir_all(&font_dir)?;
    let installed = extract_ttf_files(&zip_bytes, &font_dir)?;
    tracing::info!(count = installed, dir = %font_dir.display(), "installed font files");

    // 3. Register fonts on Windows
    #[cfg(target_os = "windows")]
    register_fonts_windows(&font_dir)?;

    // 4. Configure Windows Terminal
    #[cfg(target_os = "windows")]
    if let Err(e) = configure_windows_terminal() {
        tracing::warn!(?e, "could not auto-configure Windows Terminal font");
    }

    // 5. Write marker
    let marker = marker_path()?;
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&marker, NERD_FONT_VERSION)?;
    tracing::info!("Nerd Font installation complete");

    Ok(())
}

/// Download the Nerd Font zip archive from GitHub.
async fn download_font_zip() -> Result<Vec<u8>, FontError> {
    let url = DOWNLOAD_URL.replace("{VERSION}", NERD_FONT_VERSION);
    let response = reqwest::get(&url).await?.error_for_status()?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

/// Extract all `.ttf` files from a zip archive into `dest_dir`.
///
/// Returns the number of files extracted.
fn extract_ttf_files(zip_bytes: &[u8], dest_dir: &Path) -> Result<usize, FontError> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;
    let mut count = 0;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        // Only extract regular .ttf files (skip variable-weight fonts to save space)
        if !name.ends_with(".ttf") || name.contains("NerdFontPropo") {
            continue;
        }

        let file_name = Path::new(&name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());

        let dest = dest_dir.join(&file_name);

        // Skip if already exists (from a previous partial install)
        if dest.exists() {
            count += 1;
            continue;
        }

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        std::fs::write(&dest, &buf)?;
        tracing::debug!(file = %file_name, "extracted font file");
        count += 1;
    }

    Ok(count)
}

/// Path to the install marker file.
fn marker_path() -> Result<PathBuf, FontError> {
    Ok(super::paths::data_dir()?.join(MARKER_FILE))
}

/// User font directory (per-platform).
fn user_font_dir() -> Result<PathBuf, FontError> {
    #[cfg(target_os = "windows")]
    {
        // %LOCALAPPDATA%\Microsoft\Windows\Fonts
        let local = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .map_err(|_| FontError::NoFontDir)?;
        Ok(local.join("Microsoft").join("Windows").join("Fonts"))
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs_next::home_dir().ok_or(FontError::NoFontDir)?;
        Ok(home.join("Library").join("Fonts"))
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs_next::home_dir().ok_or(FontError::NoFontDir)?;
        Ok(home.join(".local").join("share").join("fonts"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(FontError::NoFontDir)
    }
}

/// Register font files in the Windows registry so they appear in font lists.
#[cfg(target_os = "windows")]
fn register_fonts_windows(font_dir: &Path) -> Result<(), FontError> {
    use std::os::windows::process::CommandExt;

    // Use PowerShell to register each font via Shell.Application
    // This adds entries to HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts
    let entries = std::fs::read_dir(font_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "ttf") {
            let path_str = path.to_string_lossy().to_string();

            // Use reg.exe to add the font to the registry
            let font_name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let reg_name = format!("{font_name} (TrueType)");

            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let result = std::process::Command::new("reg")
                .args([
                    "add",
                    r"HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
                    "/v",
                    &reg_name,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &path_str,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    tracing::debug!(font = %font_name, "registered font in registry");
                }
                Ok(output) => {
                    tracing::warn!(
                        font = %font_name,
                        stderr = %String::from_utf8_lossy(&output.stderr),
                        "failed to register font"
                    );
                }
                Err(e) => {
                    tracing::warn!(font = %font_name, ?e, "failed to run reg.exe");
                }
            }
        }
    }

    // Broadcast WM_FONTCHANGE so applications pick up the new fonts
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class FontHelper { [DllImport(\"user32.dll\")] public static extern int SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam); }'; [FontHelper]::SendMessage([IntPtr]::new(0xFFFF), 0x001D, [IntPtr]::Zero, [IntPtr]::Zero)",
        ])
        .creation_flags(0x0800_0000)
        .output();

    Ok(())
}

/// Configure Windows Terminal to use the Nerd Font.
///
/// Modifies the `settings.json` file to set the default font face.
#[cfg(target_os = "windows")]
fn configure_windows_terminal() -> Result<(), FontError> {
    let settings_paths = windows_terminal_settings_paths();

    for settings_path in settings_paths {
        if !settings_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&settings_path)?;

        // Parse as JSON value to check/modify
        // We do minimal string surgery to avoid pulling in a JSON crate
        // just for this one use case.

        // Check if already configured
        if content.contains(FONT_FAMILY) {
            tracing::debug!(path = %settings_path.display(), "Windows Terminal already configured");
            continue;
        }

        // Find "profiles" -> "defaults" and inject font face
        // Strategy: find `"defaults"` section and add font config
        if let Some(defaults_idx) = content.find("\"defaults\"") {
            if let Some(brace_idx) = content[defaults_idx..].find('{') {
                let insert_pos = defaults_idx + brace_idx + 1;
                let font_config = format!(
                    "\n            \"font\": {{\n                \"face\": \"{FONT_FAMILY}\"\n            }},"
                );

                let mut new_content = String::with_capacity(content.len() + font_config.len());
                new_content.push_str(&content[..insert_pos]);
                new_content.push_str(&font_config);
                new_content.push_str(&content[insert_pos..]);

                std::fs::write(&settings_path, &new_content)?;
                tracing::info!(
                    path = %settings_path.display(),
                    "configured Windows Terminal to use {FONT_FAMILY}"
                );
                return Ok(());
            }
        }

        tracing::warn!(
            path = %settings_path.display(),
            "could not find defaults section in Windows Terminal settings"
        );
    }

    Ok(())
}

/// Possible paths to Windows Terminal settings.json.
#[cfg(target_os = "windows")]
fn windows_terminal_settings_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let local = PathBuf::from(local);

        // Stable
        paths.push(
            local
                .join("Packages")
                .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
                .join("LocalState")
                .join("settings.json"),
        );
        // Preview
        paths.push(
            local
                .join("Packages")
                .join("Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe")
                .join("LocalState")
                .join("settings.json"),
        );
        // Unpackaged (scoop, winget, etc.)
        paths.push(
            local
                .join("Microsoft")
                .join("Windows Terminal")
                .join("settings.json"),
        );
    }

    paths
}
