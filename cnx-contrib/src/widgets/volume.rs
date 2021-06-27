#[cfg(target_os = "openbsd")]
#[cfg(feature = "volume")]
mod volume_bsd;
#[cfg(target_os = "linux")]
#[cfg(feature = "volume")]
mod volume_linux;
#[cfg(target_os = "openbsd")]
#[cfg(feature = "volume")]
pub use volume_bsd::Volume;
#[cfg(target_os = "linux")]
#[cfg(feature = "volume")]
pub use volume_linux::Volume;
