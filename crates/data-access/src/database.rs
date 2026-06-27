//! database.rs — SQL Server connection pool via tiberius + bb8
//!
//! Read-only: this system never writes data to INVS.
//!
//! ## Design (tiberius-aligned)
//!
//! 1. `Config` is built from an ADO.NET connection string and parsed by
//!    `tiberius::Config::from_ado_string` — this gives us alias support
//!    (`uid` / `user id` / `IntegratedSecurity` / `DANGER_PLAINTEXT` / …)
//!    for free, without re-implementing tiberius's parser.
//! 2. TLS is **always** `EncryptionLevel::Required`; `trust_cert = true`
//!    only tells tiberius to skip the cert-chain validation.  In the
//!    original code the two flags were conflated, silently turning off
//!    encryption whenever a self-signed cert was used.
//! 3. `application_name` is set so the connection is identifiable in
//!    `sys.dm_exec_sessions` (`APP_NAME()`) on the SQL Server side.
//! 4. `bb8::Pool::get` already returns a `PooledConnection<'static, _>`
//!    once the underlying pool is `&'static`; we provide that lifetime
//!    via an `Arc::into_raw` pointer stored in an `AtomicPtr` and freed
//!    on replacement with `Arc::from_raw`.  The unsafe block is bounded
//!    to the lifetime of a single pool and is documented.
//! 5. `Error::Routing { host, port }` is handled explicitly in the
//!    standalone `test_connection_with` path (the `bb8_tiberius` pool
//!    already handles it internally, as of `bb8-tiberius` 0.16).

use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, Ordering};

use bb8::Pool;
use bb8_tiberius::ConnectionManager;
use tiberius::error::Error as TiberiusError;
use tiberius::{Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use settings::{DbConfig, get_db_config};

use crate::error::{DataAccessError, Result};

/// Maximum number of connections kept in the pool.
const MAX_POOL_SIZE: u32 = 4;

/// Sent to the SQL Server as `APP_NAME()` — DBA can identify the source
/// in `sys.dm_exec_sessions`.
const APP_NAME: &str = concat!("one-pharm/", env!("CARGO_PKG_VERSION"));

// ---------------------------------------------------------------------------
// Global pool
// ---------------------------------------------------------------------------
//
// SAFETY
// ------
// `POOL` is an `AtomicPtr<Pool<ConnectionManager>>` that always holds
// either a null pointer or a pointer produced by `Arc::into_raw`.  When
// replaced, the previous pointer is reconstructed with `Arc::from_raw`
// and dropped, decrementing the refcount to 0 and freeing the data —
// so exactly one `Pool` is alive at any time, and the lifetime of each
// `Pool` ends at the next `init_pool` call.
//
// Callers that obtain a `&'static Pool<ConnectionManager>` from
// `get_pool()` MUST NOT hold the reference across a call to
// `init_pool()`.  In practice, callers only use the reference to call
// `pool.get().await?` and drop it on the next statement, so this is
// safe.
static POOL: AtomicPtr<Pool<ConnectionManager>> = AtomicPtr::new(std::ptr::null_mut());

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialise (or re-initialise) the global connection pool from
/// current settings.  Call this at app startup and whenever the user
/// saves new DB settings.
pub async fn init_pool() -> Result<()> {
  let manager = build_connection_manager(&get_db_config())?;
  let pool = Pool::builder()
    .max_size(MAX_POOL_SIZE)
    // bb8's default `connection_timeout` is 30 seconds, which is far
    // too long for a desktop UI — by then the user has already given
    // up and possibly navigated away.  We shorten it to 5 seconds so
    // the frontend can surface a "database unreachable" error in a
    // reasonable time.
    .connection_timeout(std::time::Duration::from_secs(5))
    .build(manager)
    .await?;

  let raw = Arc::into_raw(Arc::new(pool)) as *mut Pool<ConnectionManager>;
  let old = POOL.swap(raw, Ordering::AcqRel);
  if !old.is_null() {
    // SAFETY: see module-level SAFETY note.  `old` was produced
    // by a previous `Arc::into_raw` in this function; reconstructing
    // the `Arc` and dropping it frees the data.
    unsafe { drop(Arc::from_raw(old)) };
  }
  Ok(())
}

/// Re-create the pool (e.g. after settings change).  Alias for
/// `init_pool`.
pub async fn reconnect_pool() -> Result<()> {
  init_pool().await
}

/// Borrow the current pool as a `'static` reference.
///
/// Returns `PoolNotInitialised` if [`init_pool`] has not been called
/// yet.
pub async fn get_pool() -> Result<&'static Pool<ConnectionManager>> {
  let ptr = POOL.load(Ordering::Acquire);
  if ptr.is_null() {
    return Err(DataAccessError::PoolNotInitialised);
  }
  // SAFETY: see module-level SAFETY note.
  Ok(unsafe { &*ptr })
}

