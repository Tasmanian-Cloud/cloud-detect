//! Proxmox VE (LXC container).

use std::fs;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use tracing::{debug, error, instrument};

use crate::blocking::Provider;
use crate::ProviderId;

const HOSTS_FILE: &str = "/etc/hosts";
const NETWORK_INTERFACES_FILE: &str = "/etc/network/interfaces";
const RESOLV_CONF_FILE: &str = "/etc/resolv.conf";
// Proxmox's container setup (pve-container) wraps the sections it manages in
// "# --- BEGIN PVE ---" / "# --- END PVE ---" markers.
const PVE_SECTION_MARKER: &str = "--- BEGIN PVE ---";
const PID1_ENVIRON_FILE: &str = "/proc/1/environ";
const SYSTEMD_CONTAINER_FILE: &str = "/run/systemd/container";
const CONTAINER_ENV_ENTRY: &[u8] = b"container=lxc";
const SYSTEMD_CONTAINER_TYPE: &str = "lxc";
pub(crate) const IDENTIFIER: ProviderId = ProviderId::ProxmoxLxc;

pub(crate) struct ProxmoxLxc;

impl Provider for ProxmoxLxc {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify a Proxmox LXC container using all the implemented options.
    #[instrument(skip_all)]
    fn identify(&self, tx: SyncSender<ProviderId>, _timeout: Duration) {
        debug!("Checking Proxmox LXC");
        if self.check_pve_managed_files([HOSTS_FILE, NETWORK_INTERFACES_FILE, RESOLV_CONF_FILE])
            || self.check_container_env(PID1_ENVIRON_FILE, SYSTEMD_CONTAINER_FILE)
        {
            debug!("Identified Proxmox LXC");
            if let Err(err) = tx.send(IDENTIFIER) {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl ProxmoxLxc {
    /// Tries to identify a Proxmox LXC container via PVE-managed config files.
    ///
    /// Proxmox writes the sections it manages inside containers (in `/etc/hosts`,
    /// `/etc/network/interfaces` and `/etc/resolv.conf`) wrapped in
    /// `# --- BEGIN PVE ---` / `# --- END PVE ---` markers, which makes them a
    /// Proxmox-specific in-guest signature.
    #[instrument(skip_all)]
    fn check_pve_managed_files<P: AsRef<Path>>(&self, managed_files: [P; 3]) -> bool {
        for managed_file in managed_files {
            debug!(
                "Checking {} managed file: {}",
                IDENTIFIER,
                managed_file.as_ref().display()
            );

            if managed_file.as_ref().is_file() {
                match fs::read_to_string(managed_file) {
                    Ok(content) => {
                        if content.contains(PVE_SECTION_MARKER) {
                            return true;
                        }
                    }
                    Err(err) => {
                        debug!("Error reading file: {:?}", err);
                    }
                }
            }
        }

        false
    }

    /// Tries to identify an LXC container via its environment.
    ///
    /// Checks PID 1's environment for `container=lxc` (reading `/proc/1/environ`
    /// usually requires root) and systemd's `/run/systemd/container` file. This
    /// identifies LXC in general, not Proxmox specifically, so it may also match
    /// containers managed by other LXC-based platforms.
    #[instrument(skip_all)]
    fn check_container_env<P: AsRef<Path>>(
        &self,
        environ_file: P,
        systemd_container_file: P,
    ) -> bool {
        debug!(
            "Checking {} environ file: {}",
            IDENTIFIER,
            environ_file.as_ref().display()
        );

        if environ_file.as_ref().is_file() {
            match fs::read(environ_file) {
                Ok(content) => {
                    if content
                        .split(|&byte| byte == 0)
                        .any(|entry| entry == CONTAINER_ENV_ENTRY)
                    {
                        return true;
                    }
                }
                Err(err) => {
                    debug!("Error reading file: {:?}", err);
                }
            }
        }

        debug!(
            "Checking {} systemd container file: {}",
            IDENTIFIER,
            systemd_container_file.as_ref().display()
        );

        if systemd_container_file.as_ref().is_file() {
            match fs::read_to_string(systemd_container_file) {
                Ok(content) => {
                    if content.trim() == SYSTEMD_CONTAINER_TYPE {
                        return true;
                    }
                }
                Err(err) => {
                    debug!("Error reading file: {:?}", err);
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Result;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_check_pve_managed_files_success() -> Result<()> {
        let mut hosts_file = NamedTempFile::new()?;
        let interfaces_file = NamedTempFile::new()?;
        let resolv_conf_file = NamedTempFile::new()?;

        hosts_file.write_all(b"# --- BEGIN PVE ---\n127.0.1.1 ct1\n# --- END PVE ---\n")?;

        let provider = ProxmoxLxc;
        let result = provider.check_pve_managed_files([
            hosts_file.path(),
            interfaces_file.path(),
            resolv_conf_file.path(),
        ]);

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_pve_managed_files_failure() -> Result<()> {
        let mut hosts_file = NamedTempFile::new()?;
        let interfaces_file = NamedTempFile::new()?;
        let resolv_conf_file = NamedTempFile::new()?;

        hosts_file.write_all(b"127.0.0.1 localhost\n")?;

        let provider = ProxmoxLxc;
        let result = provider.check_pve_managed_files([
            hosts_file.path(),
            interfaces_file.path(),
            resolv_conf_file.path(),
        ]);

        assert!(!result);

        Ok(())
    }

    #[test]
    fn test_check_container_env_environ_success() -> Result<()> {
        let mut environ_file = NamedTempFile::new()?;
        let systemd_container_file = NamedTempFile::new()?;

        environ_file.write_all(b"PATH=/bin\0container=lxc\0TERM=linux\0")?;

        let provider = ProxmoxLxc;
        let result =
            provider.check_container_env(environ_file.path(), systemd_container_file.path());

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_container_env_systemd_success() -> Result<()> {
        let environ_file = NamedTempFile::new()?;
        let mut systemd_container_file = NamedTempFile::new()?;

        systemd_container_file.write_all(b"lxc\n")?;

        let provider = ProxmoxLxc;
        let result =
            provider.check_container_env(environ_file.path(), systemd_container_file.path());

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_container_env_failure() -> Result<()> {
        let mut environ_file = NamedTempFile::new()?;
        let mut systemd_container_file = NamedTempFile::new()?;

        environ_file.write_all(b"PATH=/bin\0TERM=linux\0")?;
        systemd_container_file.write_all(b"docker\n")?;

        let provider = ProxmoxLxc;
        let result =
            provider.check_container_env(environ_file.path(), systemd_container_file.path());

        assert!(!result);

        Ok(())
    }
}
