//! Everything related to shelling out to the `aws` CLI: the profile-
//! scoped client (`client`) and a single-instance handle for start/stop/
//! schedule-shutdown (`instance`). Each module owns the JSON response
//! shapes it needs to parse `aws`'s `--output json` output.

mod aws_command;
mod client;
mod instance;

pub use client::{AwsClient, list_profiles};
#[allow(unused_imports)] // part of the public API surface even if unnamed by callers
pub use instance::{Instance, InstanceEntry};
