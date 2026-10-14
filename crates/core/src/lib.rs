//! Folio core: everything the launcher knows that needs no I/O.
//!
//! The suite catalog, version ordering, release parsing, choosing the right download for a
//! platform and reading checksum files. Nothing here touches the network, the disk or a window,
//! so it is plain data in, plain data out (and builds for WebAssembly too).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod catalog;
pub mod checksum;
pub mod platform;
pub mod release;
pub mod status;
pub mod version;

pub use catalog::{App, ORG, SUITE, SUITE_REPO, Stage};
pub use platform::{Arch, Os, Platform};
pub use release::{Asset, Release};
pub use status::Status;
pub use version::Version;
