//! commands.rs — Tauri IPC commands invoked from the Vue frontend
//!
//! Each `#[tauri::command]` function is callable from TypeScript via
//! `invoke()`.  All business logic is delegated to workspace crates.
//!
//! Connection usage: every IPC command that touches the database
//! borrows **one** `PooledConnection` from the global bb8 pool and
//! reuses it for the whole request.  The previous implementation
//! dropped and re-acquired a connection between queries, which
//! thrashes the pool (max size 4) and forfeits bb8's connection
//! reuse benefits.  This matches the tiberius test-suite pattern
//! (one `Client` per request lifecycle).
//!
//! Error handling: `DataAccessError` (a `thiserror` enum that
//! preserves the `tiberius::Error` source chain — `.code()`,
//! `.is_deadlock()`) is serialised to a `String` at the Tauri IPC
//! boundary.  The frontend therefore still receives a `String`, but
//! the in-process code retains type information for retry logic,
//! metrics, etc.

use std::collections::HashMap;

use data_access::{self, DataAccessError, Warehouse, fetch_last_cost, safe_unit_cost};
use kpi_core::{
  DrugKpi, DrugKpiSummary, ExpiryStatus, RawExpiryLotRow, WarehouseKpi, calculate_drug_kpi,
  calculate_warehouse_kpi,
};
use settings::{self, AppSettings, DbConfig};

/// Convert a `DataAccessError` to a Tauri-friendly `String`.
/// We log the full error chain server-side and return the rendered
/// message so the frontend can surface it to the user.
fn into_ipc_error(e: DataAccessError) -> String {
  if let Some(code) = e.sql_code() {
    log::error!("SQL error {code}: {e}");
  } else if e.is_deadlock() {
    log::error!("Deadlock detected: {e}");
  } else {
    log::error!("{e}");
  }
  e.to_string()
}

// ─────────────────────────────────────────────
// Settings commands
// ─────────────────────────────────────────────

/// Return the current app settings.
#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
  Ok(settings::get_settings())
}

/// Save new app settings and reconnect the database pool.
#[tauri::command]
pub async fn save_settings(new_settings: AppSettings) -> Result<(), String> {
  settings::update_settings(new_settings)?;
  data_access::reconnect_pool()
    .await
    .map_err(into_ipc_error)?;
  Ok(())
}

/// Test a database connection with the given config (without saving).
#[tauri::command]
pub async fn test_db_connection(db: DbConfig) -> Result<String, String> {
  data_access::test_connection_with(&db)
    .await
    .map_err(into_ipc_error)
}

// ─────────────────────────────────────────────
// Health check
// ─────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct HealthResult {
  pub api: String,
  pub database: DatabaseHealth,
}

#[derive(serde::Serialize)]
pub struct DatabaseHealth {
  pub status: String,
  pub server: Option<String>,
  pub detail: Option<String>,
}

#[tauri::command]
pub async fn health_check() -> Result<HealthResult, String> {
  match data_access::test_connection().await {
    Ok(version) => Ok(HealthResult {
      api: "ok".to_string(),
      database: DatabaseHealth {
        status: "ok".to_string(),
        server: Some(version),
        detail: None,
      },
    }),
    Err(e) => Ok(HealthResult {
      api: "ok".to_string(),
      database: DatabaseHealth {
        status: "error".to_string(),
        server: None,
        detail: Some(e.to_string()),
      },
    }),
  }
}

// ─────────────────────────────────────────────
// Warehouse list
// ─────────────────────────────────────────────

#[tauri::command]
pub async fn get_warehouses() -> Result<Vec<Warehouse>, String> {
  let mut conn = data_access::get_conn().await.map_err(into_ipc_error)?;
  data_access::fetch_warehouses(&mut conn)
    .await
    .map_err(into_ipc_error)
}

// ─────────────────────────────────────────────
// KPI Summary (warehouse level)
// ─────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct WarehouseKpiOut {
  #[serde(flatten)]
  pub inner: WarehouseKpi,
  pub rolling_months: i32,
}

/// Build a per-drug KPI list from movement + expiry data.
///
/// Shared by `get_kpi_summary`, `get_drug_kpi_list` and
/// `get_drug_kpi_detail` so all three IPC commands reuse the same
/// single-connection-per-request pattern.
async fn build_drug_kpis(
  conn: &mut bb8::PooledConnection<'static, bb8_tiberius::ConnectionManager>,
  stock_id: &str,
  date_from: chrono::NaiveDate,
  date_to: chrono::NaiveDate,
  rolling_months: i32,
  expiry_days: i32,
) -> Result<Vec<DrugKpi>, String> {
  let movement_result =
    data_access::fetch_drug_movements(conn, stock_id, date_from, date_to, rolling_months)
      .await
      .map_err(into_ipc_error)?;
  let expiry_rows = data_access::fetch_near_expiry(conn, stock_id, date_to, expiry_days)
    .await
    .map_err(into_ipc_error)?;

  let mut expiry_map: HashMap<String, Vec<RawExpiryLotRow>> = HashMap::new();
  for lot in expiry_rows {
    expiry_map
      .entry(lot.WORKING_CODE.clone())
      .or_default()
      .push(lot);
  }

  let mut drug_kpis: Vec<DrugKpi> = Vec::with_capacity(movement_result.rows.len());
  for row in &movement_result.rows {
    let unit_cost = if row.RM_QTY.unwrap_or(0.0) == 0.0 {
      let cost_data = fetch_last_cost(conn, &row.WORKING_CODE)
        .await
        .map_err(into_ipc_error)?;
      safe_unit_cost(cost_data)
    } else {
      0.0
    };

    let lots = expiry_map.get(&row.WORKING_CODE).map_or(&[][..], |v| v);
    drug_kpis.push(calculate_drug_kpi(
      row,
      movement_result.display_days,
      movement_result.rolling_days,
      unit_cost,
      lots,
    ));
  }

  Ok(drug_kpis)
}

