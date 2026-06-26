<template>
    <div class="dashboard">
        <!-- ─── Control Bar (canvas-dark top band) ─── -->
        <div class="ctrl-bar">
            <div class="ctrl-inner">
                <div class="ctrl-group ctrl-warehouse">
                    <span class="ctrl-eyebrow">คลัง</span>
                    <div class="warehouse-badge">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                            stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
                            <polyline points="9 22 9 12 15 12 15 22"/>
                        </svg>
                        {{ store.stockName }}
                    </div>
                </div>

                <div class="ctrl-group">
                    <label class="ctrl-eyebrow" for="ctrl-year">ปี (พ.ศ.)</label>
                    <select id="ctrl-year" v-model="store.selectedYear" class="ctrl-select ctrl-narrow">
                        <option v-for="y in yearOptions" :key="y" :value="y">{{ y }}</option>
                    </select>
                </div>

                <div class="ctrl-group">
                    <label class="ctrl-eyebrow">เดือน</label>
                    <select v-model="store.selectedMonthFrom" class="ctrl-select ctrl-narrow">
                        <option v-for="(m, i) in thaiMonths" :key="i" :value="i + 1">{{ m }}</option>
                    </select>
                    <span class="ctrl-dash">–</span>
                    <select v-model="store.selectedMonthTo" class="ctrl-select ctrl-narrow">
                        <option v-for="(m, i) in thaiMonths" :key="i" :value="i + 1">{{ m }}</option>
                    </select>
                </div>

                <button
                    class="load-btn"
                    @click="store.loadData()"
                    :disabled="store.loadingSummary || store.loadingDrugs"
                >
                    <svg v-if="!store.loadingSummary" width="13" height="13" viewBox="0 0 24 24" fill="none"
                        stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M21 12a9 9 0 1 1-6.22-8.56"/>
                        <polyline points="21 3 21 9 15 9"/>
                    </svg>
                    <span v-if="store.loadingSummary" class="spinner" />
                    <span>{{ store.loadingSummary ? 'LOADING...' : 'LOAD DATA' }}</span>
                </button>
            </div>
        </div>

        <!-- ─── Error Banner ─── -->
        <div v-if="store.error" class="error-banner">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="8" x2="12" y2="12"/>
                <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <span>{{ store.error }}</span>
            <button class="error-dismiss" @click="store.clearError()">✕</button>
        </div>

        <!-- ─── Loading Skeleton ─── -->
        <div v-if="store.loadingSummary && !store.summary" class="skeleton-wrap">
            <div class="skel skel-hero" />
            <div class="skel-row">
                <div class="skel skel-card" v-for="n in 4" :key="n" />
            </div>
            <div class="skel skel-panel" />
        </div>

        <!-- ─── Main Content ─── -->
        <template v-else-if="store.summary">
            <!-- Hero / Summary band -->
            <section class="hero fadeUp">
                <div class="hero-ring">
                    <GradeRing :score="store.summary.avg_overall_score" :grade="store.summary.grade" :size="108" :stroke-w="6" />
                </div>
                <div class="hero-info">
                    <p class="hero-eyebrow">OVERALL GRADE</p>
                    <h1 class="hero-title">{{ store.summary.stock_name }}</h1>
                    <p class="hero-period">{{ store.periodLabel }}</p>
                    <p class="hero-count">
                        <span class="mono">{{ store.summary.total_drugs }}</span> รายการยา
                    </p>
                </div>
                <div class="hero-metrics">
                    <div class="metric-item">
                        <span class="metric-value">{{ store.summary.avg_dos !== null ? Math.round(store.summary.avg_dos) : '—' }}</span>
                        <span class="metric-label">AVG DOS</span>
                    </div>
                    <div class="metric-div" />
                    <div class="metric-item">
                        <span class="metric-value">{{ store.summary.avg_turnover !== null ? store.summary.avg_turnover.toFixed(1) : '—' }}</span>
                        <span class="metric-label">AVG TURNOVER</span>
                    </div>
                    <div class="metric-div" />
                    <div class="metric-item">
                        <span class="metric-value">{{ store.summary.dead_stock_count }}</span>
                        <span class="metric-label">DEAD STOCK</span>
                    </div>
                    <div class="metric-div" />
                    <div class="metric-item">
                        <span class="metric-value">{{ fmtValue(store.summary.dead_stock_value) }}</span>
                        <span class="metric-label">DEAD STOCK ฿</span>
                    </div>
                </div>
            </section>

            <!-- Stat tiles -->
            <section class="stat-grid fadeUp" style="animation-delay:.06s">
                <StatCard label="Stockout Risk"  :value="store.summary.stockout_count"  unit="รายการ" :icon="iconStockout"  variant="danger" :sub="`จากทั้งหมด ${store.summary.total_drugs} รายการ`" />
                <StatCard label="Low Stock"      :value="store.summary.low_stock_count"  unit="รายการ" :icon="iconLowStock"  variant="warn"   :sub="`จากทั้งหมด ${store.summary.total_drugs} รายการ`" />
                <StatCard label="ปกติ / Normal"  :value="store.summary.normal_count"     unit="รายการ" :icon="iconNormal"   variant="ok"     :sub="`จากทั้งหมด ${store.summary.total_drugs} รายการ`" />
                <StatCard label="Overstock"      :value="store.summary.overstock_count"  unit="รายการ" :icon="iconOverstock" variant="info"   :sub="`จากทั้งหมด ${store.summary.total_drugs} รายการ`" />
            </section>

            <!-- DOS Distribution panel -->
            <section class="content-wrap fadeUp" style="animation-delay:.12s">
                <div class="panel">
                    <div class="panel-header">
                        <p class="panel-eyebrow--dash">DOS DISTRIBUTION</p>
                        <h2 class="panel-title">การกระจาย Days of Supply</h2>
                    </div>
                    <div class="panel-body">
                        <div class="dos-track">
                            <div v-for="seg in dosSegments" :key="seg.key" class="dos-seg"
                                :style="{ width: store.summary.total_drugs > 0 ? (seg.count / store.summary.total_drugs * 100) + '%' : '0%', background: seg.color }"
                                :title="`${seg.label}: ${seg.count}`" />
                        </div>
                        <div class="dos-legend">
                            <div v-for="seg in dosSegments" :key="seg.key" class="dos-legend-item">
                                <span class="dos-dot" :style="{ background: seg.color }" />
                                <span class="dos-label">{{ seg.label }}</span>
                                <span class="dos-count mono">{{ seg.count }}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <!-- Issues grid (3-up) -->
            <section class="issues-grid fadeUp" style="animation-delay:.18s">
                <div class="panel">
                    <div class="panel-header">
                        <p class="panel-eyebrow--dash">TOP RISK</p>
                        <h2 class="panel-title">🔴 Stockout Risk</h2>
                    </div>
                    <div class="panel-body panel-body--list">
                        <div v-if="store.summary.top_stockout.length === 0" class="list-empty">ไม่มีรายการ</div>
                        <ul v-else class="issue-list">
                            <li v-for="d in store.summary.top_stockout" :key="d.working_code" class="issue-item" @click="goDetail(d)">
                                <span class="issue-name">{{ d.drug_name }}</span>
                                <div class="issue-meta">
                                    <span class="issue-dos mono">DOS {{ d.dos ?? '∞' }}</span>
                                    <span class="grade-tag" :class="'g-' + d.grade.toLowerCase()">{{ d.grade }}</span>
                                </div>
                            </li>
                        </ul>
                    </div>
                </div>

                <div class="panel">
                    <div class="panel-header">
                        <p class="panel-eyebrow--dash">TOP OVERSTOCK</p>
                        <h2 class="panel-title">🔵 Overstock</h2>
                    </div>
                    <div class="panel-body panel-body--list">
                        <div v-if="store.summary.top_overstock.length === 0" class="list-empty">ไม่มีรายการ</div>
                        <ul v-else class="issue-list">
                            <li v-for="d in store.summary.top_overstock" :key="d.working_code" class="issue-item" @click="goDetail(d)">
                                <span class="issue-name">{{ d.drug_name }}</span>
                                <div class="issue-meta">
                                    <span class="issue-dos mono">DOS {{ d.dos ?? '∞' }}</span>
                                    <span class="grade-tag" :class="'g-' + d.grade.toLowerCase()">{{ d.grade }}</span>
                                </div>
                            </li>
                        </ul>
                    </div>
                </div>

                <div class="panel">
                    <div class="panel-header">
                        <p class="panel-eyebrow--dash">TOP DEAD STOCK</p>
                        <h2 class="panel-title">💀 Dead Stock</h2>
                    </div>
                    <div class="panel-body panel-body--list">
                        <div v-if="store.summary.top_dead_stock.length === 0" class="list-empty">ไม่มีรายการ</div>
                        <ul v-else class="issue-list">
                            <li v-for="d in store.summary.top_dead_stock" :key="d.working_code" class="issue-item" @click="goDetail(d)">
                                <span class="issue-name">{{ d.drug_name }}</span>
                                <div class="issue-meta">
                                    <span class="issue-dos mono">{{ fmtValue(d.rm_value) }}฿</span>
                                    <span class="grade-tag" :class="'g-' + d.grade.toLowerCase()">{{ d.grade }}</span>
                                </div>
                            </li>
                        </ul>
                    </div>
                </div>
            </section>

            <!-- Drug table panel -->
            <section class="content-wrap fadeUp" style="animation-delay:.24s">
                <div class="panel">
                    <div class="panel-header">
                        <p class="panel-eyebrow--dash">ALL DRUGS</p>
                        <h2 class="panel-title">รายการยาทั้งหมด</h2>
                    </div>
                    <div class="panel-filter-row">
                        <FilterBar v-model="store.activeFilter" :filters="filterOptions" />
                    </div>
                    <DrugTable :drugs="store.filteredDrugs" @select="goDetail" />
                </div>
            </section>
        </template>

        <!-- ─── No Data State ─── -->
        <div v-else class="empty-state-full">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"
                stroke-linecap="round" stroke-linejoin="round" style="color:var(--color-hairline)">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
                <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
                <line x1="12" y1="22.08" x2="12" y2="12"/>
            </svg>
            <p class="empty-hint">เลือกคลังและช่วงเวลา แล้วกด <strong>LOAD DATA</strong></p>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import DrugTable from '@/components/DrugTable.vue';
