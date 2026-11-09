<script setup lang="ts">
// Extension settings: runtime configuration and the built-in extension
// development skill installer.
//
// Local extension source directories are managed in the Extension Center's
// "My Extensions" tab. Adding a directory immediately loads it for development.
//
// Extracted from SettingsView.vue. Settings panels load on demand so the
// initial view only needs the shell and the selected category.
import { computed, inject, onMounted, ref } from 'vue';
import { Puzzle } from 'lucide-vue-next';
import { isTauri, tauriApi } from '../../api/tauri';
import type { DevModeStatus, ExtensionEntry } from '../../api/tauri';
import AppSelect from '../AppSelect.vue';
import SkillsSection from './SkillsSection.vue';
import { useStore } from '../../stores/workbench';
import { effectiveKeepAliveIds, EXT_KEEP_ALIVE_CAP, hasKeepAliveSlot } from '../../utils/extensionKeepAlive';

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()

const emit = defineEmits<{
  /** 跳到扩展中心（本机源码目录的增删在那里） */
  (e: 'open-extensions'): void
}>()

const RUNTIME_STRATEGY_OPTIONS = [
  { value: 'auto', label: 'Automatic (prefer system Node; download the bundled runtime if missing)' },
  { value: 'builtin', label: 'Bundled runtime (always use the downloaded runtime)' },
  { value: 'system', label: 'System runtime (use system Node; do not download)' },
] as const

function onRuntimeStrategyChange(value: string) {
  void store.setRuntimeStrategy(value as 'auto' | 'builtin' | 'system').then(() => {
    showToast('Runtime strategy updated. It will apply the next time a service extension starts.')
  })
}

/** Click behavior for rows in the Extension Center's Installed and My Extensions lists. */
const ROW_CLICK_OPTIONS = [
  { value: 'detail', label: 'Open details' },
  { value: 'open', label: 'Open extension' },
] as const

function onRowClickModeChange(value: string) {
  void store.setExtensionRowClick(value as 'detail' | 'open').then(
    () => showToast(value === 'open' ? 'Extension rows now open the extension.' : 'Extension rows now open details.'),
    () => showToast('Could not save the setting. Try again.'),
  )
}

/** Automatically trust service extensions unless an extension is explicitly denied. */
function onToggleAutoTrust() {
  const next = !store.state.config.service_auto_trust
  void store.setServiceAutoTrust(next).then(
    () => showToast(next ? 'Automatic trust enabled. Newly installed or updated service extensions can run immediately.' : 'Automatic trust disabled. Confirm each service extension individually.'),
    () => showToast('Could not save the setting. Try again.'),
  )
}

// ---- My Extensions (directory management lives in the Extension Center) ----
const devMode = ref<DevModeStatus>({ enabled: true, extensions: [] })

async function loadDevMode() {
  if (!isTauri()) return
  try {
    devMode.value = await tauriApi.getDevModeStatus()
  } catch {
    // This section is only relevant to the desktop app.
  }
}

// ---- Keep-alive extensions: list configured instances and update their settings.
// The main window manages mounting and unmounting; pruning watches handle cleanup. ----
const installedList = ref<ExtensionEntry[]>([])

async function loadInstalled() {
  if (!isTauri()) return
  try {
    installedList.value = await tauriApi.listExtensions()
  } catch {
    // Fall back to the extension ID if the list cannot be loaded.
  }
}

function nameOf(id: string): string {
  return installedList.value.find((e) => e.id === id)?.name ?? id
}

const keepAliveRows = computed(() =>
  effectiveKeepAliveIds(store.state.config).map((id) => ({
    id,
    name: nameOf(id),
    keepAlive: (store.state.config.extension_keep_alive ?? []).includes(id),
    autostart: (store.state.config.extension_autostart ?? []).includes(id),
  })),
)

function onToggleKeepAlive(id: string) {
  const on = !(store.state.config.extension_keep_alive ?? []).includes(id)
  if (on && !hasKeepAliveSlot(store.state.config, id)) {
    showToast(`Up to ${EXT_KEEP_ALIVE_CAP} extensions can stay active. Turn off keep-alive for another extension first.`)
    return
  }
  store.setExtensionKeepAlive(id, on)
  // Autostart implies keep-alive, so disabling keep-alive also disables autostart.
  if (!on && (store.state.config.extension_autostart ?? []).includes(id)) {
    store.setExtensionAutostart(id, false)
  }
}

function onToggleAutostart(id: string) {
  const row = keepAliveRows.value.find((r) => r.id === id)
  if (row?.autostart) {
    store.setExtensionAutostart(id, false)
    return
  }
  // Autostart implies keep-alive. An extension already using a slot does not
  // consume another one when autostart is enabled.
  if (!hasKeepAliveSlot(store.state.config, row?.keepAlive ? id : undefined)) {
    showToast(`Up to ${EXT_KEEP_ALIVE_CAP} extensions can stay active. Turn off keep-alive for another extension first.`)
    return
  }
  if (!row?.keepAlive) store.setExtensionKeepAlive(id, true)
  store.setExtensionAutostart(id, true)
}

onMounted(() => {
  void loadDevMode()
  void loadInstalled()
})
</script>

