<template>
    <div class="dos-bar-wrap">
        <div class="dos-track">
            <div class="zone-marker" style="left:11.7%"><span>7</span></div>
            <div class="zone-marker" style="left:25%"><span>15</span></div>
            <div class="zone-marker" style="left:100%"><span>60</span></div>
            <div class="dos-fill" :style="{ width: fillPct + '%', background: color }" />
        </div>
        <div class="dos-labels">
            <span class="dos-status-label" :style="{ color }">{{ label }}</span>
            <span class="dos-value mono">{{ dosDisplay }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { DosStatus } from '@/types';

const props = withDefaults(defineProps<{ dos: number | null; status: DosStatus }>(), {
  dos: null,
  status: 'normal',
});

const MAX_VIS = 120;

const fillPct = computed(() => {
  if (props.dos === null) return 0;
  return Math.min((props.dos / MAX_VIS) * 100, 100);
});

const colorMap: Record<DosStatus, string> = {
  stockout_risk: 'var(--status-danger)',
  low_stock: 'var(--status-warn)',
  normal: 'var(--status-ok)',
  overstock: 'var(--status-info)',
};
const color = computed(() => colorMap[props.status] || 'var(--color-body)');

const labelMap: Record<DosStatus, string> = {
  stockout_risk: 'เสี่ยง Stockout',
  low_stock: 'สต็อกน้อย',
  normal: 'ปกติ',
  overstock: 'เกินกำหนด',
};
const label = computed(() => labelMap[props.status] || '-');

const dosDisplay = computed(() => (props.dos === null ? 'ไม่มีการจ่าย' : `${props.dos} วัน`));
</script>

<style scoped>
.dos-bar-wrap {
    width: 100%;
}

.dos-track {
    position: relative;
    height: 5px;
    background: var(--color-hairline);
    border-radius: 3px;
    overflow: visible;
    margin-bottom: 6px;
}

.dos-fill {
    height: 100%;
    border-radius: 3px;
    transition: width var(--dur-spring) var(--ease-spring);
    opacity: var(--opacity-hover);
}

.zone-marker {
    position: absolute;
    top: -2px;
    width: 1px;
    height: 9px;
    background: var(--color-overlay-bg);
}

.zone-marker span {
    position: absolute;
    top: 11px;
    left: 50%;
    transform: translateX(-50%);
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--color-body);
    white-space: nowrap;
}

.dos-labels {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 14px;
}

.dos-status-label {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: var(--font-weight-medium);
    letter-spacing: 0.04em;
}

.dos-value {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-body);
}
</style>