import type { FilterOption } from '@/components/FilterBar.vue';
import FilterBar from '@/components/FilterBar.vue';
import GradeRing from '@/components/GradeRing.vue';
import StatCard from '@/components/StatCard.vue';
import { useInventoryStore } from '@/stores/inventory';
import type { DrugKpiSummary } from '@/types';

const router = useRouter();
const store = useInventoryStore();

const thaiMonths = [
  'ม.ค.',
  'ก.พ.',
  'มี.ค.',
  'เม.ย.',
  'พ.ค.',
  'มิ.ย.',
  'ก.ค.',
  'ส.ค.',
  'ก.ย.',
  'ต.ค.',
  'พ.ย.',
  'ธ.ค.',
];
const currentBe = new Date().getFullYear() + 543;
const yearOptions = Array.from({ length: 5 }, (_, i) => currentBe - i);

// ─── Icons ───
const iconStockout = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`;
const iconLowStock = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>`;
const iconNormal = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>`;
const iconOverstock = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/></svg>`;

// ─── Helpers ───
function goDetail(drug: DrugKpiSummary) {
  router.push({
    path: `/drug/${drug.working_code}`,
    query: {
      stock_id: store.stockId,
      year: String(store.selectedYear),
      mf: String(store.selectedMonthFrom),
      mt: String(store.selectedMonthTo),
    },
  });
}

