<script setup lang="ts">
// Data and About settings (storage, backup and restore, notes maintenance, and app info).
//
// Extracted from SettingsView.vue. Settings panels load on demand so the initial
// view only needs the shell and the selected category.
import { computed, inject, onMounted, ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { Download, Eraser, FolderCog, FolderOutput, Lock, Upload } from 'lucide-vue-next';
import { isTauri, tauriApi } from '../../api/tauri';
import type { DataPathInfo, NoteImageGcReport } from '../../api/tauri';
import { useStore } from '../../stores/workbench';
import AppSelect from '../AppSelect.vue';
import AboutSection from '../AboutSection.vue';

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()

// ---- Data storage location ----
const dataPathInfo = ref<DataPathInfo | null>(null)
const changeDataTarget = ref<string | null>(null)
const changeDataBusy = ref(false)

const dataPathLabel = computed(() => {
  const info = dataPathInfo.value
  if (!info) return 'Loading…'
  if (info.mode === 'portable') return `Portable · ${info.path}`
  return info.path
})

async function loadDataPath() {
  if (!isTauri()) return
  try {
    dataPathInfo.value = await tauriApi.getDataPath()
  } catch (e) {
    showToast(`Failed to read the data location: ${String(e)}`)
  }
}

async function onChangeDataDir() {
  if (!isTauri() || dataPathInfo.value?.mode === 'portable') return
  const dir = await open({ multiple: false, directory: true })
  if (typeof dir !== 'string') return
  if (dir === dataPathInfo.value?.path) {
    showToast('The selected folder is already in use.')
    return
  }
  changeDataTarget.value = dir
}

function cancelChangeDataDir() {
  changeDataTarget.value = null
}

async function confirmChangeDataDir() {
  if (!changeDataTarget.value || changeDataBusy.value) return
  changeDataBusy.value = true
  try {
    await tauriApi.changeDataDir(changeDataTarget.value)
    changeDataTarget.value = null
    showToast('Data moved. The app will restart shortly.')
    setTimeout(() => void tauriApi.restartApp(), 700)
  } catch (e) {
    showToast(`Failed to move data: ${String(e)}`)
  } finally {
    changeDataBusy.value = false
  }
}

// ---- Data backup and restore ----
const confirmRestore = ref(false)
let confirmTimer: ReturnType<typeof setTimeout> | null = null

async function backupData() {
  if (!isTauri()) return
  const dir = await open({ multiple: false, directory: true })
  if (typeof dir !== 'string') return
  try {
    const name = await tauriApi.backupData(dir)
    showToast(`Backup created: ${name}`)
  } catch (e) {
    showToast(`Backup failed: ${String(e)}`)
  }
}

async function restoreData() {
  if (!isTauri()) return
  // Require a second click to confirm.
  if (!confirmRestore.value) {
    confirmRestore.value = true
    if (confirmTimer) clearTimeout(confirmTimer)
    confirmTimer = setTimeout(() => {
      confirmRestore.value = false
    }, 3000)
    return
  }
  confirmRestore.value = false
  const file = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Backup archive', extensions: ['zip'] }],
  })
  if (typeof file !== 'string') return
  try {
    await tauriApi.restoreData(file)
    showToast('Restore staged. Restart the app to apply it.')
  } catch (e) {
    showToast(`Restore failed: ${String(e)}`)
  }
}

// ---- Notes maintenance: trash retention, orphan cleanup, export, and import. ----
const RETENTION_OPTIONS = [
  { value: '0', label: 'Keep forever' },
  { value: '7', label: 'Keep for 7 days' },
  { value: '30', label: 'Keep for 30 days' },
  { value: '90', label: 'Keep for 90 days' },
  { value: '365', label: 'Keep for 1 year' },
]

const noteRetention = computed({
  get: () => String(store.state.config.note_trash_retention_days ?? 0),
  set: (v: string) => void setNoteRetention(Number(v) || 0),
})

async function setNoteRetention(days: number) {
  if (!isTauri()) return
  try {
    const r = await store.setNoteTrashRetention(days)
    showToast(r && r.purged > 0 ? `Removed ${r.purged} expired notes from the trash.` : 'Trash retention updated.')
  } catch (e) {
    showToast(`Failed to update the setting: ${String(e)}`)
  }
}

const gcBusy = ref(false)
const gcReport = ref<NoteImageGcReport | null>(null)

