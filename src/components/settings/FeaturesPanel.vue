<script setup lang="ts">
// 功能大类（速达 / 剪贴板 / 联网 / 性能）
// （AI 助手 2026-10-16 自本大类拉出为独立设置项 AiPanel.vue：对话形态 + 供应商 + AI 额度）
//
// 从 SettingsView.vue 拆出（见该文件顶部说明）：设置页按大类按需加载，
// 首次打开只需外壳 + 当前大类的代码，切大类时才加载对应面板。
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { LocateFixed, MapPin, Trash2 } from 'lucide-vue-next';
import { isPlatformModel, isTauri, tauriApi, PLATFORM_ENTRY_NAME, type ChatModelConfig } from '../../api/tauri';
import AppSelect from '../AppSelect.vue';
import SudaSubcategoryManager from './SudaSubcategoryManager.vue';
import { useStore } from '../../stores/workbench';
import { reportClientError } from '../../utils/error-report';

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()

// ---- 速达（网页打开方式 + 小类管理，ADR 0011 / 0012）----
const SUDA_OPEN_MODE_OPTIONS = [
  { value: 'panel', label: 'Embedded panel (in main window)' },
  { value: 'window', label: 'Standalone browser window' },
  { value: 'system', label: 'System default browser' },
]
const sudaWebOpenMode = computed(() => {
  const mode = store.state.config.suda_web_open_mode
  return mode === 'window' || mode === 'system' ? mode : 'panel'
})
function onSudaOpenModeChange(v: string | number) {
  void store.setSudaWebOpenMode(v === 'window' || v === 'system' ? v : 'panel')
}

async function onToggleSudaPanelToolbar() {
  await store.setSudaPanelToolbar(!store.state.config.suda_panel_toolbar)
}

// ---- GIF 录制（保存目录，recorder.rs captures_dir 消费；空 = 默认数据目录\captures）----
const recorderSaveDir = computed(() => store.state.config.recorder_save_dir ?? '')

async function chooseRecorderDir() {
  if (!isTauri()) return
  const dir = await open({ multiple: false, directory: true })
  if (typeof dir !== 'string' || !dir.trim()) return
  await store.setRecorderSaveDir(dir)
  showToast('GIF save location updated')
}

async function resetRecorderDir() {
  await store.setRecorderSaveDir('')
  showToast('Default save location restored')
}

// ---- GIF 录制默认帧率（选区面板不再逐次选择，recorder_start 收到 null 由后端读本值）----
// AppSelect 的 value 是 string：档位以字符串存储、change 时转 number
const RECORDER_FPS_OPTIONS = [
  { value: '5', label: '5 fps (smallest file)' },
  { value: '10', label: '10 fps' },
  { value: '15', label: '15 fps' },
  { value: '20', label: '20 fps (recommended · smoothest)' },
]
const recorderFps = computed(() => String(store.state.config.recorder_fps ?? 20))

function onRecorderFpsChange(v: string | number) {
  const fps = Number(v)
  if (!Number.isFinite(fps)) return
  void store.setRecorderFps(fps)
  showToast(`Default GIF frame rate set to ${fps} fps`)
}

// ---- 联网 / 在线服务 ----
const weatherCityInput = ref(store.state.config.weather_city ?? '')
const weatherSaving = ref(false)

async function onToggleOnline() {
  if (!isTauri()) return
  const next = !store.state.config.online_enabled
  await store.setOnlineEnabled(next)
  showToast(next ? 'Online features enabled' : 'Online features disabled')
}

// ---- 性能 / 内存 ----
async function onToggleWebviewMem() {
  if (!isTauri()) return
  const next = !store.state.config.webview_mem_low_on_hide
  await store.setWebviewMemLowOnHide(next)
  showToast(next ? 'Enabled: memory is released when windows are hidden' : 'Disabled: hidden windows retain their memory')
}

// ---- 速记（AI 深度整理模型，约定 38）----
// note_ai_model 取值：''=自动 / 平台入口名=负载切换 / 'platform:<名>'=指定平台模型 / 其余=自备模型名。
// 来源与模型是两级 UI 筛选，落盘只存模型值一个字段；来源「自动」时模型下拉置灰。
const noteModels = ref<ChatModelConfig[]>([])
const platformModelNames = ref<string[]>([])
const platformModelsFailed = ref(false)

function providerLabel(m: ChatModelConfig): string {
  return m.provider_name?.trim() || 'Unnamed provider'
}

const noteSourceOptions = computed(() => {
  const providers: string[] = []
  for (const m of noteModels.value) {
    if (isPlatformModel(m)) continue
    const p = providerLabel(m)
    if (!providers.includes(p)) providers.push(p)
  }
  return [
    { value: 'auto', label: 'Automatic (recommended)' },
    { value: 'platform', label: 'x-hub platform (built-in)' },
    ...providers.map((p) => ({ value: p, label: p })),
  ]
})