function fmtValue(v: number | null | undefined): string {
  if (v == null) return '—';
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1_000) return `${(v / 1_000).toFixed(1)}K`;
  return Math.round(v).toLocaleString('th-TH');
}

// ─── Computed ───
const dosSegments = computed(() => {
  const s = store.summary;
  if (!s) return [];
  return [
    {
      key: 'stockout',
      label: 'Stockout Risk',
      count: s.stockout_count,
      color: 'var(--status-danger)',
    },
    { key: 'low', label: 'Low Stock', count: s.low_stock_count, color: 'var(--status-warn)' },
    { key: 'normal', label: 'Normal', count: s.normal_count, color: 'var(--status-ok)' },
    { key: 'overstock', label: 'Overstock', count: s.overstock_count, color: 'var(--status-info)' },
  ];
});

const filterOptions = computed<FilterOption[]>(() => {
  const s = store.summary;
  const total = store.drugs.length;
  const deadCount = store.drugs.filter((d) => d.is_dead_stock).length;
  return [
    { key: 'all', label: 'ทั้งหมด', count: total, cls: 'all' },
    { key: 'stockout_risk', label: 'Stockout', count: s?.stockout_count ?? 0, cls: 'danger' },
    { key: 'low_stock', label: 'Low Stock', count: s?.low_stock_count ?? 0, cls: 'warn' },
    { key: 'normal', label: 'ปกติ', count: s?.normal_count ?? 0, cls: 'ok' },
    { key: 'overstock', label: 'Overstock', count: s?.overstock_count ?? 0, cls: 'info' },
    { key: 'dead_stock', label: 'Dead Stock', count: deadCount, cls: 'dead' },
  ];
});

onMounted(async () => {
  await store.initStock();
  await store.loadData();
});
</script>

