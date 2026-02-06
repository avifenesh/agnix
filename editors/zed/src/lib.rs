use std::fs;
use zed_extension_api::{
    self as zed, Architecture, Command, DownloadedFileType, GithubReleaseOptions, LanguageServerId,
    Os, Result,
};

/// Zed extension that integrates the agnix LSP for validating agent configurations.
struct AgnixExtension {
    /// Cached path to the agnix-lsp binary, if already downloaded.
    cached_binary_path: Option<String>,
}

/// Returns the expected release asset name and download file type for a given platform.
fn asset_for_platform(os: Os, arch: Architecture) -> Result<(&'static str, DownloadedFileType)> {
    match (os, arch) {
        (Os::Mac, Architecture::Aarch64) => Ok((
            "agnix-lsp-aarch64-apple-darwin.tar.gz",
            DownloadedFileType::GzipTar,
        )),
        // macOS x86_64 uses the ARM binary via Rosetta 2
        (Os::Mac, Architecture::X8664) => Ok((
            "agnix-lsp-aarch64-apple-darwin.tar.gz",
            DownloadedFileType::GzipTar,
        )),
        (Os::Linux, Architecture::X8664) => Ok((
            "agnix-lsp-x86_64-unknown-linux-gnu.tar.gz",
            DownloadedFileType::GzipTar,
        )),
        (Os::Linux, Architecture::Aarch64) => Ok((
            "agnix-lsp-aarch64-unknown-linux-gnu.tar.gz",
            DownloadedFileType::GzipTar,
        )),
        (Os::Windows, Architecture::X8664) => Ok((
            "agnix-lsp-x86_64-pc-windows-msvc.zip",
            DownloadedFileType::Zip,
        )),
        _ => Err(format!("unsupported platform: {os:?} {arch:?}",)),
    }
}

/// Returns the binary name for the LSP server on the given OS.
fn binary_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "agnix-lsp.exe",
        _ => "agnix-lsp",
    }
}

impl AgnixExtension {
    /// Resolves the agnix-lsp binary path, downloading it from GitHub releases if needed.
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        let (platform, _) = zed::current_platform();
        let bin = binary_name(platform);

        // Check cached path first
        if let Some(ref path) = self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        // Determine the latest release from GitHub
        let release = zed::latest_github_release(
            "avifenesh/agnix",
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let version = &release.version;
        let version_dir = format!("agnix-lsp-{version}");
        let binary_path = format!("{version_dir}/{bin}");

        // If this version is already downloaded, use it
        if fs::metadata(&binary_path).is_ok_and(|m| m.is_file()) {
            self.cached_binary_path = Some(binary_path.clone());
            return Ok(binary_path);
        }

        // Download the appropriate asset for this platform
        let (platform, arch) = zed::current_platform();
        let (asset_name, file_type) = asset_for_platform(platform, arch)?;

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no release asset found matching {asset_name}"))?;

        zed::download_file(&asset.download_url, &version_dir, file_type)
            .map_err(|e| format!("failed to download {asset_name}: {e}"))?;

        zed::make_file_executable(&binary_path)?;

        self.cached_binary_path = Some(binary_path.clone());

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        Ok(binary_path)
    }
}

impl zed::Extension for AgnixExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Command> {
        let binary_path = self.language_server_binary_path(language_server_id)?;
        Ok(Command {
            command: binary_path,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(AgnixExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_name_mac_aarch64() {
        let (name, file_type) = asset_for_platform(Os::Mac, Architecture::Aarch64).unwrap();
        assert_eq!(name, "agnix-lsp-aarch64-apple-darwin.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_mac_x86_64_uses_rosetta() {
        let (name, file_type) = asset_for_platform(Os::Mac, Architecture::X8664).unwrap();
        assert_eq!(name, "agnix-lsp-aarch64-apple-darwin.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_linux_x86_64() {
        let (name, file_type) = asset_for_platform(Os::Linux, Architecture::X8664).unwrap();
        assert_eq!(name, "agnix-lsp-x86_64-unknown-linux-gnu.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_linux_aarch64() {
        let (name, file_type) = asset_for_platform(Os::Linux, Architecture::Aarch64).unwrap();
        assert_eq!(name, "agnix-lsp-aarch64-unknown-linux-gnu.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_windows_x86_64() {
        let (name, file_type) = asset_for_platform(Os::Windows, Architecture::X8664).unwrap();
        assert_eq!(name, "agnix-lsp-x86_64-pc-windows-msvc.zip");
        assert!(matches!(file_type, DownloadedFileType::Zip));
    }

    #[test]
    fn unsupported_platform_returns_error() {
        let result = asset_for_platform(Os::Windows, Architecture::Aarch64);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("unsupported platform"));
    }

    #[test]
    fn binary_name_unix() {
        assert_eq!(binary_name(Os::Mac), "agnix-lsp");
        assert_eq!(binary_name(Os::Linux), "agnix-lsp");
    }

    #[test]
    fn binary_name_windows() {
        assert_eq!(binary_name(Os::Windows), "agnix-lsp.exe");
    }
}
