<template>
    <div class="grade-ring" :style="{ width: size + 'px', height: size + 'px' }">
        <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`" style="overflow: visible">
            <circle
                :cx="size / 2" :cy="size / 2" :r="r"
                fill="none"
                stroke="var(--color-hairline)"
                :stroke-width="strokeW"
            />
            <circle
                :cx="size / 2" :cy="size / 2" :r="r"
                fill="none"
                :stroke="gradeColor"
                :stroke-width="strokeW"
                stroke-linecap="round"
                :stroke-dasharray="circumference"
                :stroke-dashoffset="offset"
                :transform="`rotate(-90 ${size / 2} ${size / 2})`"
                :style="{ transition: 'stroke-dashoffset 1s cubic-bezier(0.16,1,0.3,1)' }"
            />
        </svg>
        <div class="ring-inner">
            <span class="ring-score">{{ displayScore }}</span>
            <span class="ring-grade" :style="{ color: gradeColor }">{{ grade }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import type { Grade } from '@/types';

const props = withDefaults(
  defineProps<{ score: number; grade: Grade; size?: number; strokeW?: number }>(),
  { size: 100, strokeW: 6 },
);

const GRADE_COLORS: Record<Grade, string> = {
  A: 'var(--status-ok)',
  B: 'var(--status-warn)',
  C: 'var(--status-danger)',
  D: 'var(--grade-d)',
};
const gradeColor = computed(() => GRADE_COLORS[props.grade] ?? 'var(--grade-d)');

const r = computed(() => props.size / 2 - props.strokeW / 2 - 2);
const circumference = computed(() => 2 * Math.PI * r.value);

const animScore = ref(0);
const offset = computed(() => circumference.value * (1 - animScore.value / 100));
const displayScore = computed(() => Math.round(animScore.value));

onMounted(() => {
  setTimeout(() => {
    animScore.value = props.score;
  }, 120);
});
watch(
  () => props.score,
  (v) => {
    animScore.value = v;
  },
);
</script>

<style scoped>
.grade-ring {
    position: relative;
    flex-shrink: 0;
}

.ring-inner {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    pointer-events: none;
}

.ring-score {
    font-family: var(--font-mono);
    font-size: 26px;
    font-weight: var(--font-weight-medium);
    color: var(--color-ink);
    line-height: 1;
    letter-spacing: -0.03em;
}

.ring-grade {
    font-family: var(--font-mono);
    font-size: 18px;
    font-weight: var(--font-weight-medium);
    line-height: 1;
    margin-top: 4px;
    letter-spacing: 0.05em;
}
</style>
