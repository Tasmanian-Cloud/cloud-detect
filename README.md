# cloud-detect

![maintenance-status](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![crates-badge](https://img.shields.io/crates/v/cloud-detect.svg)](https://crates.io/crates/cloud-detect)
[![License: GPL v3](https://img.shields.io/badge/license-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/license/mit)
[![CI](https://github.com/nikhil-prabhu/cloud-detect/actions/workflows/ci.yml/badge.svg)](https://github.com/nikhil-prabhu/cloud-detect/actions)
[![CD](https://github.com/nikhil-prabhu/cloud-detect/actions/workflows/cd.yml/badge.svg)](https://github.com/nikhil-prabhu/cloud-detect/actions)

A Rust library to detect the cloud service provider of a host.

This library is inspired by the Python-based [cloud-detect](https://github.com/dgzlopes/cloud-detect)
and the Go-based [satellite](https://github.com/banzaicloud/satellite) modules.

Like these modules, `cloud-detect` uses a combination of checking vendor files and metadata endpoints to accurately
determine the cloud provider of a host.

## Features

- Currently, this module supports the identification of the following providers:
  - Akamai Cloud (`akamai`)
  - Amazon Web Services (`aws`)
  - Microsoft Azure (`azure`)
  - BinaryLane (`binarylane`)
  - Google Cloud Platform (`gcp`)
  - Alibaba Cloud (`alibaba`)
  - OpenStack (`openstack`)
  - DigitalOcean (`digitalocean`)
  - Oracle Cloud Infrastructure (`oci`)
  - Vultr (`vultr`)
- Additionally, this module supports the identification of the following environments:
  - Docker containers (`docker`)
  - Proxmox VE KVM virtual machines (`proxmox-vm`)
  - Proxmox VE LXC containers (`proxmox-lxc`)
- Operating system detection (e.g. Alpine vs Debian) via `detect_os()`.
- Fast, simple and extensible.
- Real-time console logging using the [`tracing`](https://crates.io/crates/tracing) crate.

### Detection notes

- **Docker** is detected via the `/.dockerenv` marker file or a `docker` entry in `/proc/self/cgroup`.
  Since containers can run on any host, detection inside a container running on a supported cloud is a
  race between the container check and the cloud's checks; the local file check typically wins, so
  expect `docker` rather than the underlying cloud in that case.
- **Proxmox VE KVM virtual machines** (`proxmox-vm`) expose generic QEMU SMBIOS data by default (a
  `QEMU` system vendor and a `Standard PC` product name), so detection relies on that fingerprint and
  may also match other unbranded QEMU/KVM hosts. For unambiguous detection, brand your VMs on the
  Proxmox host with `qm set <vmid> --smbios1 manufacturer=Proxmox`; explicit `Proxmox` branding is
  matched first.
- **Proxmox VE LXC containers** (`proxmox-lxc`) are detected via the `# --- BEGIN PVE ---` sections
  that Proxmox writes into the container's `/etc/hosts`, `/etc/network/interfaces` and
  `/etc/resolv.conf`. As a fallback, generic LXC markers are checked (`container=lxc` in
  `/proc/1/environ`, which usually requires root, and `/run/systemd/container`); the fallback may
  also match LXC containers managed by other platforms.
- **BinaryLane** does not provide a link-local metadata service, so detection relies on BinaryLane
  branding in the guest's SMBIOS/DMI data (system vendor, product name or chassis asset tag).

## Usage

First, add the library to your project by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
# ...
cloud-detect = "3"
tokio = { version = "1", features = ["full"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] } # Optional; for logging.
```

To use the non-async blocking API instead, enable the `blocking` feature:

```toml
[dependencies]
# ...
cloud-detect = { version = "3", features = ["blocking"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] } # Optional; for logging.
```

Detect the cloud provider and print the result (with default timeout; async).

```rust
use cloud_detect::detect;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init(); // Optional; for logging

    let provider = detect(None).await;

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

Detect the cloud provider and print the result (with default timeout; blocking).

```rust
use cloud_detect::blocking::detect;

fn main() {
    tracing_subscriber::fmt::init(); // Optional; for logging

    let provider = detect(None).unwrap();

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

Detect the cloud provider and print the result (with custom timeout; async).

```rust
use cloud_detect::detect;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init(); // Optional; for logging

    let provider = detect(Some(10)).await;

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

Detect the cloud provider and print the result (with custom timeout; blocking).

```rust
use cloud_detect::blocking::detect;

fn main() {
    tracing_subscriber::fmt::init(); // Optional; for logging

    let provider = detect(Some(10)).unwrap();

    // When tested on AWS:
    println!("{}", provider); // "aws"

    // When tested on local/non-supported cloud environment:
    println!("{}", provider); // "unknown"
}
```

You can also detect the host's operating system (e.g. to tell Alpine from Debian). This is a
synchronous function (it only reads a few small local files) and works with both the async and
blocking APIs.

```rust
use cloud_detect::detect_os;

fn main() {
    let os = detect_os();
    println!("{}", os); // e.g. "alpine", "debian", "ubuntu"; "unknown" if undetermined.
}
```

You can also check the list of currently supported cloud providers.

Async:

```rust
use cloud_detect::supported_providers;

#[tokio::main]
async fn main() {
    println!("Supported providers: {:?}", supported_providers().await);
}
```

Blocking:

```rust
use cloud_detect::blocking::supported_providers;

fn main() {
    println!("Supported providers: {:?}", supported_providers().unwrap());
}
```

For more detailed documentation, please refer to the [Crate Documentation](https://docs.rs/cloud-detect).

## Contributing

Contributions are welcome and greatly appreciated! If you’d like to contribute to cloud-detect, here’s how you can help.

### 1. Report Issues

If you encounter a bug, unexpected behavior, or have a feature request, please open
an [issue](https://github.com/nikhil-prabhu/cloud-detect/issues/new).
Be sure to include:

- A clear description of the issue.
- Steps to reproduce, if applicable.
- Details about your environment.

### 2. Submit Pull Requests

If you're submitting a [pull request](https://github.com/nikhil-prabhu/cloud-detect/compare), please ensure the
following.

- Your code is formatted using `cargo fmt` (the Rust `nightly` channel is required, as a few unstable features are
  used).

```bash
cargo fmt +nightly --all
cargo fmt +nightly --all --check
```

- Code lints pass with:

```bash
cargo clippy --all-targets --all-features --workspace -- -D warnings
```

- Your code contains sufficient unit tests and that all tests pass.

```bash
cargo test --locked --all-features --workspace
```

### 3. Improve Documentation

If you find areas in the documentation that are unclear or incomplete, feel free to update the README or crate-level
documentation. Open a pull request with your improvements.

### 4. Review Pull Requests

You can also contribute by
reviewing [open pull requests](https://github.com/nikhil-prabhu/cloud-detect/pulls?q=is%3Aopen+is%3Apr). Providing
constructive feedback helps maintain a
high-quality
codebase.
