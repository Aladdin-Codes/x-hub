<script setup lang="ts">
// Workbench settings (layout, alerts, quote, and quote source).
//
// Extracted from SettingsView.vue. Settings panels load on demand so the initial
// view only needs the shell and the selected category.
import { inject, ref } from 'vue';
import { isTauri } from '../../api/tauri';
import AppSelect from '../AppSelect.vue';
import { useDashboardLayout } from '../../composables/useDashboardLayout';
import { useStore } from '../../stores/workbench';

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()
const layout = useDashboardLayout()
const emit = defineEmits<{ (e: 'open-layout-editor'): void }>()

/** 恢复为保存的默认布局（编辑器外立即生效并落盘；无快照时按钮置灰不会进来） */
function onRestoreDefaultLayout() {
  if (layout.restoreDefaultLayout()) {
    showToast('Restored the default dashboard layout.')
  } else {
    showToast('The saved default layout is unavailable. Save a new default in the layout editor.')
  }
}

function onToggleCountdownSound() {
  void store.setCountdownSound(!store.state.config.countdown_sound)
}

// ---- 时钟卡片语录（回车/失焦自动保存，清空则回退默认） ----
const clockQuote = ref(store.state.config.clock_quote ?? '')
const savedClockQuote = ref(clockQuote.value)

function commitClockQuote() {
  const value = clockQuote.value.trim()
  clockQuote.value = value
  if (value === savedClockQuote.value) return
  savedClockQuote.value = value
  void store.setClockQuote(value)
  showToast(value ? 'Clock quote updated.' : 'The clock now shows a random quote.')
}

const QUOTE_SOURCE_OPTIONS = [
  { value: 'online', label: 'Online quotes (random; falls back to local quotes offline)' },
  { value: 'local', label: 'Local quotes (built-in only)' },
] as const

const quoteSource = ref(store.state.config.quote_source ?? 'online')

function onQuoteSourceChange(value: string) {
  quoteSource.value = value
  if (!isTauri()) return
  void store.setQuoteSource(value as 'online' | 'local').then(() => {
    showToast(value === 'online' ? 'Quote source set to online.' : 'Quote source set to local.')
  })
}
</script>

<template>
        <section id="sv-sec-workbench" class="sv-sec" aria-label="Workbench">
          <h3 class="sv-sec-title">Workbench</h3>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Custom layout</span>
              <span class="setting-desc">Arrange dashboard modules and control their visibility by dragging them (clock, to-dos, prompts, and more). The recommended grid is 12 × 15. Return to the dashboard when you are done. Save the current layout as the default in the editor and restore it here at any time.</span>
            </div>
            <div class="layout-row-btns">
              <button
                class="ghost-btn data-btn"
                type="button"
                :disabled="!layout.hasSavedDefault.value"
                title="Restore the saved default dashboard layout immediately"
                @click="onRestoreDefaultLayout"
              >Restore default</button>
              <button class="ghost-btn data-btn" type="button" @click="emit('open-layout-editor')">Open layout editor</button>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Countdown alert sound</span>
              <span class="setting-desc">Play an additional sound when a countdown ends (off by default).</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.countdown_sound"
              :class="{ on: store.state.config.countdown_sound }"
              @click="onToggleCountdownSound"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Clock quote</span>
              <span class="setting-desc">A short quote shown below the dashboard clock. Leave blank to show a random quote; click the quote to see another.</span>
            </div>
            <div class="quote-edit">
              <input
                v-model="clockQuote"
                class="field-input"
                type="text"
                maxlength="50"
                placeholder="Leave blank to show a random quote"
                spellcheck="false"
                @blur="commitClockQuote"
                @keydown.enter="commitClockQuote"
              />
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Quote source</span>
              <span class="setting-desc">Choose random online quotes or built-in local quotes for the clock. Click a quote to show another.</span>
            </div>
            <AppSelect
              :model-value="quoteSource"
              :options="QUOTE_SOURCE_OPTIONS"
              aria-label="Quote source"
              class="quote-source"
              @update:model-value="onQuoteSourceChange"
            />
          </div>
        </section>
</template>

<style scoped>
/* 自定义布局行的两个并排按钮（打开编辑器 / 恢复默认布局） */
.layout-row-btns {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
/* 名言来源下拉：与上方语录输入框等宽（AppSelect 触发器通过 $attrs 接收 class，需 :deep 穿透） */
:deep(.quote-source) {
  min-width: 240px;
}
</style>