const noteModelSource = computed(() => {
  const v = store.state.config.note_ai_model ?? ''
  if (v === PLATFORM_ENTRY_NAME || v.startsWith('platform:')) return 'platform'
  if (!v) return 'auto'
  const m = noteModels.value.find((x) => !isPlatformModel(x) && x.name === v)
  // 存的自备模型已被删除：按「自动」渲染（不悄悄改写配置，用户下次保存时自然覆盖）
  return m ? providerLabel(m) : 'auto'
})

const noteModelOptions = computed(() => {
  const src = noteModelSource.value
  if (src === 'platform') {
    const opts = [{ value: PLATFORM_ENTRY_NAME, label: 'Automatic (load balancing)' }]
    for (const n of platformModelNames.value) opts.push({ value: `platform:${n}`, label: n })
    return opts
  }
  if (src === 'auto') return []
  return noteModels.value
    .filter((m) => !isPlatformModel(m) && providerLabel(m) === src)
    .map((m) => ({ value: m.name, label: m.name }))
})

const noteModelValue = computed(() => {
  const v = store.state.config.note_ai_model ?? ''
  if (noteModelSource.value !== 'platform') {
    return noteModelOptions.value.some((o) => o.value === v) ? v : ''
  }
  // 指定的平台模型已下架 → 显示「自动（负载切换）」
  return noteModelOptions.value.some((o) => o.value === v) ? v : PLATFORM_ENTRY_NAME
})

function onNoteModelSourceChange(v: string | number) {
  const src = String(v)
  let next = ''
  if (src === 'platform') {
    next = PLATFORM_ENTRY_NAME
  } else if (src !== 'auto') {
    const first = noteModels.value.find((m) => !isPlatformModel(m) && providerLabel(m) === src)
    if (!first) return
    next = first.name
  }
  void store.setNoteAiModel(next)
}

function onNoteModelChange(v: string | number) {
  void store.setNoteAiModel(String(v))
}

async function loadNoteAiModels() {
  if (!isTauri()) return
  try {
    noteModels.value = await tauriApi.getChatModels()
  } catch {
    // 浏览器预览无后端
  }
  try {
    platformModelNames.value = await tauriApi.platformModels()
    platformModelsFailed.value = false
  } catch {
    // 未登录或网络失败：模型下拉只剩「自动（负载切换）」，desc 里给一句交代
    platformModelNames.value = []
    platformModelsFailed.value = true
  }
}

let unlistenNoteModels: UnlistenFn | null = null
onMounted(async () => {
  void loadNoteAiModels()
  if (!isTauri()) return
  // 平台清单后台对账（chat-models-changed）后重拉：设置开着时下拉跟随最新清单
  unlistenNoteModels = await listen('chat-models-changed', () => {
    void loadNoteAiModels()
  })
})
onBeforeUnmount(() => {
  unlistenNoteModels?.()
})

async function applyWeatherCity() {
  const city = weatherCityInput.value.trim()
  if (!city) {
    showToast('Enter a city name')
    return
  }
  weatherSaving.value = true
  try {
    const loc = await store.setWeatherCity(city)
    weatherCityInput.value = loc.name
    showToast(`Weather location set to ${loc.name}`)
  } catch (e) {
    showToast(`Could not set the city: ${String(e)}`)
  } finally {
    weatherSaving.value = false
  }
}

async function onLocateByIp() {
  weatherSaving.value = true
  try {
    const loc = await store.locateWeatherByIp()
    weatherCityInput.value = loc.name
    showToast(`Location detected: ${loc.name}`)
  } catch (e) {
    showToast(`Automatic location detection failed: ${String(e)}`)
  } finally {
    weatherSaving.value = false
  }
}

// ---- 剪贴板保留策略 ----
const clipMaxItems = ref(500)
const clipTtlDays = ref(7)
const clipRetentionSaving = ref(false)

function commitClipRetention() {
  const maxItems = Math.round(clipMaxItems.value)
  const ttlDays = Math.round(clipTtlDays.value)
  if (!isTauri() || clipRetentionSaving.value) return
  if (maxItems === store.state.config.clipboard_max_items && ttlDays === store.state.config.clipboard_ttl_days) return
  clipRetentionSaving.value = true
  void store
    .setClipboardRetention(maxItems, ttlDays)
    .then(() => showToast(`Retention settings updated: up to ${maxItems} items / ${ttlDays} days`))
    .catch((e) => {
      void reportClientError('Failed to update clipboard retention settings', e)
    })
    .finally(() => {
      clipRetentionSaving.value = false
    })
}

