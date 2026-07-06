//! Proxmox VE (KVM virtual machine).

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, instrument};

use crate::{Provider, ProviderId};

const SYS_VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
const PRODUCT_NAME_FILE: &str = "/sys/class/dmi/id/product_name";
const VENDOR_NAME: &str = "proxmox";
const QEMU_VENDOR_NAME: &str = "QEMU";
// Matches both "Standard PC (i440FX + PIIX, 1996)" and "Standard PC (Q35 + ICH9, 2009)".
const QEMU_PRODUCT_NAME: &str = "Standard PC";
pub(crate) const IDENTIFIER: ProviderId = ProviderId::ProxmoxVm;

pub(crate) struct ProxmoxVm;

#[async_trait]
impl Provider for ProxmoxVm {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Proxmox using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>, _timeout: Duration) {
        debug!("Checking Proxmox VM");
        if self
            .check_vendor_files(SYS_VENDOR_FILE, PRODUCT_NAME_FILE)
            .await
        {
            debug!("Identified Proxmox VM");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl ProxmoxVm {
    /// Tries to identify Proxmox using vendor file(s).
    ///
    /// Proxmox VE guests expose the default QEMU SMBIOS data ("QEMU" vendor and a
    /// "Standard PC" product name) unless the administrator overrides it, so this
    /// check accepts either an explicit "Proxmox" branding (e.g. set via
    /// `qm set <vmid> --smbios1 manufacturer=Proxmox`) or the QEMU defaults.
    /// Note that the latter may also match other unbranded QEMU/KVM hosts.
    #[instrument(skip_all)]
    async fn check_vendor_files<P: AsRef<Path>>(
        &self,
        sys_vendor_file: P,
        product_name_file: P,
    ) -> bool {
        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            sys_vendor_file.as_ref().display()
        );

        let mut sys_vendor = String::new();

        if sys_vendor_file.as_ref().is_file() {
            match fs::read_to_string(sys_vendor_file).await {
                Ok(content) => sys_vendor = content,
                Err(err) => {
                    debug!("Error reading file: {:?}", err);
                }
            }
        }

        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            product_name_file.as_ref().display()
        );

        let mut product_name = String::new();

        if product_name_file.as_ref().is_file() {
            match fs::read_to_string(product_name_file).await {
                Ok(content) => product_name = content,
                Err(err) => {
                    debug!("Error reading file: {:?}", err);
                }
            }
        }

        if sys_vendor.to_lowercase().contains(VENDOR_NAME)
            || product_name.to_lowercase().contains(VENDOR_NAME)
        {
            return true;
        }

        sys_vendor.contains(QEMU_VENDOR_NAME) && product_name.contains(QEMU_PRODUCT_NAME)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Result;
    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_check_vendor_files_branded_success() -> Result<()> {
        let mut sys_vendor_file = NamedTempFile::new()?;
        let product_name_file = NamedTempFile::new()?;

        sys_vendor_file.write_all(b"Proxmox")?;

        let provider = ProxmoxVm;
        let result = provider
            .check_vendor_files(sys_vendor_file.path(), product_name_file.path())
            .await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_vendor_files_qemu_success() -> Result<()> {
        let mut sys_vendor_file = NamedTempFile::new()?;
        let mut product_name_file = NamedTempFile::new()?;

        sys_vendor_file.write_all(b"QEMU")?;
        product_name_file.write_all(b"Standard PC (i440FX + PIIX, 1996)")?;

        let provider = ProxmoxVm;
        let result = provider
            .check_vendor_files(sys_vendor_file.path(), product_name_file.path())
            .await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_vendor_files_failure() -> Result<()> {
        let mut sys_vendor_file = NamedTempFile::new()?;
        let mut product_name_file = NamedTempFile::new()?;

        // A QEMU vendor without the default product name (e.g. a branded cloud)
        // must not be identified as Proxmox.
        sys_vendor_file.write_all(b"QEMU")?;
        product_name_file.write_all(b"Alibaba Cloud ECS")?;

        let provider = ProxmoxVm;
        let result = provider
            .check_vendor_files(sys_vendor_file.path(), product_name_file.path())
            .await;

        assert!(!result);

        Ok(())
    }
}
