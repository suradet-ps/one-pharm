//! data-access — SQL Server connection pool and read-only queries for INVS
//!
//! Provides:
//! - Global async connection pool via tiberius + bb8
//! - SQL query functions for drug inventory data
//! - Date/period calculation helpers
//! - Row extraction from raw SQL results
//! - Unified `DataAccessError` that preserves the `tiberius::Error`
//!   source chain (`.code()`, `.is_deadlock()`)
//!
//! All queries are read-only. This system never writes data to INVS.
//!
//! Rolling Window DOS (Approach 2):
//! ─────────────────────────────────────────────────────────────────
//! RM_QTY      = snapshot at end of display period
//! DIS_QTY     = dispensed qty in display period (for display / Turnover)
//! ROLLING_DIS = dispensed qty over rolling_months (for DOS)
//! rolling_days = actual days in rolling window
//!
//! daily_usage = ROLLING_DIS / rolling_days
//! DOS         = RM_QTY / daily_usage
//! ─────────────────────────────────────────────────────────────────

mod database;
mod error;
mod queries;

pub use database::{get_conn, init_pool, reconnect_pool, test_connection, test_connection_with};
pub use error::{DataAccessError, Result};
pub use queries::{
  DrugMovementResult, Warehouse, add_months, date_to_ym, fetch_drug_movements, fetch_last_cost,
  fetch_near_expiry, fetch_warehouses, first_day_of_month, last_day_of_month, period_label,
  prev_month_ym, rolling_start, safe_unit_cost, to_date_range,
};
