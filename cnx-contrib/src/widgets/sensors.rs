#[cfg(feature = "openbsd")]
mod sensors_bsd;
#[cfg(target_os = "linux")]
mod sensors_linux;
#[cfg(feature = "openbsd")]
pub use sensors_bsd::Sensors;
#[cfg(target_os = "linux")]
pub use sensors_linux::Sensors;
