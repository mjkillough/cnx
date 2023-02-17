/// Battery widget to shows the current capacity
pub mod battery;
/// Command widget to show output of a CLI command
pub mod command;
/// CPU widget to show the current CPU consumption
pub mod cpu;
/// Disk usage widget to show current usage and remaining free space
pub mod disk_usage;
/// LeftWM widget that subscribes to leftwm-state and streams the monitors and tags upfate
#[cfg(feature = "leftwm")]
#[cfg_attr(docsrs, doc(cfg(feature = "leftwm")))]
pub mod leftwm;
/// Sensor widget to periodically parses and displays the output of the sensors provided by the system.
/// pub mod sensors;
/// Volume widget to show the current volume/mute status of the default output device.
pub mod volume;
/// Weather widget to show temperature of your location
/// pub mod weather;
/// Wireless widget to show wireless strength of your SSID
#[cfg(feature = "wireless")]
#[cfg_attr(docsrs, doc(cfg(feature = "wireless")))]
pub mod wireless;
