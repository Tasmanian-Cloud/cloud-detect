//! BinaryLane.

use std::fs;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use tracing::{debug, error, instrument};

use crate::blocking::Provider;
use crate::ProviderId;

const SYS_VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
const PRODUCT_NAME_FILE: &str = "/sys/class/dmi/id/product_name";
const CHASSIS_ASSET_TAG_FILE: &str = "/sys/class/dmi/id/chassis_asset_tag";
const VENDOR_NAMES: [&str; 2] = ["binarylane", "binary lane"];
pub(crate) const IDENTIFIER: ProviderId = ProviderId::BinaryLane;

pub(crate) struct BinaryLane;

impl Provider for BinaryLane {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify BinaryLane using all the implemented options.
    ///
    /// BinaryLane does not provide a link-local metadata service, so
    /// identification relies on the SMBIOS/DMI data exposed to the guest.
    #[instrument(skip_all)]
    fn identify(&self, tx: SyncSender<ProviderId>, _timeout: Duration) {
        debug!("Checking BinaryLane");
        if self.check_vendor_files(SYS_VENDOR_FILE, PRODUCT_NAME_FILE, CHASSIS_ASSET_TAG_FILE) {
            debug!("Identified BinaryLane");
            if let Err(err) = tx.send(IDENTIFIER) {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl BinaryLane {
    /// Tries to identify BinaryLane using vendor file(s).
    #[instrument(skip_all)]
    fn check_vendor_files<P: AsRef<Path>>(
        &self,
        sys_vendor_file: P,
        product_name_file: P,
        chassis_asset_tag_file: P,
    ) -> bool {
        for vendor_file in [sys_vendor_file, product_name_file, chassis_asset_tag_file] {
            debug!(
                "Checking {} vendor file: {}",
                IDENTIFIER,
                vendor_file.as_ref().display()
            );

            if vendor_file.as_ref().is_file() {
                match fs::read_to_string(vendor_file) {
                    Ok(content) => {
                        let content = content.to_lowercase();

                        if VENDOR_NAMES.iter().any(|&name| content.contains(name)) {
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
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Result;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_check_vendor_files_success() -> Result<()> {
        let mut sys_vendor_file = NamedTempFile::new()?;
        let product_name_file = NamedTempFile::new()?;
        let chassis_asset_tag_file = NamedTempFile::new()?;

        sys_vendor_file.write_all(b"BinaryLane")?;

        let provider = BinaryLane;
        let result = provider.check_vendor_files(
            sys_vendor_file.path(),
            product_name_file.path(),
            chassis_asset_tag_file.path(),
        );

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_vendor_files_failure() -> Result<()> {
        let mut sys_vendor_file = NamedTempFile::new()?;
        let mut product_name_file = NamedTempFile::new()?;
        let chassis_asset_tag_file = NamedTempFile::new()?;

        sys_vendor_file.write_all(b"QEMU")?;
        product_name_file.write_all(b"Standard PC (i440FX + PIIX, 1996)")?;

        let provider = BinaryLane;
        let result = provider.check_vendor_files(
            sys_vendor_file.path(),
            product_name_file.path(),
            chassis_asset_tag_file.path(),
        );

        assert!(!result);

        Ok(())
    }
}