<template>
        <section id="sv-sec-extensions" class="sv-sec" aria-label="Extensions">
          <h3 class="sv-sec-title">Extensions</h3>
          <p class="dev-hint">
            Install, update, and develop extensions in the sidebar's <b>Extension Center</b>. Use <b>Installed</b> to manage installed extensions, <b>Marketplace</b> to find and install extensions, and <b>My Extensions</b> to add and work on local source directories. This page only configures the runtime.
          </p>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Service runtime</span>
              <span class="setting-desc">Choose which Node.js runtime service extensions use: automatic, bundled, or system.</span>
            </div>
            <AppSelect
              :model-value="store.state.config.runtime_strategy || 'auto'"
              :options="RUNTIME_STRATEGY_OPTIONS"
              aria-label="Service runtime"
              @update:model-value="onRuntimeStrategyChange"
            />
          </div>

          <!-- Let users choose whether selecting a row opens the extension or its details. -->
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">When selecting an extension row</span>
              <span class="setting-desc">Choose whether selecting a row in Installed or My Extensions opens its details or opens the extension directly. When set to open directly, use the ⋯ button to view details.</span>
            </div>
            <AppSelect
              :model-value="store.state.config.extension_row_click || 'detail'"
              :options="ROW_CLICK_OPTIONS"
              aria-label="When selecting an extension row"
              @update:model-value="onRowClickModeChange"
            />
          </div>

          <!-- Service backends are not sandboxed and may access files and the network.
               An explicit denial for an individual extension always takes precedence. -->
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Automatically trust service extensions</span>
              <span class="setting-desc">
                When enabled, newly installed or updated service extensions can run their local backend without individual approval. ⚠️ Service backends are not sandboxed and can access files available to you and connect to the network. Enable this only for extensions you trust. An explicit decision to disable a specific extension's backend always takes precedence.
              </span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.service_auto_trust"
              aria-label="Automatically trust service extensions"
              :class="{ on: store.state.config.service_auto_trust }"
              @click="onToggleAutoTrust"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <!-- Keep-alive settings for extensions that are currently configured to run. -->
          <h4 class="sv-subtitle">Keep-alive extensions</h4>
          <template v-if="keepAliveRows.length">
            <div v-for="row in keepAliveRows" :key="row.id" class="setting-row">
              <div class="setting-info">
                <span class="setting-name">{{ row.name }}</span>
                <span class="setting-desc">Keep running when you switch pages, preserve state, and resume instantly. Autostart launches the extension in the background when x-hub starts.</span>
              </div>
              <div class="ka-controls">
                <span class="ka-ctl-label">Keep alive</span>
                <button
                  class="toggle"
                  role="switch"
                  type="button"
                  :aria-checked="row.keepAlive"
                  :class="{ on: row.keepAlive }"
                  :aria-label="`Keep ${row.name} running in the background`"
                  @click="onToggleKeepAlive(row.id)"
                >
                  <span class="toggle-knob"></span>
                </button>
                <span class="ka-ctl-label">Start with app</span>
                <button
                  class="toggle"
                  role="switch"
                  type="button"
                  :aria-checked="row.autostart"
                  :class="{ on: row.autostart }"
                  :aria-label="`Start ${row.name} with x-hub`"
                  @click="onToggleAutostart(row.id)"
                >
                  <span class="toggle-knob"></span>
                </button>
              </div>
            </div>
          </template>
          <div v-else class="setting-row">
            <div class="setting-info">
              <span class="setting-name">No keep-alive extensions</span>
              <span class="setting-desc">In the Extension Center, open an extension's details and enable Background run → Keep alive. Its page will continue running when you switch away. Limit: {{ EXT_KEEP_ALIVE_CAP }} extensions; each instance uses tens of MB of memory.</span>
            </div>
          </div>

          <!-- Local source directories are managed in the Extension Center. -->
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">My Extensions</span>
              <span class="setting-desc">
                {{ devMode.extensions.length ? `${devMode.extensions.length} local source director${devMode.extensions.length === 1 ? 'y' : 'ies'} added` : 'No local source directories added' }} (each must contain manifest.json). Directories load immediately and reload when files change, so you can test locally. Developer verification is required to publish. Add or remove directories in the My Extensions tab of the Extension Center.
              </span>
            </div>
            <button class="ghost-btn data-btn ext-entry-btn" type="button" @click="emit('open-extensions')">
              <Puzzle :size="14" :stroke-width="2" />
              Open Extension Center
            </button>
          </div>
        </section>

        <!-- Install the built-in extension development skill in a local AI assistant. -->
        <SkillsSection />
</template>

<style scoped>
/* Panel-specific styles; shared settings styles live in settings/shared.css. */
.dev-hint {
  margin: 0 0 10px;
  font-size: 0.75rem;
  line-height: 1.6;
  color: var(--text-2);
}
.dev-hint b {
  color: var(--text-1);
}
/* Keep the Extension Center button on one line beside the long description. */
.ext-entry-btn {
  flex-shrink: 0;
  flex-wrap: nowrap;
  white-space: nowrap;
}
/* Keep the keep-alive and autostart controls on one line. */
.ka-controls {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: nowrap;
}
.ka-ctl-label {
  font-size: 0.75rem;
  color: var(--text-3);
  white-space: nowrap;
}
</style>
