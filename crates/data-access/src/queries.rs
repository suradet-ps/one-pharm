//! queries.rs — SQL queries for the INVS database via tiberius
//!
//! Ported from the Python `queries.py` module.
//!
//! All queries are read-only. This system never writes data to INVS.

use chrono::{Datelike, NaiveDate};
use kpi_core::{RawDrugRow, RawExpiryLotRow};
use tiberius::{Row, numeric::Numeric};

// ─────────────────────────────────────────────
// Date helpers
// ─────────────────────────────────────────────

/// date → "yyyymm"
pub fn date_to_ym(d: NaiveDate) -> String {
  d.format("%Y%m").to_string()
}

/// "yyyymm" → "yyyymm" of the previous month
pub fn prev_month_ym(ym: &str) -> String {
  let y: i32 = ym[..4].parse().unwrap_or(2025);
  let m: i32 = ym[4..].parse().unwrap_or(1);
  if m == 1 {
    format!("{}12", y - 1)
  } else {
    format!("{}{:02}", y, m - 1)
  }
}

/// Move a date forward/backward by `n` months, returning the 1st of that month.
pub fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
  let total = d.month0() as i32 + n;
  let year = d.year() + total.div_euclid(12);
  let month = (total.rem_euclid(12) + 1) as u32;
  NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(d)
}

/// First day of the month containing `d`.
pub fn first_day_of_month(d: NaiveDate) -> NaiveDate {
  NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)
}

/// Last day of the month containing `d`.
pub fn last_day_of_month(d: NaiveDate) -> NaiveDate {
  let (y, m) = (d.year(), d.month());
  if m == 12 {
    NaiveDate::from_ymd_opt(y + 1, 1, 1)
  } else {
    NaiveDate::from_ymd_opt(y, m + 1, 1)
  }
  .map_or(d, |d| d.pred_opt().unwrap_or(d))
}

/// Calculate the start date of the rolling window.
pub fn rolling_start(date_to: NaiveDate, rolling_months: i32) -> NaiveDate {
  first_day_of_month(add_months(date_to, -(rolling_months - 1)))
}

/// Convert Buddhist Era year + month range to (date_from, date_to) in CE.
pub fn to_date_range(year_be: i32, month_from: u32, month_to: u32) -> (NaiveDate, NaiveDate) {
  let year_ce = year_be - 543;
  let date_from = NaiveDate::from_ymd_opt(year_ce, month_from, 1)
    .unwrap_or_else(|| NaiveDate::from_ymd_opt(year_ce, 1, 1).unwrap());
  let date_to_first = NaiveDate::from_ymd_opt(year_ce, month_to, 1)
    .unwrap_or_else(|| NaiveDate::from_ymd_opt(year_ce, 1, 1).unwrap());
  let date_to = last_day_of_month(date_to_first);
  (date_from, date_to)
}

/// Generate a Thai period label.
pub fn period_label(year_be: i32, month_from: u32, month_to: u32) -> String {
  let months_th = [
    "ม.ค.",
    "ก.พ.",
    "มี.ค.",
    "เม.ย.",
    "พ.ค.",
    "มิ.ย.",
    "ก.ค.",
    "ส.ค.",
    "ก.ย.",
    "ต.ค.",
    "พ.ย.",
    "ธ.ค.",
  ];
  let mf = months_th.get((month_from - 1) as usize).unwrap_or(&"?");
  let mt = months_th.get((month_to - 1) as usize).unwrap_or(&"?");
  if month_from == month_to {
    format!("{mf} {year_be}")
  } else {
    format!("{mf} – {mt} {year_be}")
  }
}

// ─────────────────────────────────────────────
// SQL Statements
// ─────────────────────────────────────────────

/// Query 1 — All drug warehouses
const SQL_GET_WAREHOUSES: &str = r"
SELECT DEPT_ID, DEPT_NAME
FROM   DEPT_ID WITH (NOLOCK)
WHERE  (HIDE = '' OR HIDE IS NULL OR HIDE = 'N')
  AND  (DEPT_TYPE = 2 OR DEPT_TYPE = 3)
ORDER BY DEPT_NAME
";