async function scanOrphanImages() {
  if (!isTauri() || gcBusy.value) return
  gcBusy.value = true
  try {
    gcReport.value = await tauriApi.gcOrphanNoteImages(true)
  } catch (e) {
    showToast(`Scan failed: ${String(e)}`)
  } finally {
    gcBusy.value = false
  }
}

async function cleanOrphanImages() {
  if (!isTauri() || gcBusy.value || !gcReport.value) return
  gcBusy.value = true
  try {
    const r = await tauriApi.gcOrphanNoteImages(false)
    gcReport.value = r
    showToast(
      r.failed > 0
        ? `Removed ${r.removed} orphaned attachments; ${r.failed} could not be deleted (they may be in use).`
        : `Removed ${r.removed} orphaned attachments.`,
    )
  } catch (e) {
    showToast(`Cleanup failed: ${String(e)}`)
  } finally {
    gcBusy.value = false
  }
}

const exportBusy = ref(false)

async function exportNotes() {
  if (!isTauri() || exportBusy.value) return
  const dir = await open({ multiple: false, directory: true, title: 'Select an export folder' })
  if (typeof dir !== 'string') return
  exportBusy.value = true
  try {
    const r = await tauriApi.exportNotes(dir)
    showToast(
      r.failed.length > 0
        ? `Exported ${r.exported} notes; ${r.failed.length} failed. → ${r.dir}`
        : `Exported ${r.exported} notes and ${r.images} images. → ${r.dir}`,
    )
  } catch (e) {
    showToast(`Export failed: ${String(e)}`)
  } finally {
    exportBusy.value = false
  }
}

const importBusy = ref(false)
const importing = ref(false)

async function importNotes() {
  if (!isTauri() || importBusy.value) return
  const dir = await open({ multiple: false, directory: true, title: 'Select an x-hub export folder' })
  if (typeof dir !== 'string') return
  importBusy.value = true
  importing.value = true
  try {
    const r = await tauriApi.importNotes(dir, { overwrite: false })
    importing.value = false
    void store.refreshNotes()
    void store.refreshNoteFolders()
    const failNote = r.failed.length > 0 ? `, ${r.failed.length} failed` : ''
    const cancelNote = r.cancelled ? ' (cancelled)' : ''
    showToast(
      `Import complete: ${r.imported} imported, ${r.skipped} skipped${failNote}${cancelNote}.`,
    )
  } catch (e) {
    showToast(`Import failed: ${String(e)}`)
  } finally {
    importing.value = false
    importBusy.value = false
  }
}

async function cancelImport() {
  try {
    await tauriApi.importNotesCancel()
  } catch {
    /* 无导入在进行时静默 */
  }
}

onMounted(() => {

  void loadDataPath()

})
</script>