<style scoped>
.dashboard {
    padding-bottom: 64px;
    min-height: 100vh;
    background: var(--color-canvas);
}

.ctrl-bar {
    background: var(--color-canvas-dark);
    border-bottom: 1px solid var(--color-surface-dark-soft);
}

.ctrl-inner {
    max-width: var(--content-max-width);
    margin: 0 auto;
    display: flex;
    align-items: center;
    gap: var(--space-xl);
    padding: var(--space-lg) var(--space-3xl);
    flex-wrap: wrap;
}

.ctrl-group {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
}

.ctrl-eyebrow {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.55px;
    text-transform: uppercase;
    color: var(--color-on-dark-muted);
    white-space: nowrap;
}

.ctrl-warehouse { gap: var(--space-sm); }

.warehouse-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-surface-dark-soft);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--rounded-sm);
    color: var(--color-on-dark);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: var(--font-weight-medium);
    padding: 5px var(--space-md);
    white-space: nowrap;
    letter-spacing: 0.05em;
}

.ctrl-select {
    background: var(--color-surface-dark-soft);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: var(--rounded-sm);
    color: var(--color-on-dark);
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 5px 28px 5px var(--space-md);
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%23ffffff' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round' opacity='0.4'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    cursor: pointer;
}

.ctrl-select:focus { outline: none; border-color: rgba(255,255,255,0.3); }
.ctrl-narrow { width: 88px; }

.ctrl-dash {
    color: var(--color-on-dark-muted);
    font-size: 12px;
}

.load-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 7px var(--space-2xl);
    margin-left: auto;
    background: var(--color-on-dark);
    color: var(--color-canvas-dark);
    border: none;
    border-radius: var(--rounded-sm);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.08px;
    text-transform: uppercase;
    cursor: pointer;
    transition: opacity var(--dur-fast) var(--ease);
    white-space: nowrap;
}

.load-btn:hover:not(:disabled) { opacity: var(--opacity-hover); }
.load-btn:disabled { opacity: var(--opacity-disabled); cursor: not-allowed; }

.error-banner {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    max-width: var(--content-max-width);
    margin: var(--space-xl) auto 0;
    padding: var(--space-lg) var(--space-2xl);
    background: var(--status-danger-bg);
    border: 1px solid rgba(220, 38, 38, 0.2);
    border-radius: var(--rounded-sm);
    color: var(--status-danger);
    font-size: 14px;
}

.error-dismiss {
    margin-left: auto;
    background: none;
    border: none;
    color: var(--status-danger);
    cursor: pointer;
    font-size: 14px;
    opacity: var(--opacity-muted);
    transition: opacity var(--dur-fast);
}
.error-dismiss:hover { opacity: 1; }

.skeleton-wrap {
    max-width: var(--content-max-width);
    margin: 0 auto;
    padding: var(--space-3xl) var(--space-3xl);
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
}

.skel {
    background: linear-gradient(90deg, var(--color-hairline) 25%, var(--color-elevated-bg) 50%, var(--color-hairline) 75%);
    background-size: 400% 100%;
    border-radius: var(--rounded-sm);
    animation: shimmer 1.6s ease infinite;
}

.skel-hero  { height: 120px; }
.skel-row   { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--space-lg); }
.skel-card  { height: 110px; }
.skel-panel { height: 260px; }

.content-wrap {
    max-width: var(--content-max-width);
    margin: var(--space-3xl) auto 0;
    padding: 0 var(--space-3xl);
}

.hero {
    max-width: var(--content-max-width);
    margin: var(--space-3xl) auto 0;
    padding: var(--space-2xl) var(--space-3xl);
    display: flex;
    align-items: center;
    gap: var(--space-3xl);
    background: var(--color-canvas);
    border: 1px solid var(--color-hairline);
    border-radius: var(--rounded-sm);
}

.hero-ring { flex-shrink: 0; }

.hero-info {
    flex: 1;
    min-width: 0;
}

.hero-eyebrow {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.55px;
    text-transform: uppercase;
    color: var(--color-body);
    margin-bottom: var(--space-xs);
}

.hero-title {
    font-size: 22px;
    font-weight: var(--font-weight-medium);
    color: var(--color-ink);
    letter-spacing: -0.022em;
    margin: 0;
}

.hero-period {
    font-size: 14px;
    color: var(--color-body);
    margin: var(--space-xs) 0 0;
}

.hero-count {
    font-size: 13px;
    color: var(--color-body);
    margin: var(--space-xs) 0 0;
}