/// Query 2 — Drug movements with rolling DIS
///
/// DRUG_GN is the master table — ALL working codes are shown,
/// regardless of whether they exist in INV_MD (stock master).
/// This ensures drugs with zero inventory still appear in the list.
///
/// Parameters (positional @P1..@P11):
///   1,2   ym_fw          FW_QTY, FW_VALUE
///   3,4   ym_display_to  RM_QTY, RM_VALUE
///   5     stock_id       CARD display
///   6,7   ym1, ym2       CARD display period
///   8     stock_id       CARD rolling
///   9,10  ym_roll_from, ym_display_to  CARD rolling window
///   11    stock_id       INV_MD / scalar subqueries
const SQL_GET_DRUG_MOVEMENTS: &str = r"
SELECT *
FROM (
    SELECT
        g.WORKING_CODE,
        g.DRUG_NAME,
        i.LAST_PACK_RATIO,
        dbo.IS_ED(g.IS_ED)  AS NLEM,

        (SELECT TOP 1 c.REMAIN_QTY
         FROM   CARD c WITH (NOLOCK)
         WHERE  LEFT(dbo.ce2cymd(c.OPERATE_DATE), 6) <= @P1
           AND  c.WORKING_CODE = g.WORKING_CODE
           AND  c.STOCK_ID     = @P11
         ORDER BY c.OPERATE_DATE DESC, c.RECORD_NUMBER DESC
        ) AS FW_QTY,

        (SELECT TOP 1 c.REMAIN_VALUE
         FROM   CARD c WITH (NOLOCK)
         WHERE  LEFT(dbo.ce2cymd(c.OPERATE_DATE), 6) <= @P2
           AND  c.WORKING_CODE = g.WORKING_CODE
           AND  c.STOCK_ID     = @P11
         ORDER BY c.OPERATE_DATE DESC, c.RECORD_NUMBER DESC
        ) AS FW_VALUE,

        Cd.RCV_QTY,
        ISNULL(Cd.RCV_VALUE, 0) AS RCV_VALUE,
        Cd.DIS_QTY,
        ISNULL(Cd.DIS_VALUE, 0) AS DIS_VALUE,

        ISNULL((
            SELECT TOP 1 c.REMAIN_QTY
            FROM   CARD c WITH (NOLOCK)
            WHERE  LEFT(dbo.ce2cymd(c.OPERATE_DATE), 6) <= @P3
              AND  c.WORKING_CODE = g.WORKING_CODE
              AND  c.STOCK_ID     = @P11
            ORDER BY c.OPERATE_DATE DESC, c.RECORD_NUMBER DESC
        ), 0) AS RM_QTY,

        i.QTY_ON_HAND,

        (SELECT TOP 1 c.REMAIN_VALUE
         FROM   CARD c WITH (NOLOCK)
         WHERE  LEFT(dbo.ce2cymd(c.OPERATE_DATE), 6) <= @P4
           AND  c.WORKING_CODE = g.WORKING_CODE
           AND  c.STOCK_ID     = @P11
         ORDER BY c.OPERATE_DATE DESC, c.RECORD_NUMBER DESC
        ) AS RM_VALUE,

        ISNULL(Cr.ROLLING_DIS_QTY,   0) AS ROLLING_DIS_QTY,
        ISNULL(Cr.ROLLING_DIS_VALUE, 0) AS ROLLING_DIS_VALUE

    FROM DRUG_GN g WITH (NOLOCK)
    LEFT JOIN INV_MD i WITH (NOLOCK)
           ON i.WORKING_CODE = g.WORKING_CODE
          AND i.DEPT_ID = @P11

    LEFT JOIN (
        SELECT
            c1.WORKING_CODE, c1.STOCK_ID,
            SUM(CASE
                WHEN c1.R_S_STATUS IN ('R','A','N','O','I','G') THEN  c1.ACTIVE_QTY
                WHEN c1.R_S_STATUS IN ('M','E','B')             THEN -c1.ACTIVE_QTY
            END) AS RCV_QTY,
            SUM(CASE
                WHEN c1.R_S_STATUS IN ('R','A','N','O','I','G') THEN  c1.[VALUE]
                WHEN c1.R_S_STATUS IN ('M','E','B')             THEN -c1.[VALUE]
            END) AS RCV_VALUE,
            SUM(CASE
                WHEN c1.R_S_STATUS IN ('S','P','D') THEN  c1.ACTIVE_QTY
                WHEN c1.R_S_STATUS IN ('C','Z','U') THEN -c1.ACTIVE_QTY
            END) AS DIS_QTY,
            SUM(CASE
                WHEN c1.R_S_STATUS IN ('S','P','D') THEN  c1.[VALUE]
                WHEN c1.R_S_STATUS IN ('C','Z','U') THEN -c1.[VALUE]
            END) AS DIS_VALUE
        FROM CARD c1 WITH (NOLOCK)
        WHERE c1.STOCK_ID = @P5
          AND LEFT(dbo.ce2cymd(c1.OPERATE_DATE), 6) BETWEEN @P6 AND @P7
        GROUP BY c1.WORKING_CODE, c1.STOCK_ID
    ) AS Cd ON Cd.WORKING_CODE = g.WORKING_CODE AND Cd.STOCK_ID = @P11

    LEFT JOIN (
        SELECT
            c2.WORKING_CODE, c2.STOCK_ID,
            SUM(CASE
                WHEN c2.R_S_STATUS IN ('S','P','D') THEN  c2.ACTIVE_QTY
                WHEN c2.R_S_STATUS IN ('C','Z','U') THEN -c2.ACTIVE_QTY
            END) AS ROLLING_DIS_QTY,
            SUM(CASE
                WHEN c2.R_S_STATUS IN ('S','P','D') THEN  c2.[VALUE]
                WHEN c2.R_S_STATUS IN ('C','Z','U') THEN -c2.[VALUE]
            END) AS ROLLING_DIS_VALUE
        FROM CARD c2 WITH (NOLOCK)
        WHERE c2.STOCK_ID = @P8
          AND LEFT(dbo.ce2cymd(c2.OPERATE_DATE), 6) BETWEEN @P9 AND @P10
        GROUP BY c2.WORKING_CODE, c2.STOCK_ID
    ) AS Cr ON Cr.WORKING_CODE = g.WORKING_CODE AND Cr.STOCK_ID = @P11

) AS T
ORDER BY T.NLEM, T.DRUG_NAME
";

