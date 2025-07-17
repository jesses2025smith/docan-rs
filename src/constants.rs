pub const P2_MAX: u16 = 50;
pub const P2_STAR_MAX: u16 = 500;
pub const DEFAULT_P2_START_MS: u64 = 5_000;

#[cfg(feature = "client")]
pub(crate) const LOG_TAG_CLIENT: &'static str = "DoCanClient - ";
#[cfg(feature = "server")]
pub(crate) const LOG_TAG_SERVER: &'static str = "DoCanServer - ";
