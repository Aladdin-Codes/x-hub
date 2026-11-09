<script setup lang="ts">
import { computed, onBeforeUnmount, ref, toRef, watch } from 'vue'
import { useFocusTrap } from '../composables/useFocusTrap'
import type { ExportFormat } from '../utils/noteExport'

/**
 * 速记随手导出弹窗（单条/整夹共用，2026-10-22 #t69）：
 * 格式三选一（Markdown / HTML / PDF）+ 确定；整夹批量时切进度态（逐条 PDF 较慢，可取消）。
 * 整夹可勾「合并为一个文件」（子树笔记写进同一文件，另存为单个文件）。
 * 单条且目标是表格笔记（kind='sheet'）时在最前多一项 Excel（.xlsx）并默认选中（2026-10-31）。
 * 导出动作由宿主（SpeednoteView）执行，本组件只管选格式与展示进度。
 */
const props = defineProps<{
  visible: boolean
  /** note = 单条导出；folder = 整夹导出 */
  kind: 'note' | 'folder'
  /** 笔记标题 / 文件夹名（展示用） */
  title: string
  /** 目标是否表格笔记（仅 kind='note' 有意义；true 时多出 Excel 格式） */
  sheet: boolean
  busy: boolean
  progress: { done: number; total: number } | null
}>()

const emit = defineEmits<{ confirm: [format: ExportFormat, merged: boolean]; cancel: []; cancelExport: [] }>()

const format = ref<ExportFormat>('markdown')
const merged = ref(false)

// 每次打开重置默认格式（表格笔记默认 Excel，其余默认 Markdown，均为最高频选择）+ 不合并（默认每条一个文件）
watch(
  () => props.visible,
  (on) => {
    if (on) {
      format.value = props.kind === 'note' && props.sheet ? 'excel' : 'markdown'
      merged.value = false
    }
  },
)

const cardRef = ref<HTMLElement | null>(null)
useFocusTrap(toRef(props, 'visible'), cardRef)

function onKeydown(e: KeyboardEvent) {
  if (!props.visible || props.busy) return
  if (e.key === 'Escape') {
    e.preventDefault()
    emit('cancel')
  } else if (e.key === 'Enter') {
    e.preventDefault()
    emit('confirm', format.value, merged.value)
  }
}

watch(
  () => props.visible,
  (on) => {
    if (typeof window === 'undefined') return
    if (on) window.addEventListener('keydown', onKeydown)
    else window.removeEventListener('keydown', onKeydown)
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  if (typeof window !== 'undefined') window.removeEventListener('keydown', onKeydown)
})

