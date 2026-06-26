<template>
    <div class="drug-table-wrap">
        <!-- Toolbar -->
        <div class="table-toolbar">
            <div class="search-box">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <path d="m21 21-4.35-4.35"/>
                </svg>
                <input v-model="search" placeholder="ค้นหาชื่อยา / รหัสยา..." class="search-input" />
            </div>
            <div class="table-meta"><span>{{ filtered.length.toLocaleString('th-TH') }} รายการ</span></div>
        </div>

        <!-- Table -->
        <div class="table-scroll">
            <table class="drug-table">
                <thead>
                    <tr>
                        <th @click="doSort('drug_name')" class="sortable">ชื่อยา</th>
                        <th>บัญชี</th>
                        <th @click="doSort('rm_qty')" class="sortable num">คงเหลือ</th>
                        <th @click="doSort('dos')" class="sortable num">DOS</th>
                        <th class="num">สถานะ</th>
                        <th @click="doSort('turnover_rate')" class="sortable num">Turnover</th>
                        <th @click="doSort('overall_score')" class="sortable num">Score</th>
                        <th class="num">Grade</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="drug in paginated" :key="drug.working_code" class="drug-row" @click="$emit('select', drug)">
                        <td class="drug-name-td">
                            <div class="drug-name-cell">
                                <span class="drug-name">{{ drug.drug_name }}</span>
                                <span v-if="drug.is_dead_stock" class="dead-badge">DEAD</span>
                            </div>
                        </td>
                        <td><span class="nlem-badge">{{ drug.nlem || '—' }}</span></td>
                        <td class="num mono">{{ fmt(drug.rm_qty) }}</td>
                        <td class="num mono">
                            <span v-if="drug.dos !== null">{{ drug.dos }}</span>
                            <span v-else class="muted">∞</span>
                        </td>
                        <td class="num">
                            <span class="status-chip" :class="drug.dos_status ?? ''">{{ statusLabel(drug.dos_status) }}</span>
                        </td>
                        <td class="num mono">{{ drug.turnover_rate !== null ? drug.turnover_rate?.toFixed(2) : '—' }}</td>
                        <td class="num">
                            <div class="score-cell">
                                <div class="score-bar-bg">
                                    <div class="score-bar-fill" :style="{ width: drug.overall_score + '%', background: gradeColor(drug.grade) }" />
                                </div>
                                <span class="mono">{{ drug.overall_score }}</span>
                            </div>
                        </td>
                        <td class="num">
                            <span class="grade-badge" :class="'grade-' + drug.grade.toLowerCase()">{{ drug.grade }}</span>
                        </td>
                    </tr>
                    <tr v-if="filtered.length === 0">
                        <td colspan="8" class="empty-row">ไม่พบรายการ</td>
                    </tr>
                </tbody>
            </table>
        </div>

        <!-- Pagination -->
        <div v-if="totalPages > 1" class="pagination">
            <button @click="page = Math.max(1, page - 1)" :disabled="page === 1" class="pg-btn">‹</button>
            <span class="mono">{{ page }} / {{ totalPages }}</span>
            <button @click="page = Math.min(totalPages, page + 1)" :disabled="page === totalPages" class="pg-btn">›</button>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { DosStatus, DrugKpiSummary, Grade } from '@/types';

const props = defineProps<{ drugs: DrugKpiSummary[] }>();
defineEmits<{ select: [drug: DrugKpiSummary] }>();

const search = ref('');
const sortKey = ref<keyof DrugKpiSummary>('overall_score');
const sortAsc = ref(false);
const page = ref(1);
const PER_PAGE = 20;

const filtered = computed(() => {
  let d = props.drugs;
  if (search.value) {
    const q = search.value.toLowerCase();
    d = d.filter((x) => x.drug_name.toLowerCase().includes(q));
  }
  return [...d].sort((a, b) => {
    const av = (a[sortKey.value] as number) ?? -Infinity;
    const bv = (b[sortKey.value] as number) ?? -Infinity;
    return sortAsc.value ? (av > bv ? 1 : -1) : av < bv ? 1 : -1;
  });
});

const totalPages = computed(() => Math.max(1, Math.ceil(filtered.value.length / PER_PAGE)));
const paginated = computed(() => {
  const s = (page.value - 1) * PER_PAGE;
  return filtered.value.slice(s, s + PER_PAGE);
});

watch(filtered, () => {
  page.value = 1;
});

function doSort(key: string) {
  const k = key as keyof DrugKpiSummary;
  if (sortKey.value === k) sortAsc.value = !sortAsc.value;
  else {
    sortKey.value = k;
    sortAsc.value = false;
  }
}

const fmt = (n: number | null) => n?.toLocaleString('th-TH') ?? '—';

const statusLabel = (s: DosStatus | null): string => {
  const map: Record<DosStatus, string> = {
    stockout_risk: 'Stockout',
    low_stock: 'Low',
    normal: 'Normal',
    overstock: 'Over',
  };
  return s ? (map[s] ?? '—') : '—';
};

