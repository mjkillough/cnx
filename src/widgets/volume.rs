#[cfg(feature = "openbsd")]
pub mod volume_bsd;
#[cfg(feature = "openbsd")]
pub use volume_bsd::Volume;
#[cfg(target_os = "linux")]
pub mod volume_linux;
#[cfg(target_os = "linux")]
pub use volume_linux::Volume;
