//! error.rs — Unified error type for the `data-access` crate.
//!
//! Mirrors `tiberius::Error` semantics so callers can introspect
//! `.code()`, `.is_deadlock()` and the source chain instead of getting
//! a flattened `String`.
//!
//! Implements `serde::Serialize` so Tauri IPC commands can return it
//! directly without a manual `String` conversion at the call-site.

use serde::{Serialize, Serializer};

/// All errors that can be produced by the data-access layer.
#[derive(Debug, thiserror::Error)]
pub enum DataAccessError {
  /// An error originating in the `tiberius` TDS client
  /// (protocol, I/O, server reply, encoding, …).
  #[error(transparent)]
  Tiberius(#[from] tiberius::error::Error),

  /// Pool exhaustion / connection acquisition failure from `bb8`.
  /// Returned by `Pool::get().await`.
  #[error(transparent)]
  Pool(#[from] bb8::RunError<bb8_tiberius::Error>),

  /// Direct `bb8_tiberius::Error` returned by `Pool::builder().build()`.
  /// We collapse it into the same variant as `Pool` because
  /// `bb8_tiberius::Error` is a transparent wrapper over
  /// `tiberius::error::Error` / `io::Error` and doesn't carry extra
  /// information at this layer.
  #[error(transparent)]
  PoolBuild(#[from] bb8_tiberius::Error),

  /// Generic I/O error (TCP connect, set_nodelay, …).
  #[error(transparent)]
  Io(#[from] std::io::Error),

  /// The connection pool has not been initialised yet.
  #[error("database pool is not initialised — call `init_pool` first")]
  PoolNotInitialised,

  /// A configuration problem (missing field, invalid combination, …).
  #[error("config error: {0}")]
  Config(String),

  /// Auth method requested at runtime is not compiled in
  /// (e.g. Windows Integrated Auth without the gssapi feature).
  #[error("auth method not available: {0}")]
  AuthUnavailable(&'static str),
}

impl DataAccessError {
  /// Convenience: forward to `tiberius::Error::is_deadlock`
  /// when the underlying cause is a server-side error.
  pub fn is_deadlock(&self) -> bool {
    match self {
      Self::Tiberius(e) => e.is_deadlock(),
      // `bb8_tiberius::Error` is a transparent wrapper around
      // `tiberius::Error` / `io::Error`; we can't introspect
      // deadlock on this path without losing the type info.
      _ => false,
    }
  }

  /// SQL Server error code if this is a server-side error, else `None`.
  pub fn sql_code(&self) -> Option<u32> {
    match self {
      Self::Tiberius(e) => e.code(),
      _ => None,
    }
  }
}

/// `Result` alias used throughout the data-access layer.
pub type Result<T> = std::result::Result<T, DataAccessError>;

/// Flatten the error chain into the message `String` consumed by
/// Tauri IPC.  We serialise the error as a plain string because
/// `DataAccessError` carries non-`Serialize` inner types
/// (`tiberius::Error`, `bb8::RunError`, …).
impl Serialize for DataAccessError {
  fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
    s.serialize_str(&self.to_string())
  }
}