.hero-metrics {
    display: flex;
    align-items: center;
    gap: var(--space-3xl);
    flex-shrink: 0;
}

.metric-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
}

.metric-value {
    font-family: var(--font-mono);
    font-size: 22px;
    font-weight: var(--font-weight-medium);
    color: var(--color-ink);
    line-height: 1;
    letter-spacing: -0.03em;
}

.metric-label {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.55px;
    text-transform: uppercase;
    color: var(--color-body);
}

.metric-div {
    width: 1px;
    height: 36px;
    background: var(--color-hairline);
}

.stat-grid {
    max-width: var(--content-max-width);
    margin: var(--space-3xl) auto 0;
    padding: 0 var(--space-3xl);
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-lg);
}

.panel-eyebrow--dash--dash {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.55px;
    text-transform: uppercase;
    color: var(--color-body);
    margin: 0 0 var(--space-xs);
}

.panel-body--list { padding: var(--space-lg) var(--space-md); }

.panel-filter-row {
    padding: var(--space-lg) var(--space-2xl);
    border-bottom: 1px solid var(--color-hairline);
}

.dos-track {
    display: flex;
    height: 16px;
    border-radius: var(--rounded-xs);
    overflow: hidden;
    gap: 2px;
}

.dos-seg {
    transition: width var(--dur-slow) var(--ease-spring);
    min-width: 2px;
}

.dos-legend {
    display: flex;
    gap: var(--space-2xl);
    margin-top: var(--space-lg);
    flex-wrap: wrap;
}

.dos-legend-item {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
}

.dos-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
}

.dos-label {
    font-size: 13px;
    color: var(--color-body);
}

.dos-count {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-ink);
}

.issues-grid {
    max-width: var(--content-max-width);
    margin: var(--space-3xl) auto 0;
    padding: 0 var(--space-3xl);
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-lg);
}

.issue-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
}

.issue-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    padding: 10px var(--space-md);
    border-radius: var(--rounded-xs);
    cursor: pointer;
    transition: background var(--dur-fast);
    border-bottom: 1px solid var(--color-hairline);
}

.issue-item:last-child { border-bottom: none; }
.issue-item:hover { background: var(--color-hover-bg); }

.issue-name {
    font-size: 13px;
    color: var(--color-ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
}

.issue-meta {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-shrink: 0;
}

.issue-dos {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-body);
}

.grade-tag {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: var(--font-weight-medium);
    width: 22px;
    height: 22px;
    border-radius: var(--rounded-xs);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    text-transform: uppercase;
}

.g-a { background: var(--grade-a-bg); color: var(--grade-a); }
.g-b { background: var(--grade-b-bg); color: var(--grade-b); }
.g-c { background: var(--grade-c-bg); color: var(--grade-c); }
.g-d { background: var(--grade-d-bg); color: var(--grade-d); }

.list-empty {
    text-align: center;
    padding: var(--space-3xl) var(--space-lg);
    color: var(--color-body);
    font-size: 13px;
}

.empty-state-full {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-lg);
    padding: 120px var(--space-2xl);
    text-align: center;
    color: var(--color-body);
    font-size: 15px;
}

.empty-hint strong {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08px;
    text-transform: uppercase;
    color: var(--color-ink);
}

@media (max-width: 1100px) {
    .stat-grid { grid-template-columns: repeat(2, 1fr); }
    .issues-grid { grid-template-columns: repeat(2, 1fr); }
    .issues-grid > .panel:last-child { grid-column: span 2; }
    .hero-metrics { gap: var(--space-xl); }
}

@media (max-width: 860px) {
    .hero { flex-direction: column; align-items: flex-start; gap: var(--space-xl); }
    .hero-metrics { width: 100%; justify-content: space-between; }
    .issues-grid { grid-template-columns: 1fr; }
    .issues-grid > .panel:last-child { grid-column: span 1; }
}

@media (max-width: 640px) {
    .ctrl-inner { padding: var(--space-md) var(--space-lg); gap: var(--space-md); }
    .load-btn { width: 100%; justify-content: center; }
    .stat-grid { grid-template-columns: 1fr 1fr; padding: 0 var(--space-lg); gap: var(--space-sm); }
    .issues-grid { padding: 0 var(--space-lg); }
    .content-wrap { padding: 0 var(--space-lg); }
    .hero { padding: var(--space-lg); }
    .hero-metrics { flex-wrap: wrap; gap: var(--space-md); }
    .metric-div { display: none; }
    .metric-item { flex: 1; min-width: 60px; }
}
</style>
