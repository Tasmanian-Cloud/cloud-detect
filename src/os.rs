//! Operating system detection.

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use std::path::Path;

use strum::Display;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use tracing::debug;

/// The `os-release` file, as specified by freedesktop.org.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const OS_RELEASE_FILES: [&str; 2] = ["/etc/os-release", "/usr/lib/os-release"];
/// Fallback marker files for systems without an `os-release` file.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const ALPINE_RELEASE_FILE: &str = "/etc/alpine-release";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const DEBIAN_VERSION_FILE: &str = "/etc/debian_version";

/// Represents an identifier for an operating system.
#[non_exhaustive]
#[derive(Debug, Default, Display, Eq, PartialEq)]
pub enum OsId {
    /// Unknown operating system.
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    /// AlmaLinux.
    #[strum(serialize = "almalinux")]
    AlmaLinux,
    /// Alpine Linux.
    #[strum(serialize = "alpine")]
    Alpine,
    /// Amazon Linux.
    #[strum(serialize = "amazon")]
    Amazon,
    /// Arch Linux.
    #[strum(serialize = "arch")]
    Arch,
    /// CentOS.
    #[strum(serialize = "centos")]
    CentOS,
    /// Debian.
    #[strum(serialize = "debian")]
    Debian,
    /// Fedora.
    #[strum(serialize = "fedora")]
    Fedora,
    /// FreeBSD.
    #[strum(serialize = "freebsd")]
    FreeBSD,
    /// Gentoo.
    #[strum(serialize = "gentoo")]
    Gentoo,
    /// Kali Linux.
    #[strum(serialize = "kali")]
    Kali,
    /// Linux Mint.
    #[strum(serialize = "mint")]
    Mint,
    /// macOS.
    #[strum(serialize = "macos")]
    MacOS,
    /// NixOS.
    #[strum(serialize = "nixos")]
    NixOS,
    /// openSUSE.
    #[strum(serialize = "opensuse")]
    OpenSuse,
    /// Oracle Linux.
    #[strum(serialize = "oracle")]
    OracleLinux,
    /// Red Hat Enterprise Linux (RHEL).
    #[strum(serialize = "rhel")]
    RHEL,
    /// Rocky Linux.
    #[strum(serialize = "rocky")]
    Rocky,
    /// SUSE Linux Enterprise (SLES/SLED).
    #[strum(serialize = "sles")]
    SLES,
    /// Ubuntu.
    #[strum(serialize = "ubuntu")]
    Ubuntu,
    /// Microsoft Windows.
    #[strum(serialize = "windows")]
    Windows,
}

/// Extracts the value of the `ID` field from `os-release` file contents.
fn parse_os_release_id(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');

            if !value.is_empty() {
                return Some(value.to_lowercase());
            }
        }
    }

    None
}

/// Maps an `os-release` `ID` value to an [OsId].
fn os_id_from_release_id(id: &str) -> OsId {
    match id {
        "almalinux" => OsId::AlmaLinux,
        "alpine" => OsId::Alpine,
        "amzn" => OsId::Amazon,
        "arch" => OsId::Arch,
        "centos" => OsId::CentOS,
        "debian" => OsId::Debian,
        "fedora" => OsId::Fedora,
        "freebsd" => OsId::FreeBSD,
        "gentoo" => OsId::Gentoo,
        "kali" => OsId::Kali,
        "linuxmint" => OsId::Mint,
        "nixos" => OsId::NixOS,
        "ol" => OsId::OracleLinux,
        "rhel" => OsId::RHEL,
        "rocky" => OsId::Rocky,
        "sles" | "sled" => OsId::SLES,
        "ubuntu" => OsId::Ubuntu,
        id if id.starts_with("opensuse") => OsId::OpenSuse,
        _ => OsId::Unknown,
    }
}

/// Detects the operating system from the given `os-release` candidates and
/// fallback marker files.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) fn detect_from_files<P: AsRef<Path>>(
    os_release_files: &[P],
    alpine_release_file: P,
    debian_version_file: P,
) -> OsId {
    for os_release_file in os_release_files {
        debug!(
            "Checking os-release file: {}",
            os_release_file.as_ref().display()
        );

        if os_release_file.as_ref().is_file() {
            match std::fs::read_to_string(os_release_file) {
                Ok(content) => {
                    if let Some(id) = parse_os_release_id(&content) {
                        return os_id_from_release_id(&id);
                    }
                }
                Err(err) => {
                    debug!("Error reading file: {:?}", err);
                }
            }
        }
    }

    if alpine_release_file.as_ref().is_file() {
        return OsId::Alpine;
    }

    if debian_version_file.as_ref().is_file() {
        return OsId::Debian;
    }

    OsId::Unknown
}

/// Detects the host's operating system.
pub(crate) fn detect() -> OsId {
    #[cfg(target_os = "windows")]
    {
        OsId::Windows
    }

    #[cfg(target_os = "macos")]
    {
        OsId::MacOS
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        detect_from_files(&OS_RELEASE_FILES, ALPINE_RELEASE_FILE, DEBIAN_VERSION_FILE)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Result;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_parse_os_release_id() {
        assert_eq!(
            parse_os_release_id("NAME=\"Alpine Linux\"\nID=alpine\n"),
            Some("alpine".to_string())
        );
        // Quoted values and surrounding fields (like VERSION_ID) must not confuse the parser.
        assert_eq!(
            parse_os_release_id(
                "PRETTY_NAME=\"Debian GNU/Linux 12\"\nVERSION_ID=\"12\"\nID=\"debian\"\n"
            ),
            Some("debian".to_string())
        );
        assert_eq!(parse_os_release_id("NAME=Something\n"), None);
    }

    #[test]
    fn test_os_id_from_release_id() {
        assert_eq!(os_id_from_release_id("alpine"), OsId::Alpine);
        assert_eq!(os_id_from_release_id("debian"), OsId::Debian);
        assert_eq!(os_id_from_release_id("ubuntu"), OsId::Ubuntu);
        assert_eq!(os_id_from_release_id("opensuse-leap"), OsId::OpenSuse);
        assert_eq!(os_id_from_release_id("voidlinux"), OsId::Unknown);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[test]
    fn test_detect_from_files_os_release() -> Result<()> {
        let mut os_release_file = NamedTempFile::new()?;
        let alpine_release_file = NamedTempFile::new()?;
        let debian_version_file = NamedTempFile::new()?;

        os_release_file.write_all(b"NAME=\"Alpine Linux\"\nID=alpine\n")?;

        let result = detect_from_files(
            &[os_release_file.path()],
            alpine_release_file.path(),
            debian_version_file.path(),
        );

        assert_eq!(result, OsId::Alpine);

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[test]
    fn test_detect_from_files_fallback() -> Result<()> {
        let debian_version_file = NamedTempFile::new()?;

        let result = detect_from_files(
            &[Path::new("/nonexistent/os-release")],
            Path::new("/nonexistent/alpine-release"),
            debian_version_file.path(),
        );

        assert_eq!(result, OsId::Debian);

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[test]
    fn test_detect_from_files_unknown() {
        let result = detect_from_files(
            &[Path::new("/nonexistent/os-release")],
            Path::new("/nonexistent/alpine-release"),
            Path::new("/nonexistent/debian_version"),
        );

        assert_eq!(result, OsId::Unknown);
    }
}
