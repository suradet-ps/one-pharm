# OnePharm

```
 ██████╗ ███╗   ██╗███████╗██████╗ ██╗  ██╗ █████╗ ██████╗ ███╗   ███╗
██╔═══██╗████╗  ██║██╔════╝██╔══██╗██║  ██║██╔══██╗██╔══██╗████╗ ████║
██║   ██║██╔██╗ ██║█████╗  ██████╔╝███████║███████║██████╔╝██╔████╔██║
██║   ██║██║╚██╗██║██╔══╝  ██╔═══╝ ██║  ██║██╔══██║██╔══██╗██║╚██╔╝██║
╚██████╔╝██║ ╚████║███████╗██║     ██║  ██║██║  ██║██║  ██║██║ ╚═╝ ██║
 ╚═════╝ ╚═╝  ╚═══╝╚══════╝╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝
```

---

## ◆ PULSE

A stockroom does not fail suddenly; it fails quietly, day by day, in
days of supply. OnePharm evaluates pharmacy stock efficiency at a
glance: warehouse, year (B.E.), and month in, and a scored answer out -
overall grade A to D, the DOS distribution (stockout, low, normal,
overstock), the top three issues, and a weighted KPI breakdown per drug
with expiry lots tracked. Read from INVS SQL Server, read-only, on a
desktop app that lives on the pharmacy's own machine.

| Grade ▣ | DOS ▣ | KPIs ▣ | Expiry ▣ |
|---|---|---|---|

*v0.1.0 - the stock verdict is computed, never guessed.*

> Built with Tauri 2 + Vue 3, read from SQL Server through `tiberius`
> and `bb8`, scored by `kpi-core` - 23 tests hold the math honest.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

Three tools, one launch.

```
⟫ npm install
⟫ cd src-tauri && cargo check
⟫ cd .. && npx tauri dev
```

The release artifact: `⟫ npx tauri build` - installers land in
`src-tauri/target/release/bundle/`.

<details>
<summary>Prerequisites</summary>

- [Rust](https://rustup.rs) (1.85+)
- Node.js 20.19+ or 22.12+
- SQL Server with the INVS database reachable over TCP/IP (port 1433)

</details>

---

## ◆ ANATOMY

One pool, three crates, a scoring system with no hidden weights.

- **Connects** - `bb8` + `bb8-tiberius` hold the SQL Server pool over
  TDS - no ODBC, no driver installs; the connection is read-only by
  design.
- **Scores** - `kpi-core` computes the verdicts with fixed, published
  weights: Turnover Rate 30%, Days of Supply 25%, Dead Stock 20%,
  Expiry Risk 15%, Stock Accuracy 10%. Twenty-three unit tests pin the
  math down.
- **Classifies** - DOS under 7 days is Stockout Risk, 7-14 Low, 15-60
  Normal, over 60 Overstock; dead stock is stock with receipts but no
  issue for 60 days; expiry is graded Expired / Critical / Warning.
- **Grades** - the weighted score becomes a letter: A at 80, B at 60,
  C at 40, D below - one glance, one verdict.
- **Details** - each drug opens its own card: five KPI breakdowns, the
  movement flow (Opening, Receipt, Issue, Closing), the weight math,
  and expiry lots with their own countdown.
- **Remembers** - settings persist as JSON per OS (`%APPDATA%`, `~/Library`,
  `~/.local/share`) - connection, rolling months, expiry warnings.

---

## ◆ RITUALS

**The core ceremony** - the monthly stock review:

1. Open OnePharm and connect to INVS. One configuration, remembered.
2. Choose warehouse, year (B.E.), and month. The dashboard answers:
   grade, DOS distribution, top three issues.
3. Filter to the drug that matters; its KPI card shows the five scores,
   the movement flow, and the expiry countdown.
4. Decide with the numbers - stockout, overstock, or dead stock each
   have a row in the table and a face in the chart.

**The ceremony of the weight** - no score is a black box. The weights
are printed in the settings, the math is unit-tested, and the grade
traces to five named KPIs.

**The ceremony of restraint** - OnePharm reads the INVS database and
never writes a row. A dedicated SQL user is recommended over `sa`, and
the connection never leaves the machine.

---

## ◆ ECHOES

**Where this artifact is heading**

```
connect ▸ tiberius + bb8 pool, TDS protocol, read-only ────────────── ▸ sealed
score    ▸ 5 weighted KPIs, 23 tests, grades A-D ──────────────────── ▸ sealed
classify ▸ DOS bands, dead stock, expiry tiers ────────────────────── ▸ sealed
detail   ▸ per-drug KPI card, movement flow, expiry lots ──────────── ▸ sealed
settings ▸ JSON persistence, connection test ──────────────────────── ▸ sealed
```

**Raising the artifact** - the workspace notes live in
`docs/cargo-workspace.md`; the security posture in `docs/security.md`;
the design language in `DESIGN.md`. Gates: `cargo fmt`, `cargo clippy`,
`cargo test` (23 tests), `vue-tsc --noEmit`. Open an issue first to
discuss a change.

**Status** - Rust checks and Windows builds run in CI on every push.
[Watch the gates](.github/workflows).

---

```
  ─────────────────────────────────────────
   Dead stock does not announce itself.
   It waits to be counted.
  ─────────────────────────────────────────
```

Distributed under the [MIT License](LICENSE).