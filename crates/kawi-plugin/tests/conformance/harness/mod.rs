mod driver;
mod editor;
mod events;
mod fixtures;
mod host;
mod model;
mod policy;
mod query;

use std::time::Duration;

pub(crate) use host::ConformanceHost;
pub(crate) use model::{HostRequestRecord, Observation, Runtime};

pub(crate) const PLUGIN_NAME: &str = "conformance";
pub(crate) const START_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const INVOCATION_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const DRIVER_POLL: Duration = Duration::from_millis(50);
pub(crate) const STATUS_POLL: Duration = Duration::from_millis(5);