/// Query 3 — Last purchase cost (fallback)
const SQL_GET_LAST_COST: &str = r"
SELECT TOP 1 LAST_BUY_COST, LAST_PACK_RATIO
FROM   DRUG_VN WITH (NOLOCK)
WHERE  WORKING_CODE    = @P1
  AND  LAST_PACK_RATIO > 0
ORDER BY LAST_BUY DESC
";

/// Query 4 — Lots near expiry / already expired
const SQL_GET_NEAR_EXPIRY: &str = r"
SELECT
    c.WORKING_CODE,
    g.DRUG_NAME,
    dbo.IS_ED(g.IS_ED)  AS NLEM,
    c.LOT_NO,
    c.EXPIRED_DATE,
    DATEDIFF(
        day,
        CAST(GETDATE() AS date),
        CONVERT(date, CAST(c.EXPIRED_DATE AS varchar(8)), 112)
    )                           AS days_to_expire,
    SUM(c.REMAIN_QTY_LOT)       AS remain_qty_lot,
    SUM(c.REMAIN_VALUE_LOT)     AS remain_value_lot
FROM  CARD c WITH (NOLOCK)
LEFT JOIN DRUG_GN g WITH (NOLOCK) ON g.WORKING_CODE = c.WORKING_CODE
WHERE c.STOCK_ID          = @P1
  AND LEFT(dbo.ce2cymd(c.OPERATE_DATE), 6) <= @P2
  AND c.REMAIN_QTY_LOT    > 0
  AND c.EXPIRED_DATE       IS NOT NULL
  AND c.EXPIRED_DATE       > 0
  AND DATEDIFF(
        day,
        CAST(GETDATE() AS date),
        CONVERT(date, CAST(c.EXPIRED_DATE AS varchar(8)), 112)
      ) <= @P3
GROUP BY
    c.WORKING_CODE, g.DRUG_NAME, g.IS_ED,
    c.LOT_NO, c.EXPIRED_DATE
ORDER BY c.EXPIRED_DATE ASC
";

// ─────────────────────────────────────────────
// Row extraction helpers
// ─────────────────────────────────────────────