/** 表格笔记在最前多一项 Excel（整夹导出不出现——合并/逐条的表格处置是另一个话题） */
const options = computed<{ value: ExportFormat; label: string; desc: string }[]>(() => {
  const base: { value: ExportFormat; label: string; desc: string }[] = [
    { value: 'markdown', label: 'Markdown (.md)', desc: 'Portable note format. Images are saved in a nearby assets folder.' },
    { value: 'html', label: 'HTML (.html)', desc: 'A standalone web page. Images are embedded in the file.' },
    { value: 'pdf', label: 'PDF (.pdf)', desc: 'For printing and sharing; text remains selectable and searchable.' },
  ]
  if (props.kind === 'note' && props.sheet) {
    base.unshift({ value: 'excel', label: 'Excel (.xlsx)', desc: 'Spreadsheet format. Continue editing in Excel or another spreadsheet app.' })
  }
  return base
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-mask" @click.self="!busy && emit('cancel')">
      <div ref="cardRef" class="modal-card export-card" role="dialog" aria-modal="true">
        <template v-if="!busy">
          <h2 class="export-title">Export {{ kind === 'note' ? 'note' : 'folder' }}</h2>
          <p class="export-sub">{{ title }}</p>

          <div class="export-opts" role="radiogroup" aria-label="Export format">
            <label
              v-for="opt in options"
              :key="opt.value"
              class="export-opt"
              :class="{ on: format === opt.value }"
            >
              <input v-model="format" type="radio" name="export-format" :value="opt.value" />
              <span class="export-opt-label">{{ opt.label }}</span>
              <span class="export-opt-desc">{{ opt.desc }}</span>
            </label>
          </div>

          <label
            v-if="kind === 'folder'"
            class="export-opt export-merge"
            :class="{ on: merged }"
          >
            <input v-model="merged" type="checkbox" name="export-merge" />
            <span class="export-opt-label">Combine into one file</span>
            <span class="export-opt-desc">Write all notes, including subfolders, into a single file in order.</span>
          </label>

          <p class="export-hint">
            {{
              kind === 'folder'
                ? merged
                  ? 'Combine all notes in this folder tree into one file. Choose its name and location when saving.'
                  : 'Export one file per note into a subfolder named after this folder.'
                : 'Choose where to save the exported file.'
            }}
          </p>

          <div class="export-foot">
            <button type="button" class="export-btn" @click="emit('cancel')">Cancel</button>
            <button
              type="button"
              class="export-btn export-btn--primary"
              @click="emit('confirm', format, merged)"
            >
              Export
            </button>
          </div>
        </template>

        <template v-else>
          <h2 class="export-title">Exporting…</h2>
          <p class="export-sub">{{ title }}</p>
          <div class="export-progress" role="status">
            <div class="export-progress-bar">
              <div
                class="export-progress-fill"
                :style="{ width: progress ? `${Math.round((progress.done / Math.max(progress.total, 1)) * 100)}%` : '100%' }"
              />
            </div>
            <p class="export-progress-text">
              {{ progress ? `${progress.done} / ${progress.total} items` : 'Processing…' }}
            </p>
          </div>
          <div class="export-foot">
            <button type="button" class="export-btn" @click="emit('cancelExport')">Cancel export</button>
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.export-card {
  width: min(420px, calc(100vw - 48px));
}
.export-title {
  margin: 0 0 4px;
  font-size: 1rem;
  font-weight: 650;
  color: var(--text-1);
}
.export-sub {
  margin: 0 0 14px;
  font-size: 0.8125rem;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.export-opts {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 6px;
}
.export-merge {
  margin-bottom: 12px;
}
.export-opt {
  display: grid;
  grid-template-columns: auto 1fr;
  grid-template-rows: auto auto;
  column-gap: 10px;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--border-1, rgba(128, 128, 128, 0.25));
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.export-opt:hover {
  background: var(--bg-card-soft);
}
.export-opt.on {
  border-color: var(--brand-500);
  background: var(--bg-card-soft);
}
.export-opt input {
  grid-row: 1 / span 2;
  accent-color: var(--brand-500);
}
.export-opt-label {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-1);
}
.export-opt.on .export-opt-label {
  color: var(--brand-500);
}
.export-opt-desc {
  font-size: 0.75rem;
  color: var(--text-3);
  margin-top: 2px;
}
.export-hint {
  margin: 0 0 14px;
  font-size: 0.75rem;
  color: var(--text-4);
}
.export-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.export-btn {
  padding: 6px 16px;
  border: 1px solid var(--border-1, rgba(128, 128, 128, 0.25));
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-2);
  font-size: 0.8125rem;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.export-btn:hover {
  background: var(--bg-card-soft);
  color: var(--text-1);
}
.export-btn--primary {
  background: var(--brand-500);
  border-color: var(--brand-500);
  color: #fff;
}
.export-btn--primary:hover {
  background: var(--brand-600, var(--brand-500));
  color: #fff;
}
.export-progress {
  margin-bottom: 16px;
}
.export-progress-bar {
  height: 6px;
  border-radius: 999px;
  background: var(--bg-card-soft);
  overflow: hidden;
}
.export-progress-fill {
  height: 100%;
  border-radius: 999px;
  background: var(--brand-500);
  transition: width 0.25s ease;
}
.export-progress-text {
  margin: 8px 0 0;
  font-size: 0.75rem;
  color: var(--text-3);
  text-align: center;
}
</style>
