mod driver;
mod events;
mod fixtures;
mod host;
mod model;
mod policy;
mod query;

use std::time::Duration;

pub(crate) use host::ConformanceHost;
pub(crate) use model::Runtime;

pub(crate) const PLUGIN_NAME: &str = "conformance";
pub(crate) const START_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const INVOCATION_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const DRIVER_POLL: Duration = Duration::from_millis(50);
pub(crate) const STATUS_POLL: Duration = Duration::from_millis(5);