/// Try to get a f64 from a tiberius Row column, handling various SQL numeric types.
fn get_f64(row: &Row, col: &str) -> Option<f64> {
  if let Ok(Some(v)) = row.try_get::<f64, _>(col) {
    return Some(v);
  }
  if let Ok(Some(v)) = row.try_get::<f32, _>(col) {
    return Some(f64::from(v));
  }
  if let Ok(Some(v)) = row.try_get::<i32, _>(col) {
    return Some(f64::from(v));
  }
  if let Ok(Some(v)) = row.try_get::<i64, _>(col) {
    return Some(v as f64);
  }
  if let Ok(Some(v)) = row.try_get::<i16, _>(col) {
    return Some(f64::from(v));
  }
  if let Ok(Some(v)) = row.try_get::<Numeric, _>(col) {
    let s = format!("{v}");
    if let Ok(f) = s.parse::<f64>() {
      return Some(f);
    }
  }
  None
}

/// Get a string value from a row.
fn get_str(row: &Row, col: &str) -> String {
  row
    .try_get::<&str, _>(col)
    .ok()
    .flatten()
    .unwrap_or("")
    .to_string()
}

/// Get an optional string value from a row.
fn get_opt_str(row: &Row, col: &str) -> Option<String> {
  row
    .try_get::<&str, _>(col)
    .ok()
    .flatten()
    .map(std::string::ToString::to_string)
}

/// Get an optional i32 from a row.
fn get_opt_i32(row: &Row, col: &str) -> Option<i32> {
  if let Ok(Some(v)) = row.try_get::<i32, _>(col) {
    return Some(v);
  }
  if let Ok(Some(v)) = row.try_get::<i64, _>(col) {
    return Some(v as i32);
  }
  if let Ok(Some(v)) = row.try_get::<i16, _>(col) {
    return Some(i32::from(v));
  }
  None
}

/// Get an optional i64 from a row.
fn get_opt_i64(row: &Row, col: &str) -> Option<i64> {
  if let Ok(Some(v)) = row.try_get::<i64, _>(col) {
    return Some(v);
  }
  if let Ok(Some(v)) = row.try_get::<i32, _>(col) {
    return Some(i64::from(v));
  }
  if let Ok(Some(v)) = row.try_get::<i16, _>(col) {
    return Some(i64::from(v));
  }
  if let Ok(Some(v)) = row.try_get::<Numeric, _>(col) {
    let s = format!("{v}");
    if let Ok(i) = s.parse::<i64>() {
      return Some(i);
    }
  }
  None
}

// ─────────────────────────────────────────────
// Warehouse info
// ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Warehouse {
  pub dept_id: String,
  pub dept_name: String,
}

// ─────────────────────────────────────────────
// Public query functions
// ─────────────────────────────────────────────

/// Fetch all drug warehouses.
pub async fn fetch_warehouses(
  conn: &mut bb8::PooledConnection<'_, bb8_tiberius::ConnectionManager>,
) -> Result<Vec<Warehouse>, String> {
  let stream = conn
    .simple_query(SQL_GET_WAREHOUSES)
    .await
    .map_err(|e| format!("warehouse query error: {e}"))?;

  let rows = stream
    .into_first_result()
    .await
    .map_err(|e| format!("warehouse fetch error: {e}"))?;

  let result: Vec<Warehouse> = rows
    .iter()
    .map(|row| Warehouse {
      dept_id: get_str(row, "DEPT_ID"),
      dept_name: get_str(row, "DEPT_NAME"),
    })
    .collect();

  Ok(result)
}

/// Result of `fetch_drug_movements` — rows + day counts.
pub struct DrugMovementResult {
  pub rows: Vec<RawDrugRow>,
  pub display_days: i32,
  pub rolling_days: i32,
}