async function onToggleClipboardPause() {
  if (!isTauri()) return
  const next = !store.state.config.clipboard_paused
  await store.setClipboardPaused(next)
  showToast(next ? 'Clipboard history paused' : 'Clipboard history resumed')
}

// ---- 粘贴快捷键方式 ----
const PASTE_METHOD_OPTIONS = [
  { value: 'auto', label: 'Automatic (Ctrl+Shift+V in terminals, Ctrl+V elsewhere)' },
  { value: 'ctrl_v', label: 'Ctrl+V' },
  { value: 'ctrl_shift_v', label: 'Ctrl+Shift+V' },
  { value: 'shift_insert', label: 'Shift+Insert' },
] as const

const pasteMethod = ref(store.state.config.clipboard_paste_method ?? 'auto')

function onPasteMethodChange(value: string) {
  pasteMethod.value = value
  if (!isTauri()) return
  void tauriApi
    .setClipboardPasteMethod(value)
    .then(() => showToast(`Paste method updated to ${PASTE_METHOD_OPTIONS.find((o) => o.value === value)?.label ?? value}.`))
    .catch((e) => {
      void reportClientError('Failed to update the paste method', e)
    })
}

async function onClearClipboard() {
  if (!isTauri()) return
  try {
    await tauriApi.clipboardClear()
    showToast('Clipboard history cleared')
  } catch (e) {
    showToast(`Could not clear history: ${String(e)}`)
  }
}

onMounted(() => {

  clipMaxItems.value = store.state.config.clipboard_max_items ?? 500

  clipTtlDays.value = store.state.config.clipboard_ttl_days ?? 7

  pasteMethod.value = store.state.config.clipboard_paste_method ?? 'auto'

})
</script>