const gradeColor = (g: Grade): string => {
  const map: Record<Grade, string> = {
    A: 'var(--status-ok)',
    B: 'var(--status-warn)',
    C: 'var(--status-danger)',
    D: 'var(--grade-d)',
  };
  return map[g] ?? 'var(--grade-d)';
};
</script>

<style scoped>
.drug-table-wrap {
    display: flex;
    flex-direction: column;
}

.table-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg) var(--space-2xl);
    border-bottom: 1px solid var(--color-hairline);
}

.search-box {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-body);
}

.search-input {
    background: none;
    border: none;
    outline: none;
    color: var(--color-ink);
    font-family: var(--font-display);
    font-size: 14px;
    width: 240px;
}

.search-input::placeholder { color: var(--color-body); }

.table-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-body);
    letter-spacing: 0.05px;
}

.table-scroll { overflow-x: auto; }

.drug-table {
    width: 100%;
    border-collapse: collapse;
}

.drug-table th {
    padding: var(--space-md) var(--space-lg);
    text-align: left;
    background: var(--color-hairline);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: var(--font-weight-medium);
    text-transform: uppercase;
    letter-spacing: 0.55px;
    color: var(--color-body);
    white-space: nowrap;
    user-select: none;
    border-bottom: 1px solid var(--color-hairline);
}

.drug-table th.num { text-align: right; }
.drug-table th.sortable { cursor: pointer; }
.drug-table th.sortable:hover { color: var(--color-ink); }

.drug-row { cursor: pointer; }
.drug-row:hover td { background: var(--color-hover-bg); }
.drug-row:hover td:first-child { border-left: 2px solid var(--color-ink); padding-left: calc(var(--space-lg) - 2px); }

.drug-table td {
    padding: var(--space-md) var(--space-lg);
    font-size: 13.5px;
    border-bottom: 1px solid var(--color-hairline);
    border-left: 2px solid transparent;
    vertical-align: middle;
    color: var(--color-ink);
}

.drug-table td.num { text-align: right; }

.drug-name-cell {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    overflow: hidden;
}

.drug-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.dead-badge {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: var(--font-weight-medium);
    padding: 2px 6px;
    border-radius: var(--rounded-xs);
    background: var(--status-danger-bg);
    color: var(--status-danger);
    border: 1px solid rgba(220, 38, 38, 0.25);
    flex-shrink: 0;
    letter-spacing: var(--tracking-mono);
    text-transform: uppercase;
}

.nlem-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-body);
}

.status-chip {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: var(--font-weight-medium);
    padding: 3px 8px;
    border-radius: var(--rounded-sm);
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: var(--tracking-mono);
    background: var(--color-hairline);
    color: var(--color-body);
    border: 1px solid var(--color-hairline);
}

.status-chip.stockout_risk { background: var(--status-danger-bg); color: var(--status-danger); border-color: rgba(220,38,38,.2); }
.status-chip.low_stock     { background: var(--status-warn-bg);   color: var(--status-warn);   border-color: rgba(217,119,6,.2); }
.status-chip.normal        { background: var(--status-ok-bg);     color: var(--status-ok);     border-color: rgba(22,163,74,.2); }
.status-chip.overstock     { background: var(--status-info-bg);   color: var(--status-info);   border-color: rgba(37,99,235,.2); }

.score-cell { display: flex; align-items: center; gap: 8px; justify-content: flex-end; }
.score-bar-bg { width: 48px; height: 3px; background: var(--color-hairline); border-radius: 2px; }
.score-bar-fill { height: 100%; border-radius: 2px; transition: width var(--dur-slow) var(--ease); }

.grade-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: var(--font-weight-medium);
    width: 28px;
    height: 28px;
    border-radius: var(--rounded-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.grade-a { background: var(--grade-a-bg); color: var(--grade-a); border: 1px solid rgba(22,163,74,.25); }
.grade-b { background: var(--grade-b-bg); color: var(--grade-b); border: 1px solid rgba(217,119,6,.25); }
.grade-c { background: var(--grade-c-bg); color: var(--grade-c); border: 1px solid rgba(220,38,38,.25); }
.grade-d { background: var(--grade-d-bg); color: var(--grade-d); border: 1px solid var(--color-hairline); }

.empty-row {
    text-align: center;
    color: var(--color-body);
    font-size: 14px;
    padding: 48px 40px;
}

.muted { color: var(--color-body); }

.pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 18px;
    padding: var(--space-lg);
    border-top: 1px solid var(--color-hairline);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-body);
}

.pg-btn {
    background: var(--color-canvas);
    border: 1px solid var(--color-hairline);
    color: var(--color-ink);
    border-radius: var(--rounded-sm);
    width: 30px;
    height: 30px;
    cursor: pointer;
    display: grid;
    place-items: center;
    font-size: 16px;
    transition: border-color var(--dur-fast);
}

.pg-btn:hover:not(:disabled) { border-color: var(--color-ink); }
.pg-btn:disabled { opacity: 0.3; cursor: not-allowed; }
</style>
