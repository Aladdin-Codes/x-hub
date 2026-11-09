<script setup lang="ts">
// Skills section under Extensions. Install the bundled extension-development
// skill into a local AI coding assistant's skills directory.
//
// The skill ships with the app. This panel detects destinations and supports
// installation, updates, and removal. Automatic detection lists existing folders
// only; users can also select a custom destination.
import { computed, inject, onMounted, ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { Download, FolderPlus, Lock, RefreshCw, Sparkles, Trash2 } from 'lucide-vue-next';
import { isTauri, tauriApi } from '../../api/tauri';
import type { SkillOverview, SkillTarget } from '../../api/tauri';

const showToast = inject<(msg: string) => void>('showToast', () => {})

const overview = ref<SkillOverview | null>(null)
/** Destination currently being processed; prevents concurrent operations. */
const opPath = ref('')
/** Whether the install-all operation is running. */
const installingAll = ref(false)
/** Destination awaiting confirmation before replacing an existing skill. */
const confirmPath = ref<string | null>(null)
/** Custom destination awaiting confirmation. */
const customConfirm = ref<string | null>(null)
let confirmTimer: ReturnType<typeof setTimeout> | null = null

const skill = computed(() => overview.value?.skill ?? null)
const targets = computed(() => overview.value?.targets ?? [])
/** Number of detected destinations without an installed skill. */
const pendingCount = computed(() => targets.value.filter((t) => !t.installed).length)
/** Number of outdated skills installed by this app; foreign copies require confirmation. */
const outdatedCount = computed(
  () => targets.value.filter((t) => t.installed && !t.up_to_date && !t.foreign).length,
)
/** Whether the update-all operation is running. */
const updatingAll = ref(false)

async function load() {
  if (!isTauri()) return
  try {
    overview.value = await tauriApi.getSkillOverview()
  } catch (e) {
    showToast(`Failed to load skill status: ${String(e)}`)
  }
}

function fmtSize(bytes: number): string {
  if (!bytes) return '0 B'
  return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`
}

function statusText(t: SkillTarget): string {
  if (!t.installed) return 'Not installed'
  if (t.foreign) return 'A folder with this name already exists. Confirm before replacing it.'
  if (t.up_to_date) return t.installed_version ? `Installed v${t.installed_version}` : 'Installed'
  return t.installed_version
    ? `Update available (installed v${t.installed_version})`
    : 'Update available'
}

/** Whether replacing this foreign installation requires confirmation. */
function needConfirm(t: SkillTarget): boolean {
  return t.installed && t.foreign
}

function armConfirm(path: string, custom = false) {
  if (custom) customConfirm.value = path
  else confirmPath.value = path
  if (confirmTimer) clearTimeout(confirmTimer)
  confirmTimer = setTimeout(() => {
    confirmPath.value = null
    customConfirm.value = null
  }, 4000)
}

/** Install or update at a destination; force replaces an existing installation. */
async function installTo(t: SkillTarget, force: boolean) {
  if (opPath.value) return
  if (needConfirm(t) && confirmPath.value !== t.path) {
    armConfirm(t.path)
    showToast('A skill with this name already exists and was not installed by this app. Click again to confirm replacement.')
    return
  }
  opPath.value = t.path
  confirmPath.value = null
  customConfirm.value = null
  try {
    overview.value = await tauriApi.installSkill(t.path, force)
    showToast(t.installed ? `Updated in ${t.label}.` : `Installed in ${t.label}.`)
  } catch (e) {
    showToast(`Installation failed: ${String(e)}`)
  } finally {
    opPath.value = ''
  }
}

/** Main destination action: update, confirm replacement, or install. */
function onAction(t: SkillTarget) {
  // Existing destinations require an explicit replacement request.
  void installTo(t, t.installed)
}

async function uninstall(t: SkillTarget) {
  if (opPath.value) return
  opPath.value = t.path
  try {
    overview.value = await tauriApi.uninstallSkill(t.path)
    showToast(`Uninstalled from ${t.label}.`)
  } catch (e) {
    showToast(`Uninstall failed: ${String(e)}`)
  } finally {
    opPath.value = ''
  }
}

/** Remove a custom destination from the list without deleting its files. */
async function forgetCustom(t: SkillTarget) {
  if (opPath.value) return
  opPath.value = t.path
  try {
    overview.value = await tauriApi.removeSkillRoot(t.path)
    showToast('Removed from the list. Files on disk were not deleted.')
  } catch (e) {
    showToast(`Could not remove the destination: ${String(e)}`)
  } finally {
    opPath.value = ''
  }
}

/** Install into all detected destinations without an existing copy. */
async function installAll() {
  const pending = targets.value.filter((t) => !t.installed)
  if (!pending.length) {
    showToast('The skill is already installed in all detected destinations.')
    return
  }
  installingAll.value = true
  let ok = 0
  for (const t of pending) {
    try {
      overview.value = await tauriApi.installSkill(t.path, false)
      ok += 1
    } catch (e) {
      showToast(`Installation failed in ${t.label}: ${String(e)}`)
    }
  }
  installingAll.value = false
  showToast(`Installed in ${ok} of ${pending.length} destinations.`)
}

/** Update all outdated copies installed by this app. */
async function updateAll() {
  const outdated = targets.value.filter((t) => t.installed && !t.up_to_date && !t.foreign)
  if (!outdated.length) return
  updatingAll.value = true
  let ok = 0
  for (const t of outdated) {
    try {
      overview.value = await tauriApi.installSkill(t.path, true)
      ok += 1
    } catch (e) {
      showToast(`Update failed in ${t.label}: ${String(e)}`)
    }
  }
  updatingAll.value = false
  showToast(`Updated ${ok} of ${outdated.length} destinations.`)
}

/** Install missing copies, update outdated copies, or do nothing when all are current. */
function onPrimaryAction() {
  if (pendingCount.value) void installAll()
  else if (outdatedCount.value) void updateAll()
}

/** Install in a custom destination; a second click confirms replacement. */
async function pickCustom() {
  if (opPath.value) return
  if (customConfirm.value) {
    const path = customConfirm.value
    customConfirm.value = null
    opPath.value = path
    try {
      overview.value = await tauriApi.installSkill(path, true)
      showToast('Installed in the custom destination.')
    } catch (e) {
      showToast(`Installation failed: ${String(e)}`)
    } finally {
      opPath.value = ''
    }
    return
  }
  const picked = await open({
    directory: true,
    multiple: false,
    title: 'Select an AI assistant skills directory',
  })
  if (typeof picked !== 'string') return
  opPath.value = picked
  try {
    overview.value = await tauriApi.installSkill(picked, false)
    showToast('Installed in the custom destination.')
  } catch (e) {
    const msg = String(e)
    if (msg.includes('CONFLICT')) {
      armConfirm(picked, true)
      showToast('A skill with this name already exists. Click “Select destination” again to confirm replacement.')
    } else {
      showToast(`Installation failed: ${msg}`)
    }
  } finally {
    opPath.value = ''
  }
}

onMounted(() => {
  void load()
})
</script>

<template>
  <section id="sv-sec-skills" class="sv-sec" aria-label="Skills">
    <h3 class="sv-sec-title">Skills</h3>

    <p class="skill-hint">
      Install the extension development skill in your AI coding assistant's skills directory (Claude Code, ZCode, Codex, Doubao, and others). Then ask the assistant to use x-hub-extension to build an extension.
    </p>

    <!-- Bundled skill package. -->
    <div v-if="skill" class="skill-card">
      <div class="skill-card-head">
        <Sparkles :size="15" :stroke-width="2" class="skill-card-icon" aria-hidden="true" />
        <span class="skill-card-name">{{ skill.name }}</span>
        <span class="skill-meta">Bundled with app v{{ skill.app_version }}</span>
      </div>
      <p class="skill-card-desc">
        Helps your assistant create extensions that follow the x-hub specification (manifest, entry page, and optional service backend). Includes {{ skill.file_count }} files ({{ fmtSize(skill.size) }}); install it in a detected assistant directory below.
      </p>
      <div class="skill-actions">
        <button
          class="pill-btn"
          type="button"
          :disabled="installingAll || updatingAll || !!opPath || (!pendingCount && !outdatedCount)"
          @click="onPrimaryAction"
        >
          <Download :size="14" :stroke-width="2" />
          {{
            installingAll
              ? 'Installing…'
              : updatingAll
                ? 'Updating…'
                : pendingCount
                  ? `Install all (${pendingCount})`
                  : outdatedCount
                    ? `Update all (${outdatedCount})`
                    : 'All up to date'
          }}
        </button>
        <button class="ghost-btn data-btn" type="button" :disabled="!!opPath" @click="pickCustom">
          <FolderPlus :size="14" :stroke-width="2" />
          {{ customConfirm ? 'Confirm replacement' : 'Select destination' }}
        </button>
      </div>
    </div>

    <h4 class="sv-subtitle">Detected assistant directories</h4>
    <p v-if="!targets.length" class="skill-empty">
      No known AI assistant skills directories were found. Use “Select destination” above to choose a custom location.
    </p>

    <div v-for="t in targets" :key="t.path" class="setting-row">
      <div class="setting-info">
        <span class="setting-name">
          {{ t.label }}
          <b v-if="t.installed && t.up_to_date" class="skill-badge">Installed</b>
          <b v-else-if="t.installed" class="skill-badge warn">Update available</b>
        </span>
        <span class="dev-dir-path" :title="t.path">{{ t.path }}</span>
        <span class="skill-state" :class="{ warn: t.foreign || (t.installed && !t.up_to_date) }">
          {{ statusText(t) }}
        </span>
      </div>
      <div class="skill-row-actions">
        <button
          v-if="!t.up_to_date"
          class="ghost-btn data-btn"
          type="button"
          :disabled="!!opPath"
          :class="{ confirm: confirmPath === t.path }"
          @click="onAction(t)"
        >
          <Download v-if="!t.installed" :size="14" :stroke-width="2" />
          <RefreshCw v-else :size="14" :stroke-width="2" />
          {{ confirmPath === t.path ? 'Confirm replacement' : t.installed ? 'Update' : 'Install' }}
        </button>
        <button
          v-if="t.installed"
          class="ghost-btn data-btn danger"
          type="button"
          :disabled="!!opPath"
          @click="uninstall(t)"
        >
          <Trash2 :size="14" :stroke-width="2" />
          Uninstall
        </button>
        <button
          v-if="t.kind === 'custom'"
          class="ghost-btn data-btn"
          type="button"
          :disabled="!!opPath"
          title="Remove from the list without deleting files on disk"
          @click="forgetCustom(t)"
        >
          Remove
        </button>
      </div>
    </div>

    <p class="settings-foot">
      <Lock :size="12" :stroke-width="2" class="settings-lock" aria-hidden="true" />
      The skill ships with this app and uses the same version. Automatic detection lists existing directories only; it does not create directories.
    </p>
  </section>
</template>

<style scoped>
/* 本分区专属样式（通用设置样式在 settings/shared.css，见约定 50） */
.skill-hint {
  margin: 0 0 12px;
  font-size: 0.75rem;
  line-height: 1.6;
  color: var(--text-2);
}
.skill-card {
  margin-bottom: 14px;
  padding: 12px 14px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-lg);
  background: var(--bg-card-soft);
}
.skill-card-head {
  display: flex;
  align-items: center;
  gap: 6px;
}
.skill-card-icon {
  color: var(--brand-500);
}
.skill-card-name {
  font-family: ui-monospace, Consolas, monospace;
  font-size: 0.82rem;
  font-weight: 650;
  color: var(--text-1);
}
.skill-meta {
  margin-left: auto;
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--bg-card);
  font-size: 0.66rem;
  color: var(--text-3);
}
.skill-card-desc {
  margin: 8px 0 10px;
  font-size: 0.75rem;
  line-height: 1.6;
  color: var(--text-2);
}
.skill-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.skill-empty {
  margin: 0 0 10px;
  font-size: 0.72rem;
  line-height: 1.6;
  color: var(--text-3);
}
.skill-badge {
  margin-left: 6px;
  padding: 0 5px;
  border-radius: 999px;
  background: var(--bg-card-soft);
  font-size: 0.66rem;
  font-weight: 600;
  color: var(--c-green-ink, var(--text-2));
}
.skill-badge.warn {
  color: var(--c-orange-ink, var(--text-2));
}
.skill-state {
  font-size: 0.72rem;
  color: var(--text-3);
}
.skill-state.warn {
  color: var(--c-orange-ink, var(--text-2));
}
.skill-row-actions {
  display: flex;
  flex-shrink: 0;
  gap: 6px;
}
</style>