<template>
        <section id="sv-sec-suda" class="sv-sec" aria-label="Quick access">
          <h3 class="sv-sec-title">Quick access</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Default web link behavior</span>
              <span class="setting-desc">Where web links open: the embedded panel opens in the main window (lightweight, single page); the standalone browser supports multiple tabs and reuses tabs for the same address; Windows opens links in the system default browser. Use the context menu to choose another option temporarily.</span>
            </div>
            <AppSelect
              :model-value="sudaWebOpenMode"
              :options="SUDA_OPEN_MODE_OPTIONS"
              aria-label="Default web link behavior"
              @update:model-value="onSudaOpenModeChange"
            />
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Show toolbar in embedded panel</span>
              <span class="setting-desc">Show an address bar and back, forward, and refresh buttons at the top of the embedded panel. Off by default; the panel displays only the web page. Use the left navigation to return to Quick access.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.suda_panel_toolbar"
              :class="{ on: store.state.config.suda_panel_toolbar }"
              @click="onToggleSudaPanelToolbar"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Subcategory management</span>
              <span class="setting-desc">Organize resources into subcategories under Apps, Web, or Files. Rename inline, drag to reorder, or select the star to set a default. Use / to create nested categories (for example, Development/Frontend). Deletion requires two clicks: the trash icon becomes a checkmark on the first click; click again within 3 seconds to delete. Items move to the default subcategory. Use “Bulk delete” in the upper-right to delete multiple categories.</span>
            </div>
          </div>
          <SudaSubcategoryManager />
        </section>

        <section id="sv-sec-notes" class="sv-sec" aria-label="Notes">
          <h3 class="sv-sec-title">Notes</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">AI model for deep cleanup</span>
              <span class="setting-desc">Model used by the editor’s “Deep cleanup” button. Automatic prefers the platform when platform credits are available (with load balancing across platform models); otherwise, it uses the default chat model. You can also choose a specific platform or custom-provider model. If the selected model is deleted or unavailable, this resets to Automatic.</span>
            </div>
            <div class="notes-ai-model">
              <AppSelect
                :model-value="noteModelSource"
                :options="noteSourceOptions"
                aria-label="Cleanup model source"
                @update:model-value="onNoteModelSourceChange"
              />
              <AppSelect
                :model-value="noteModelValue"
                :options="noteModelOptions"
                :disabled="noteModelSource === 'auto'"
                aria-label="AI deep cleanup model"
                @update:model-value="onNoteModelChange"
              />
              <span v-if="noteModelSource === 'platform' && platformModelsFailed" class="notes-ai-hint">
                Could not load platform models (sign in or check your connection). Automatic load balancing is still available.
              </span>
            </div>
          </div>
        </section>

        <section id="sv-sec-clipboard" class="sv-sec" aria-label="Clipboard">
          <h3 class="sv-sec-title">Clipboard</h3>

          <h4 class="sv-subtitle">Retention</h4>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Maximum items</span>
              <span class="setting-desc">Maximum number of history items, including pinned items (default: 500)</span>
            </div>
            <div class="num-edit">
              <input
                v-model.number="clipMaxItems"
                class="num-input"
                type="number"
                min="20"
                max="5000"
                step="50"
                @change="commitClipRetention"
              />
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Retention period</span>
              <span class="setting-desc">Automatically remove unpinned items after this many days (default: 7)</span>
            </div>
            <div class="num-edit">
              <input
                v-model.number="clipTtlDays"
                class="num-input"
                type="number"
                min="1"
                max="365"
                step="1"
                @change="commitClipRetention"
              />
            </div>
          </div>

          <h4 class="sv-subtitle">History behavior</h4>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Paste method</span>
              <span class="setting-desc">In Automatic mode, use Ctrl+Shift+V in terminals and command prompts that don’t support Ctrl+V, and Ctrl+V in other apps.</span>
            </div>
            <AppSelect
              :model-value="pasteMethod"
              :options="PASTE_METHOD_OPTIONS"
              aria-label="Paste method"
              @update:model-value="onPasteMethodChange"
            />
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Pause history</span>
              <span class="setting-desc">Copied content won’t be added to history while paused. Existing items are kept.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.clipboard_paused"
              :class="{ on: store.state.config.clipboard_paused }"
              @click="onToggleClipboardPause"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <h4 class="sv-subtitle">Actions</h4>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Clear history</span>
              <span class="setting-desc">Immediately delete all clipboard history, including pinned items. This can’t be undone.</span>
            </div>
            <button class="ghost-btn data-btn danger" @click="onClearClipboard">
              <Trash2 :size="14" :stroke-width="2" />
              Clear
            </button>
          </div>
        </section>

        <section id="sv-sec-gif" class="sv-sec" aria-label="GIF recording">
          <h3 class="sv-sec-title">GIF recording</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Default frame rate</span>
              <span class="setting-desc">Frames captured per second. Higher values look smoother and create larger files. The selection panel uses this default each time.</span>
            </div>
            <AppSelect
              :model-value="recorderFps"
              :options="RECORDER_FPS_OPTIONS"
              aria-label="Default frame rate"
              @update:model-value="onRecorderFpsChange"
            />
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Save location</span>
              <span class="setting-desc">
                {{ recorderSaveDir || `Default (captures folder in the data directory)` }}
                <template v-if="recorderSaveDir">(falls back to the default if unavailable)</template>
              </span>
            </div>
            <div class="gif-dir-actions">
              <button class="ghost-btn" type="button" @click="chooseRecorderDir">Choose folder</button>
              <button
                v-if="recorderSaveDir"
                class="ghost-btn"
                type="button"
                @click="resetRecorderDir"
              >
                Restore default
              </button>
            </div>
          </div>
        </section>

        <section id="sv-sec-online" class="sv-sec" aria-label="Online services">
          <h3 class="sv-sec-title">Online services</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Online features</span>
              <span class="setting-desc">When enabled, weather and online quotes appear when connected and are hidden when offline. Local features are unaffected.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.online_enabled"
              :class="{ on: store.state.config.online_enabled }"
              @click="onToggleOnline"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Weather location</span>
              <span class="setting-desc">Enter a city name (major cities are included; exact matches are required). IP-based location is approximate.</span>
            </div>
            <div class="weather-edit">
              <input
                v-model="weatherCityInput"
                class="field-input"
                type="text"
                maxlength="30"
                placeholder="Enter a city, e.g. Seattle"
                spellcheck="false"
                @keydown.enter="applyWeatherCity"
              />
              <button
                class="ghost-btn data-btn"
                type="button"
                :disabled="weatherSaving"
                @click="applyWeatherCity"
              >
                <MapPin :size="14" :stroke-width="2" />
                Set
              </button>
              <button
                class="ghost-btn data-btn"
                type="button"
                :disabled="weatherSaving"
                @click="onLocateByIp"
              >
                <LocateFixed :size="14" :stroke-width="2" />
                Detect automatically
              </button>
            </div>
          </div>
        </section>

        <section id="sv-sec-mem" class="sv-sec" aria-label="Performance">
          <h3 class="sv-sec-title">Performance</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Reduce memory use for hidden windows</span>
              <span class="setting-desc">Chat, clipboard, notification, and Quick access browser windows stay in the background. When enabled, hidden windows return as much renderer memory as possible to the system and restore it when shown; scripts and messages continue running. When disabled, hidden windows retain their memory.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.webview_mem_low_on_hide"
              :class="{ on: store.state.config.webview_mem_low_on_hide }"
              @click="onToggleWebviewMem"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>
        </section>
</template>