<template>
        <section id="sv-sec-data" class="sv-sec" aria-label="Data">
          <h3 class="sv-sec-title">Data</h3>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Data location</span>
              <span class="setting-desc data-path">{{ dataPathLabel }}</span>
            </div>
            <button
              class="ghost-btn data-btn"
              :disabled="dataPathInfo?.mode === 'portable'"
              :title="dataPathInfo?.mode === 'portable' ? 'Portable data is stored beside the app and cannot be moved.' : 'Change the data location'"
              @click="onChangeDataDir"
            >
              <FolderCog :size="14" :stroke-width="2" />
              Change
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Back up data</span>
              <span class="setting-desc">Create an archive containing the database and icons.</span>
            </div>
            <button class="ghost-btn data-btn" @click="backupData">
              <Download :size="14" :stroke-width="2" />
              Back up
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Restore data</span>
              <span class="setting-desc">Restore from a backup archive. Changes take effect after restarting.</span>
            </div>
            <button
              class="ghost-btn data-btn"
              :class="{ confirm: confirmRestore }"
              @click="restoreData"
            >
              <Upload :size="14" :stroke-width="2" />
              {{ confirmRestore ? 'Confirm restore?' : 'Restore' }}
            </button>
          </div>

          <h3 class="sv-sec-title gap-above">Notes maintenance</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Trash retention</span>
              <span class="setting-desc">Deleted notes go to the trash and are removed after the selected period. Set to 0 to keep them indefinitely. Applies at startup and when changed.</span>
            </div>
            <AppSelect
              v-model="noteRetention"
              :options="RETENTION_OPTIONS"
              style="width: 140px"
            />
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Orphaned attachment cleanup</span>
              <span class="setting-desc">
                <template v-if="gcReport">
                  {{ gcReport.total_files }} attachments (images and spreadsheets); {{ gcReport.referenced }} are referenced by notes
                  <template v-if="gcReport.orphan_files.length">, {{ gcReport.orphan_files.length }} orphaned</template>
                </template>
                <template v-else>Scan for images and spreadsheets not referenced by any note, including notes in the trash. Scan before cleanup.</template>
              </span>
            </div>
            <button class="ghost-btn data-btn" :disabled="gcBusy" @click="scanOrphanImages">
              <Eraser :size="14" :stroke-width="2" />
              {{ gcBusy ? 'Working…' : gcReport ? 'Scan again' : 'Scan' }}
            </button>
            <button
              v-if="gcReport && gcReport.orphan_files.length > 0"
              class="ghost-btn data-btn"
              :disabled="gcBusy"
              @click="cleanOrphanImages"
            >
              Remove {{ gcReport.orphan_files.length }}
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Export notes</span>
              <span class="setting-desc">Export notes by folder as Markdown, including front matter and image assets. The export can be imported later.</span>
            </div>
            <button class="ghost-btn data-btn" :disabled="exportBusy" @click="exportNotes">
              <FolderOutput :size="14" :stroke-width="2" />
              {{ exportBusy ? 'Exporting…' : 'Export' }}
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Import notes</span>
              <span class="setting-desc">Only files exported by x-hub are supported. Notes with a matching source URL are skipped by default.</span>
            </div>
            <button class="ghost-btn data-btn" :disabled="importBusy" @click="importNotes">
              <Download :size="14" :stroke-width="2" />
              {{ importBusy ? 'Importing…' : 'Import' }}
            </button>
            <button v-if="importing" class="ghost-btn data-btn" @click="cancelImport">Cancel</button>
          </div>

          <p class="settings-foot">
            <Lock :size="12" :stroke-width="2" class="settings-lock" aria-hidden="true" />
            Your data is stored locally by default and is not uploaded to the cloud.
          </p>
        </section>

        <!-- Confirm data location change -->
        <Teleport to="body">
          <Transition name="mask">
            <div
              v-if="changeDataTarget"
              class="modal-mask"
              role="presentation"
              @click.self="cancelChangeDataDir"
            >
              <div
                class="modal-card data-move-card"
                role="dialog"
                aria-modal="true"
                aria-label="Change data location"
              >
                <h3 class="dm-title">Move data</h3>
                <p class="dm-desc">Move all x-hub data to the folder below? The app will restart after you confirm.</p>
                <div class="dm-paths">
                  <div class="dm-path">
                    <span class="dm-label">New folder</span>
                    <span class="dm-val">{{ changeDataTarget }}</span>
                  </div>
                </div>
                <footer class="dm-footer">
                  <button class="ghost-btn" type="button" @click="cancelChangeDataDir">Cancel</button>
                  <button
                    class="pill-btn"
                    type="button"
                    :disabled="changeDataBusy"
                    @click="confirmChangeDataDir"
                  >
                    {{ changeDataBusy ? 'Moving…' : 'Move data' }}
                  </button>
                </footer>
              </div>
            </div>
          </Transition>
        </Teleport>

        <section id="sv-sec-about" class="sv-sec" aria-label="About">
          <h3 class="sv-sec-title">About</h3>
          <AboutSection />
        </section>
</template>

<style scoped>
/* 迁移数据弹窗：Teleport 到 body，不在 .settings-view 子树里，只能靠 scoped 命中 */
.data-move-card {
  width: 480px;
  max-width: calc(100vw - 48px);
}
.dm-title {
  margin: 0 0 10px;
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-1);
}
.dm-desc {
  margin: 0 0 14px;
  font-size: 0.8125rem;
  line-height: 1.6;
  color: var(--text-2);
}
.dm-paths {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}
.dm-path {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  background: var(--bg-card-soft);
}
.dm-label {
  font-size: 0.6875rem;
  font-weight: 700;
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.dm-val {
  font-size: 0.75rem;
  color: var(--text-2);
  overflow-wrap: anywhere;
}
.dm-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid var(--border-soft);
}

/* 弹窗遮罩过渡 */
.mask-enter-active,
.mask-leave-active {
  transition: opacity 0.18s ease-out;
}
.mask-enter-from,
.mask-leave-to {
  opacity: 0;
}
</style>
