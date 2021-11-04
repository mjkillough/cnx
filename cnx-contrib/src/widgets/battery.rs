#[cfg(feature = "openbsd")]
mod battery_bsd;
#[cfg(target_os = "linux")]
mod battery_linux;
#[cfg(feature = "openbsd")]
pub use battery_bsd::Battery;
#[cfg(target_os = "linux")]
pub use battery_linux::{Battery, BatteryInfo, Status};
