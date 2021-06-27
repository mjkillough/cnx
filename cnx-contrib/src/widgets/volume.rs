#[cfg(feature = "openbsd")]
mod volume_bsd;
#[cfg(target_os = "linux")]
mod volume_linux;
#[cfg(feature = "openbsd")]
pub use volume_bsd::Volume;
#[cfg(target_os = "linux")]
pub use volume_linux::Volume;
