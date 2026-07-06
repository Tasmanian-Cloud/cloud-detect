//! Docker.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, instrument};

use crate::{Provider, ProviderId};

const DOCKERENV_FILE: &str = "/.dockerenv";
const CGROUP_FILE: &str = "/proc/self/cgroup";
pub(crate) const IDENTIFIER: ProviderId = ProviderId::Docker;

pub(crate) struct Docker;

#[async_trait]
impl Provider for Docker {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Docker using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>, _timeout: Duration) {
        debug!("Checking Docker");
        if self.check_dockerenv_file(DOCKERENV_FILE).await
            || self.check_cgroup_file(CGROUP_FILE).await
        {
            debug!("Identified Docker");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Docker {
    /// Tries to identify Docker via the `/.dockerenv` marker file.
    ///
    /// The Docker daemon creates this file at the root of every container's
    /// filesystem, regardless of the host's cgroup version.
    #[instrument(skip_all)]
    async fn check_dockerenv_file<P: AsRef<Path>>(&self, dockerenv_file: P) -> bool {
        debug!(
            "Checking {} marker file: {}",
            IDENTIFIER,
            dockerenv_file.as_ref().display()
        );

        dockerenv_file.as_ref().is_file()
    }

    /// Tries to identify Docker via the process's cgroup file.
    ///
    /// On cgroup v1 hosts, container processes are placed in cgroup paths
    /// containing `docker` (e.g. `/docker/<container-id>`).
    #[instrument(skip_all)]
    async fn check_cgroup_file<P: AsRef<Path>>(&self, cgroup_file: P) -> bool {
        debug!(
            "Checking {} cgroup file: {}",
            IDENTIFIER,
            cgroup_file.as_ref().display()
        );

        if cgroup_file.as_ref().is_file() {
            return match fs::read_to_string(cgroup_file).await {
                Ok(content) => content.contains("docker"),
                Err(err) => {
                    debug!("Error reading file: {:?}", err);
                    false
                }
            };
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

    #[tokio::test]
    async fn test_check_dockerenv_file_success() -> Result<()> {
        let dockerenv_file = NamedTempFile::new()?;

        let provider = Docker;
        let result = provider.check_dockerenv_file(dockerenv_file.path()).await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_dockerenv_file_failure() {
        let provider = Docker;
        let result = provider
            .check_dockerenv_file("/nonexistent/.dockerenv")
            .await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_cgroup_file_success() -> Result<()> {
        let mut cgroup_file = NamedTempFile::new()?;
        cgroup_file.write_all(b"12:cpuset:/docker/0123456789abcdef")?;

        let provider = Docker;
        let result = provider.check_cgroup_file(cgroup_file.path()).await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_cgroup_file_failure() -> Result<()> {
        let mut cgroup_file = NamedTempFile::new()?;
        cgroup_file.write_all(b"0::/init.scope")?;

        let provider = Docker;
        let result = provider.check_cgroup_file(cgroup_file.path()).await;

        assert!(!result);

        Ok(())
    }
}