#[tauri::command]
pub async fn get_kpi_summary(
  stock_id: String,
  year: i32,
  month_from: u32,
  month_to: u32,
  rolling_months: Option<i32>,
  expiry_days: Option<i32>,
) -> Result<WarehouseKpiOut, String> {
  let app_settings = settings::get_settings();
  let rolling = rolling_months.unwrap_or(app_settings.default_rolling_months);
  let exp_days = expiry_days.unwrap_or(app_settings.default_expiry_days);

  let (date_from, date_to) = data_access::to_date_range(year, month_from, month_to);
  let period = data_access::period_label(year, month_from, month_to);

  // One connection for the whole request.
  let mut conn = data_access::get_conn().await.map_err(into_ipc_error)?;

  let warehouses = data_access::fetch_warehouses(&mut conn)
    .await
    .map_err(into_ipc_error)?;
  let stock_name = warehouses
    .iter()
    .find(|w| w.dept_id == stock_id)
    .map_or_else(|| stock_id.clone(), |w| w.dept_name.clone());

  let drug_kpis =
    build_drug_kpis(&mut conn, &stock_id, date_from, date_to, rolling, exp_days).await?;

  let wh = calculate_warehouse_kpi(&drug_kpis, &stock_id, &stock_name, &period);

  Ok(WarehouseKpiOut {
    inner: wh,
    rolling_months: rolling,
  })
}

// ─────────────────────────────────────────────
// KPI Drug List
// ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn get_drug_kpi_list(
  stock_id: String,
  year: i32,
  month_from: u32,
  month_to: u32,
  rolling_months: Option<i32>,
  expiry_days: Option<i32>,
  // Filters
  dos_status: Option<String>,
  dead_stock_only: Option<bool>,
  expiry_only: Option<bool>,
  nlem: Option<String>,
) -> Result<Vec<DrugKpiSummary>, String> {
  let app_settings = settings::get_settings();
  let rolling = rolling_months.unwrap_or(app_settings.default_rolling_months);
  let exp_days = expiry_days.unwrap_or(app_settings.default_expiry_days);

  let (date_from, date_to) = data_access::to_date_range(year, month_from, month_to);

  let mut conn = data_access::get_conn().await.map_err(into_ipc_error)?;
  let drug_kpis =
    build_drug_kpis(&mut conn, &stock_id, date_from, date_to, rolling, exp_days).await?;

  // Apply filters
  let filtered: Vec<DrugKpiSummary> = drug_kpis
    .iter()
    .filter(|d| {
      if let Some(ref ds) = dos_status {
        if d.dos_status.label() != ds.as_str() {
          return false;
        }
      }
      if dead_stock_only.unwrap_or(false) && !d.is_dead_stock {
        return false;
      }
      if expiry_only.unwrap_or(false)
        && !matches!(
          d.expiry_status,
          ExpiryStatus::Expired | ExpiryStatus::Critical | ExpiryStatus::Warning
        )
      {
        return false;
      }
      if let Some(ref n) = nlem {
        if d.nlem != *n {
          return false;
        }
      }
      true
    })
    .map(DrugKpi::to_summary)
    .collect();

  Ok(filtered)
}

// ─────────────────────────────────────────────
// KPI Drug Detail
// ─────────────────────────────────────────────

#[tauri::command]
pub async fn get_drug_kpi_detail(
  working_code: String,
  stock_id: String,
  year: i32,
  month_from: u32,
  month_to: u32,
  rolling_months: Option<i32>,
  expiry_days: Option<i32>,
) -> Result<DrugKpi, String> {
  let app_settings = settings::get_settings();
  let rolling = rolling_months.unwrap_or(app_settings.default_rolling_months);
  let exp_days = expiry_days.unwrap_or(app_settings.default_expiry_days);

  let (date_from, date_to) = data_access::to_date_range(year, month_from, month_to);

  let mut conn = data_access::get_conn().await.map_err(into_ipc_error)?;
  let movement_result =
    data_access::fetch_drug_movements(&mut conn, &stock_id, date_from, date_to, rolling)
      .await
      .map_err(into_ipc_error)?;

  let target = movement_result
    .rows
    .iter()
    .find(|r| r.WORKING_CODE == working_code)
    .ok_or_else(|| format!("ไม่พบยารหัส {working_code}"))?
    .clone();

  let unit_cost = if target.RM_QTY.unwrap_or(0.0) == 0.0 {
    let cost_data = fetch_last_cost(&mut conn, &working_code)
      .await
      .map_err(into_ipc_error)?;
    safe_unit_cost(cost_data)
  } else {
    0.0
  };

  let expiry_rows = data_access::fetch_near_expiry(&mut conn, &stock_id, date_to, exp_days)
    .await
    .map_err(into_ipc_error)?;
  let lots: Vec<_> = expiry_rows
    .into_iter()
    .filter(|l| l.WORKING_CODE == working_code)
    .collect();

  let drug = calculate_drug_kpi(
    &target,
    movement_result.display_days,
    movement_result.rolling_days,
    unit_cost,
    &lots,
  );

  Ok(drug)
}
