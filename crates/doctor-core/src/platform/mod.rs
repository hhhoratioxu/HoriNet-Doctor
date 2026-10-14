#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{ping, snapshot};

#[cfg(not(windows))]
pub fn snapshot() -> crate::Result<crate::models::Snapshot> {
    Err(crate::Error::Platform(
        "Native inspection currently requires Windows 10/11".into(),
    ))
}
#[cfg(not(windows))]
pub fn ping(_: std::net::IpAddr, _: u8, _: u32) -> crate::Result<crate::models::PingReply> {
    Err(crate::Error::Platform(
        "Native ICMP currently requires Windows".into(),
    ))
}
