mod error;
pub use error::*;
mod constants;
pub use constants::*;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use server::*;

/// SecurityAlgo
///
/// # Params
///
/// #1 level of security
///
/// #2 seed
///
/// #3 salt or other params
///
/// # Return
///
/// if all seed is 0x00, return None
/// else all seed is not 0xFF return algo data,
/// otherwise return Error
pub type SecurityAlgo = fn(u8, &[u8], &[u8]) -> Result<Option<Vec<u8>>, DoCanError>;