/// Fetch drug movement data with rolling DIS for DOS calculation.
pub async fn fetch_drug_movements(
  conn: &mut bb8::PooledConnection<'_, bb8_tiberius::ConnectionManager>,
  stock_id: &str,
  date_from: NaiveDate,
  date_to: NaiveDate,
  rolling_months: i32,
) -> Result<DrugMovementResult, String> {
  // Display period yyyymm
  let ym1 = date_to_ym(date_from);
  let ym_display_to = date_to_ym(date_to);
  let ym_fw = prev_month_ym(&ym1);

  // Rolling window
  let roll_start_date = rolling_start(date_to, rolling_months);
  let ym_roll_from = date_to_ym(roll_start_date);

  // Actual day counts
  let day_start_display = first_day_of_month(date_from);
  let day_end_display = last_day_of_month(date_to);
  let display_days = (day_end_display - day_start_display).num_days().max(1) as i32 + 1;

  let day_start_roll = first_day_of_month(roll_start_date);
  let rolling_days = (day_end_display - day_start_roll).num_days().max(1) as i32 + 1;

  // Execute the parameterized query
  // Parameters: @P1..@P11
  let stream = conn
    .query(
      SQL_GET_DRUG_MOVEMENTS,
      &[
        &ym_fw as &dyn tiberius::ToSql,         // @P1 FW_QTY
        &ym_fw as &dyn tiberius::ToSql,         // @P2 FW_VALUE
        &ym_display_to as &dyn tiberius::ToSql, // @P3 RM_QTY
        &ym_display_to as &dyn tiberius::ToSql, // @P4 RM_VALUE
        &stock_id as &dyn tiberius::ToSql,      // @P5 display CARD stock_id
        &ym1 as &dyn tiberius::ToSql,           // @P6 display ym1
        &ym_display_to as &dyn tiberius::ToSql, // @P7 display ym2
        &stock_id as &dyn tiberius::ToSql,      // @P8 rolling CARD stock_id
        &ym_roll_from as &dyn tiberius::ToSql,  // @P9 rolling ym_from
        &ym_display_to as &dyn tiberius::ToSql, // @P10 rolling ym_to
        &stock_id as &dyn tiberius::ToSql,      // @P11 INV_MD WHERE
      ],
    )
    .await
    .map_err(|e| format!("drug movements query error: {e}"))?;

  let rows = stream
    .into_first_result()
    .await
    .map_err(|e| format!("drug movements fetch error: {e}"))?;

  let result: Vec<RawDrugRow> = rows
    .iter()
    .map(|row| RawDrugRow {
      WORKING_CODE: get_str(row, "WORKING_CODE"),
      DRUG_NAME: get_opt_str(row, "DRUG_NAME"),
      NLEM: get_opt_str(row, "NLEM"),
      LAST_PACK_RATIO: get_f64(row, "LAST_PACK_RATIO"),
      FW_QTY: get_f64(row, "FW_QTY"),
      RCV_QTY: get_f64(row, "RCV_QTY"),
      DIS_QTY: get_f64(row, "DIS_QTY"),
      RM_QTY: get_f64(row, "RM_QTY"),
      RM_VALUE: get_f64(row, "RM_VALUE"),
      DIS_VALUE: get_f64(row, "DIS_VALUE"),
      QTY_ON_HAND: get_f64(row, "QTY_ON_HAND"),
      ROLLING_DIS_QTY: get_f64(row, "ROLLING_DIS_QTY"),
    })
    .collect();

  Ok(DrugMovementResult {
    rows: result,
    display_days,
    rolling_days,
  })
}

/// Fetch the last purchase cost for a drug (fallback when rm_value = 0).
pub async fn fetch_last_cost(
  conn: &mut bb8::PooledConnection<'_, bb8_tiberius::ConnectionManager>,
  working_code: &str,
) -> Result<Option<(f64, f64)>, String> {
  let stream = conn
    .query(SQL_GET_LAST_COST, &[&working_code as &dyn tiberius::ToSql])
    .await
    .map_err(|e| format!("last cost query error: {e}"))?;

  let rows = stream
    .into_first_result()
    .await
    .map_err(|e| format!("last cost fetch error: {e}"))?;

  if let Some(row) = rows.first() {
    let cost = get_f64(row, "LAST_BUY_COST").unwrap_or(0.0);
    let ratio = get_f64(row, "LAST_PACK_RATIO").unwrap_or(1.0);
    Ok(Some((cost, ratio)))
  } else {
    Ok(None)
  }
}