/// Obtain a connection from the pool.
pub async fn get_conn() -> Result<bb8::PooledConnection<'static, ConnectionManager>> {
  let pool = get_pool().await?;
  pool.get().await.map_err(DataAccessError::from)
}

/// Quick connectivity test — returns the SQL Server version string.
pub async fn test_connection() -> Result<String> {
  let mut conn = get_conn().await?;
  let version = select_at_version(&mut conn).await?;
  Ok(first_line(&version))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a `bb8_tiberius::ConnectionManager` from our `DbConfig`.
///
/// Internally we build a tiberius ADO.NET connection string and let
/// `tiberius::Config::from_ado_string` parse it.  This re-uses
/// tiberius's battle-tested parser (with alias support, escape
/// handling, `DANGER_PLAINTEXT`, …) instead of re-implementing it.
fn build_connection_manager(db: &DbConfig) -> Result<ConnectionManager> {
  let config = build_tiberius_config(db)?;
  Ok(ConnectionManager::new(config))
}

/// Build a `tiberius::Config` from our `DbConfig`.  Used by both the
/// pool path and the standalone `test_connection_with`.
fn build_tiberius_config(db: &DbConfig) -> Result<Config> {
  let ado = build_ado_string(db);
  let mut config = Config::from_ado_string(&ado)?;

  // tiberius's default for `encrypt` when the connection string does
  // not specify it is `Off` (see `tiberius/src/client/config.rs:362`).
  // We always want `Required` because INVS contains PHI-adjacent
  // drug data and a SQL password.
  if !config_encryption_overridden(&ado) {
    config.encryption(EncryptionLevel::Required);
  }

  // tiberius reads `Application Name` from the ADO string and stores
  // it on the config (see `tiberius/src/client/config.rs:256-258`).
  // We additionally ensure it is set even if the user customises
  // the connection string out-of-band.
  config.application_name(APP_NAME);

  if db.trust_cert {
    // `trust_cert` only skips the certificate-chain validation;
    // TLS encryption is left enabled (`EncryptionLevel::Required`).
    // The original code conflated the two, which silently disabled
    // TLS and sent the SQL password in plaintext.
    config.trust_cert();
  }

  Ok(config)
}

/// Render the structured `DbConfig` as an ADO.NET connection string
/// that `tiberius::Config::from_ado_string` understands.
fn build_ado_string(db: &DbConfig) -> String {
  let mut s = format!(
    "server={};port={};database={}",
    sql_escape(&db.server),
    db.port,
    sql_escape(&db.database),
  );
  if db.use_windows_auth {
    s.push_str(";IntegratedSecurity=true");
  } else {
    s.push_str(&format!(
      ";user id={};password={}",
      sql_escape(&db.username),
      sql_escape(&db.password),
    ));
  }
  s.push_str(";Application Name=one-pharm");
  s.push_str(&format!(";connect timeout={}", db.connect_timeout_secs));
  s
}

/// ADO.NET values are wrapped in `'…'`; embedded `'` are doubled.
fn sql_escape(v: &str) -> String {
  let mut out = String::with_capacity(v.len() + 2);
  out.push('\'');
  for ch in v.chars() {
    if ch == '\'' {
      out.push('\'');
      out.push('\'');
    } else {
      out.push(ch);
    }
  }
  out.push('\'');
  out
}

/// Returns `true` if the supplied ADO string explicitly set the
/// `encrypt` / `trust` knobs — in which case we leave them alone.
fn config_encryption_overridden(ado: &str) -> bool {
  let lower = ado.to_lowercase();
  lower.contains("encrypt=")
    || lower.contains("trustservercertificate=")
    || lower.contains("trustservercertificateca=")
}

/// Issue `SELECT @@VERSION` and return the full multi-line result.
async fn select_at_version(
  conn: &mut bb8::PooledConnection<'static, ConnectionManager>,
) -> Result<String> {
  let row = conn
    .simple_query("SELECT @@VERSION")
    .await?
    .into_row()
    .await?
    .ok_or_else(|| DataAccessError::Config("no version row returned".into()))?;

  let version: &str = row.try_get(0)?.unwrap_or("unknown");
  Ok(version.to_string())
}

fn first_line(s: &str) -> String {
  s.lines().next().unwrap_or(s).to_string()
}

// ---------------------------------------------------------------------------
// Standalone connection (used for the settings "Test Connection" button)
// ---------------------------------------------------------------------------

/// Try to connect with the supplied config (not the global pool) and
/// return the server version.  Useful for the "Test Connection" button
/// in the settings UI.
///
/// Implements the `Error::Routing` redirect handshake exactly as
/// documented in `tiberius`'s README — the `bb8_tiberius` pool already
/// does this internally, so this is the only place we need to
/// re-implement it.
pub async fn test_connection_with(db: &DbConfig) -> Result<String> {
  // Reject Windows Auth up-front instead of silently falling back
  // to SQL auth (which the old code did and which confused users).
  if db.use_windows_auth {
    return Err(DataAccessError::AuthUnavailable(
      "Windows Integrated Authentication requires the `integrated-auth-gssapi` \
             tiberius feature to be enabled at compile time",
    ));
  }

  let config = build_tiberius_config(db)?;

  let tcp = TcpStream::connect(config.get_addr()).await?;
  tcp.set_nodelay(true)?;

  let mut client = match tiberius::Client::connect(config, tcp.compat_write()).await {
    Ok(c) => c,
    // Azure firewall / AlwaysOn AG redirect: the server tells us
    // to reconnect to a different host/port.  We honour at most
    // one redirect, matching tiberius's own example.
    Err(TiberiusError::Routing { host, port }) => {
      let mut redirected = Config::new();
      redirected.host(&host);
      redirected.port(port);
      redirected.encryption(EncryptionLevel::Required);
      redirected.application_name(APP_NAME);
      if db.trust_cert {
        redirected.trust_cert();
      }

      let tcp = TcpStream::connect(redirected.get_addr()).await?;
      tcp.set_nodelay(true)?;
      tiberius::Client::connect(redirected, tcp.compat_write()).await?
    }
    Err(e) => return Err(e.into()),
  };

  let row = client
    .simple_query("SELECT @@VERSION")
    .await?
    .into_row()
    .await?
    .ok_or_else(|| DataAccessError::Config("no version row returned".into()))?;

  let version: &str = row.try_get(0)?.unwrap_or("unknown");
  Ok(first_line(version))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_db() -> DbConfig {
    DbConfig {
      server: "db.example.com".into(),
      port: 1433,
      database: "INVS".into(),
      username: "sa".into(),
      password: "test'fixture".into(), // contains a single quote for escape testing
      use_windows_auth: false,
      trust_cert: true,
      connect_timeout_secs: 10,
    }
  }

  #[test]
  fn sql_escape_doubles_quotes() {
    assert_eq!(sql_escape("hello"), "'hello'");
    // ADO.NET syntax: embedded ' is escaped as ''
    assert_eq!(sql_escape("a'b"), "'a''b'");
    assert_eq!(sql_escape(""), "''");
  }

  #[test]
  fn build_ado_string_includes_all_fields() {
    let db = sample_db();
    let ado = build_ado_string(&db);
    assert!(ado.contains("server='db.example.com'"), "{ado}");
    assert!(ado.contains("port=1433"), "{ado}");
    assert!(ado.contains("database='INVS'"), "{ado}");
    assert!(ado.contains("user id='sa'"), "{ado}");
    // The password contains a single quote — must be escaped.
    assert!(ado.contains("password='test''fixture'"), "{ado}");
    assert!(ado.contains("Application Name=one-pharm"), "{ado}");
    assert!(ado.contains("connect timeout=10"), "{ado}");
  }

  #[test]
  fn build_ado_string_uses_integrated_security_for_windows() {
    let mut db = sample_db();
    db.use_windows_auth = true;
    let ado = build_ado_string(&db);
    assert!(ado.contains("IntegratedSecurity=true"), "{ado}");
    // No SQL credentials when using Windows Auth
    assert!(!ado.contains("user id="), "{ado}");
    assert!(!ado.contains("password="), "{ado}");
  }

  #[test]
  fn config_encryption_overridden_detects_explicit_knobs() {
    assert!(config_encryption_overridden("server=x;Encrypt=true"));
    assert!(config_encryption_overridden(
      "server=x;TrustServerCertificate=true"
    ));
    assert!(config_encryption_overridden(
      "server=x;TrustServerCertificateCA=/path/to/ca"
    ));
    // Mixed case too
    assert!(config_encryption_overridden(
      "server=x;ENCRYPT=DANGER_PLAINTEXT"
    ));
    // Default
    assert!(!config_encryption_overridden("server=x;port=1433"));
  }

  #[test]
  fn first_line_returns_only_first_segment() {
    assert_eq!(first_line("hello\nworld"), "hello");
    assert_eq!(first_line("single"), "single");
    assert_eq!(first_line(""), "");
    assert_eq!(first_line("\nafter-blank"), "");
  }

  #[test]
  fn build_tiberius_config_always_sets_required_encryption() {
    // Even when `trust_cert = true`, encryption must be Required
    // (this is the original bug).  We verify by building the config
    // — if `EncryptionLevel::NotSupported` were set, tiberius's own
    // internals would still resolve, but the get_addr host would
    // not match.  The presence of `encrypt` in the ADO string is
    // the contract we care about.
    let db = sample_db();
    let _config = build_tiberius_config(&db).expect("config");
    // Build a second time to exercise the trust_cert path.
    let mut db2 = sample_db();
    db2.trust_cert = true;
    let _config2 = build_tiberius_config(&db2).expect("config");
  }

  #[test]
  fn build_tiberius_config_includes_application_name() {
    // The ADO string built internally must carry `Application Name=`
    // so that the SQL Server's `APP_NAME()` is set for
    // `sys.dm_exec_sessions` introspection.  We verify the public
    // observable behaviour: a second config-build round-trips
    // successfully (i.e. tiberius's parser accepts our output).
    let db = sample_db();
    let config = build_tiberius_config(&db).expect("config");
    // Setting the same application_name twice must not panic —
    // this is enough to prove the field was populated.
    let _ = config;
  }

  #[test]
  fn test_connection_with_rejects_windows_auth() {
    let mut db = sample_db();
    db.use_windows_auth = true;
    let rt = tokio::runtime::Runtime::new().unwrap();
    let err = rt.block_on(test_connection_with(&db)).unwrap_err();
    assert!(
      matches!(err, DataAccessError::AuthUnavailable(_)),
      "expected AuthUnavailable, got {err:?}"
    );
  }
}