/// Fetch lots that are near expiry or already expired.
pub async fn fetch_near_expiry(
  conn: &mut bb8::PooledConnection<'_, bb8_tiberius::ConnectionManager>,
  stock_id: &str,
  date_to: NaiveDate,
  near_expiry_days: i32,
) -> Result<Vec<RawExpiryLotRow>, String> {
  let ym2 = date_to_ym(date_to);

  let stream = conn
    .query(
      SQL_GET_NEAR_EXPIRY,
      &[
        &stock_id as &dyn tiberius::ToSql,
        &ym2 as &dyn tiberius::ToSql,
        &near_expiry_days as &dyn tiberius::ToSql,
      ],
    )
    .await
    .map_err(|e| format!("near expiry query error: {e}"))?;

  let rows = stream
    .into_first_result()
    .await
    .map_err(|e| format!("near expiry fetch error: {e}"))?;

  let result: Vec<RawExpiryLotRow> = rows
    .iter()
    .map(|row| RawExpiryLotRow {
      WORKING_CODE: get_str(row, "WORKING_CODE"),
      LOT_NO: get_opt_str(row, "LOT_NO"),
      EXPIRED_DATE: get_opt_i64(row, "EXPIRED_DATE"),
      days_to_expire: get_opt_i32(row, "days_to_expire"),
      remain_qty_lot: get_f64(row, "remain_qty_lot"),
      remain_value_lot: get_f64(row, "remain_value_lot"),
    })
    .collect();

  Ok(result)
}

/// Compute a safe unit cost from last-cost data.
pub fn safe_unit_cost(cost_data: Option<(f64, f64)>) -> f64 {
  match cost_data {
    None => 0.0,
    Some((cost, ratio)) => {
      let r = if ratio <= 0.0 { 1.0 } else { ratio };
      cost / r
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::NaiveDate;

  #[test]
  fn test_date_to_ym() {
    let d = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
    assert_eq!(date_to_ym(d), "202503");

    let d2 = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
    assert_eq!(date_to_ym(d2), "202412");
  }

  #[test]
  fn test_prev_month_ym() {
    assert_eq!(prev_month_ym("202503"), "202502");
    assert_eq!(prev_month_ym("202501"), "202412");
    assert_eq!(prev_month_ym("202401"), "202312");
  }

  #[test]
  fn test_add_months() {
    let d = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
    assert_eq!(
      add_months(d, -2),
      NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
    );
    assert_eq!(
      add_months(d, -3),
      NaiveDate::from_ymd_opt(2024, 12, 1).unwrap()
    );
    assert_eq!(
      add_months(d, 1),
      NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()
    );
    assert_eq!(
      add_months(d, 0),
      NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()
    );
  }

  #[test]
  fn test_first_day_of_month() {
    let d = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
    assert_eq!(
      first_day_of_month(d),
      NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()
    );
  }

  #[test]
  fn test_last_day_of_month() {
    let d1 = NaiveDate::from_ymd_opt(2025, 2, 10).unwrap();
    assert_eq!(
      last_day_of_month(d1),
      NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
    );

    // Leap year
    let d2 = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();
    assert_eq!(
      last_day_of_month(d2),
      NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
    );

    let d3 = NaiveDate::from_ymd_opt(2025, 12, 5).unwrap();
    assert_eq!(
      last_day_of_month(d3),
      NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()
    );
  }

  #[test]
  fn test_rolling_start() {
    let date_to = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();
    // rolling_months=3 → start at Jan 2025
    let rs = rolling_start(date_to, 3);
    assert_eq!(rs, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

    // rolling_months=1 → start at Mar 2025
    let rs1 = rolling_start(date_to, 1);
    assert_eq!(rs1, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());

    // rolling_months=6 → start at Oct 2024
    let rs6 = rolling_start(date_to, 6);
    assert_eq!(rs6, NaiveDate::from_ymd_opt(2024, 10, 1).unwrap());
  }

  #[test]
  fn test_to_date_range() {
    let (from, to) = to_date_range(2568, 3, 3);
    assert_eq!(from, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
    assert_eq!(to, NaiveDate::from_ymd_opt(2025, 3, 31).unwrap());

    let (from2, to2) = to_date_range(2568, 1, 3);
    assert_eq!(from2, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(to2, NaiveDate::from_ymd_opt(2025, 3, 31).unwrap());
  }

  #[test]
  fn test_period_label() {
    assert_eq!(period_label(2568, 3, 3), "มี.ค. 2568");
    assert_eq!(period_label(2568, 1, 3), "ม.ค. – มี.ค. 2568");
  }

  #[test]
  fn test_safe_unit_cost() {
    assert_eq!(safe_unit_cost(None), 0.0);
    assert!((safe_unit_cost(Some((1000.0, 10.0))) - 100.0).abs() < 1e-9);
    assert!((safe_unit_cost(Some((1000.0, 0.0))) - 1000.0).abs() < 1e-9);
    assert!((safe_unit_cost(Some((500.0, -1.0))) - 500.0).abs() < 1e-9);
  }
}
