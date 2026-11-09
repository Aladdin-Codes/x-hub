<script setup lang="ts">
// General settings (startup, floating ball, and shortcuts).
//
// Extracted from SettingsView.vue. Settings panels load on demand so the
// initial view only needs the shell and the selected category.
import { computed, inject, onMounted, ref } from 'vue';
import { Keyboard } from 'lucide-vue-next';
import { isTauri, tauriApi } from '../../api/tauri';
import type { AutostartStatus } from '../../api/tauri';
import { useStore } from '../../stores/workbench';
import { normalizeShortcutDisplay, useShortcutRecorder } from '../../composables/useShortcutRecorder';
import { FLOATING_BALL_BUTTONS, FLOATING_BALL_MAX_BUTTONS } from '../../composables/floatingBallButtons';

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()

// ---- Desktop floating ball: enablement, edge hiding, visibility, idle spin, and menu items. ----
const ballButtons = computed(() => store.state.config.floating_ball_buttons ?? [])

function onToggleFloatingBall() {
  void store.setFloatingBallEnabled(!store.state.config.floating_ball_enabled)
}

function onToggleFloatingBallAutoHide() {
  void store.setFloatingBallAutoHide(!store.state.config.floating_ball_auto_hide)
}

function onToggleFloatingBallWithMain() {
  void store.setFloatingBallWithMain(!store.state.config.floating_ball_with_main)
}

function onToggleFloatingBallIdleSpin() {
  void store.setFloatingBallIdleSpin(!store.state.config.floating_ball_idle_spin)
}

async function addBallButton(id: string) {
  if (ballButtons.value.includes(id) || ballButtons.value.length >= FLOATING_BALL_MAX_BUTTONS) return
  await store.setFloatingBallButtons([...ballButtons.value, id])
}

async function removeBallButton(id: string) {
  await store.setFloatingBallButtons(ballButtons.value.filter((b) => b !== id))
}

async function moveBallButton(index: number, delta: number) {
  const target = index + delta
  if (target < 0 || target >= ballButtons.value.length) return
  const arr = [...ballButtons.value]
  ;[arr[index], arr[target]] = [arr[target], arr[index]]
  await store.setFloatingBallButtons(arr)
}

// ---- All seven configurable shortcuts share the same recorder logic. ----
const {
  value: shortcut,
  error: shortcutError,
  listening: shortcutListening,
  inputRef: shortcutInputRef,
  commit: commitShortcut,
  startListening: startListeningShortcut,
  onBlur: onShortcutBlur,
  onKeydown: onShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.global_shortcut),
  label: 'Global shortcut',
  save: (v) => store.setGlobalShortcut(v),
  showToast,
})

const {
  value: clipShortcut,
  saved: clipSavedShortcut,
  error: clipError,
  listening: clipListening,
  inputRef: clipInputRef,
  commit: commitClipShortcut,
  startListening: startListenClipShortcut,
  onBlur: onClipShortcutBlur,
  onKeydown: onClipShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.clipboard_shortcut ?? 'Ctrl+`'),
  label: 'Clipboard shortcut',
  save: (v) => store.setClipboardShortcut(v),
  showToast,
})

const {
  value: searchShortcut,
  error: searchError,
  listening: searchListening,
  inputRef: searchInputRef,
  commit: commitSearchShortcut,
  startListening: startListenSearchShortcut,
  onBlur: onSearchShortcutBlur,
  onKeydown: onSearchShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.search_shortcut ?? 'Ctrl+K'),
  label: 'Search shortcut',
  save: (v) => store.setSearchShortcut(v),
  showToast,
})

const {
  value: chatShortcut,
  error: chatError,
  listening: chatListening,
  inputRef: chatInputRef,
  commit: commitChatShortcut,
  startListening: startListenChatShortcut,
  onBlur: onChatShortcutBlur,
  onKeydown: onChatShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.chat_shortcut ?? 'Ctrl+Shift+K'),
  label: 'AI chat shortcut',
  save: (v) => store.setChatShortcut(v),
  showToast,
})

const {
  value: notesShortcut,
  error: notesError,
  listening: notesListening,
  inputRef: notesInputRef,
  commit: commitNotesShortcut,
  startListening: startListenNotesShortcut,
  onBlur: onNotesShortcutBlur,
  onKeydown: onNotesShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.notes_shortcut ?? 'Ctrl+Shift+N'),
  label: 'Notes shortcut',
  save: (v) => store.setNotesShortcut(v),
  showToast,
})

const {
  value: shotShortcut,
  error: shotError,
  listening: shotListening,
  inputRef: shotInputRef,
  commit: commitShotShortcut,
  startListening: startListenShotShortcut,
  onBlur: onShotShortcutBlur,
  onKeydown: onShotShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.shot_shortcut ?? 'Ctrl+Shift+A'),
  label: 'Screenshot shortcut',
  save: (v) => store.setShotShortcut(v),
  showToast,
})

const {
  value: recorderShortcut,
  error: recorderError,
  listening: recorderListening,
  inputRef: recorderInputRef,
  commit: commitRecorderShortcut,
  startListening: startListenRecorderShortcut,
  onBlur: onRecorderShortcutBlur,
  onKeydown: onRecorderShortcutKeydown,
} = useShortcutRecorder({
  initial: normalizeShortcutDisplay(store.state.config.recorder_shortcut ?? 'Ctrl+Shift+G'),
  label: 'GIF recorder shortcut',
  save: (v) => store.setRecorderShortcut(v),
  showToast,
})

// These refs are used by template bindings. Explicitly reference them because
// vue-tsc does not count template refs as reads when checking noUnusedLocals.
void shortcutInputRef
void clipInputRef
void searchInputRef
void chatInputRef
void notesInputRef
void shotInputRef
void recorderInputRef

// ---- Disabling a shortcut unregisters it but preserves its assigned keys. ----
type ShortcutKind = 'main' | 'clipboard' | 'search' | 'chat' | 'notes' | 'shot' | 'recorder'

function shortcutEnabled(kind: ShortcutKind): boolean {
  const c = store.state.config
  switch (kind) {
    case 'main':
      return c.global_shortcut_enabled
    case 'clipboard':
      return c.clipboard_shortcut_enabled
    case 'search':
      return c.search_shortcut_enabled
    case 'chat':
      return c.chat_shortcut_enabled
    case 'notes':
      return c.notes_shortcut_enabled
    case 'shot':
      return c.shot_shortcut_enabled
    case 'recorder':
      return c.recorder_shortcut_enabled
  }
}

const shortcutToggleBusy = ref(false)

async function toggleShortcutEnabled(kind: ShortcutKind) {
  if (shortcutToggleBusy.value) return
  const next = !shortcutEnabled(kind)
  shortcutToggleBusy.value = true
  try {
    await store.setShortcutEnabled(kind, next)
    const label = { main: 'Global', clipboard: 'Clipboard', search: 'Search', chat: 'AI chat', notes: 'Notes', shot: 'Screenshot', recorder: 'GIF recorder' }[kind]
    showToast(next ? `${label} shortcut enabled.` : `${label} shortcut disabled. Its key is saved and can be re-enabled at any time.`)
  } catch (e) {
    showToast(`Failed to update the shortcut: ${String(e)}`)
  } finally {
    shortcutToggleBusy.value = false
  }
}

// ---- Notification duration in seconds; changes apply to new notifications immediately. ----
const noticeSeconds = ref(5)
const noticeSaving = ref(false)

function commitNoticeDuration() {
  const sec = Math.min(60, Math.max(1, Math.round(Number(noticeSeconds.value) || 0)))
  noticeSeconds.value = sec
  if (!isTauri() || noticeSaving.value) return
  const ms = sec * 1000
  if (ms === (store.state.config.notice_duration_ms ?? 5000)) return
  noticeSaving.value = true
  void store
    .setNoticeDuration(ms)
    .then(() => showToast(`Notification duration set to ${sec} seconds.`))
    .catch((e) => showToast(`Failed to update the setting: ${String(e)}`))
    .finally(() => {
      noticeSaving.value = false
    })
}

// ---- Launch at startup ----
const autostartBusy = ref(false)
// Check the system state to verify that the app will actually launch at sign-in.
const autostartStatus = ref<AutostartStatus | null>(null)
// If enabled in settings but not registered with the system, offer a repair.
const autostartFailed = computed(
  () => !!autostartStatus.value && autostartStatus.value.configured && !autostartStatus.value.enabled,
)
const autostartFailReason = computed(() => {
  const s = autostartStatus.value
  if (!s) return ''
  if (s.os_disabled) return 'Disabled by Windows or security software in Startup settings.'
  if (!s.registered) return 'The startup entry is missing or the app path has changed.'
  return 'The app will not launch at startup.'
})

async function refreshAutostartStatus() {
  if (!isTauri()) return
  try {
    autostartStatus.value = await tauriApi.getRunAtStartup()
  } catch {
    // A failed status check does not change the setting.
  }
}

// Recreate the Run entry using the current executable path and clear the disabled flag.
async function repairAutostart() {
  if (autostartBusy.value) return
  autostartBusy.value = true
  try {
    await store.setRunAtStartup(true)
    await refreshAutostartStatus()
    showToast(autostartFailed.value ? 'Startup repair was incomplete. Check your security software settings.' : 'Startup entry repaired.')
  } catch (e) {
    showToast(`Startup repair failed: ${String(e)}`)
  } finally {
    autostartBusy.value = false
  }
}

async function onToggleAutostart() {
  if (autostartBusy.value) return
  autostartBusy.value = true
  const next = !store.state.config.run_at_startup
  try {
    await store.setRunAtStartup(next)
    await refreshAutostartStatus()
    showToast(next ? 'Launch at startup enabled.' : 'Launch at startup disabled.')
  } catch (e) {
    showToast(`Failed to update the setting: ${String(e)}`)
  } finally {
    autostartBusy.value = false
  }
}

onMounted(async () => {

  if (!isTauri()) return

  shortcut.value = normalizeShortcutDisplay(await tauriApi.getGlobalShortcut())

  clipShortcut.value = normalizeShortcutDisplay(store.state.config.clipboard_shortcut ?? 'Ctrl+`')

  clipSavedShortcut.value = clipShortcut.value

  searchShortcut.value = normalizeShortcutDisplay(store.state.config.search_shortcut ?? 'Ctrl+K')

  chatShortcut.value = normalizeShortcutDisplay(store.state.config.chat_shortcut ?? 'Ctrl+Shift+K')

  notesShortcut.value = normalizeShortcutDisplay(store.state.config.notes_shortcut ?? 'Ctrl+Shift+N')

  shotShortcut.value = normalizeShortcutDisplay(store.state.config.shot_shortcut ?? 'Ctrl+Shift+A')

  recorderShortcut.value = normalizeShortcutDisplay(store.state.config.recorder_shortcut ?? 'Ctrl+Shift+G')

  noticeSeconds.value = Math.round((store.state.config.notice_duration_ms ?? 5000) / 1000)

  void refreshAutostartStatus()

})
</script>

<template>
        <section id="sv-sec-general" class="sv-sec" aria-label="General">
          <h3 class="sv-sec-title">General</h3>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Launch at startup</span>
              <span class="setting-desc">Start in the background when you sign in to Windows. The main window stays hidden; open it from the system tray.</span>
              <span v-if="autostartFailed" class="autostart-warn">
                ⚠ Startup launch is unavailable: {{ autostartFailReason }}
                <button type="button" class="autostart-repair" :disabled="autostartBusy" @click="repairAutostart">
                  Enable again
                </button>
              </span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.run_at_startup"
              :class="{ on: store.state.config.run_at_startup }"
              :disabled="autostartBusy"
              @click="onToggleAutostart"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Notification duration</span>
              <span class="setting-desc">How long lower-right notifications stay visible (1–60 seconds; default 5). Click a notification to dismiss it immediately.</span>
            </div>
            <div class="num-edit with-unit">
              <input
                v-model.number="noticeSeconds"
                class="num-input"
                type="number"
                min="1"
                max="60"
                step="1"
                aria-label="Notification duration (seconds)"
                @change="commitNoticeDuration"
              />
              <span class="num-unit">sec</span>
            </div>
          </div>
        </section>

        <section id="sv-sec-ball" class="sv-sec" aria-label="Floating ball">
          <h3 class="sv-sec-title">Floating ball</h3>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Desktop floating ball</span>
              <span class="setting-desc">Show the floating ball when the main window is hidden or minimized. Enable “Show alongside the main window” below to keep it visible. Click to open its circular menu, double-click to show the main window, or right-click for a shortcut menu. Drag it to move; it partially hides at screen edges.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.floating_ball_enabled"
              :class="{ on: store.state.config.floating_ball_enabled }"
              @click="onToggleFloatingBall"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Auto-hide at screen edges</span>
              <span class="setting-desc">Release the ball near a screen edge to partially hide it. Hover over it to reveal the full ball; move away to hide it again.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.floating_ball_auto_hide"
              :class="{ on: store.state.config.floating_ball_auto_hide }"
              :disabled="!store.state.config.floating_ball_enabled"
              @click="onToggleFloatingBallAutoHide"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Show alongside the main window</span>
              <span class="setting-desc">Keep the floating ball on the desktop even while the main window is visible. Click it to open the menu.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.floating_ball_with_main"
              :class="{ on: store.state.config.floating_ball_with_main }"
              :disabled="!store.state.config.floating_ball_enabled"
              @click="onToggleFloatingBallWithMain"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-name">Keep spinning while idle</span>
              <span class="setting-desc">Keep the gyroscope ring and particles animated while idle. This uses more power and may increase heat. When off, idle animation pauses at 24 FPS and resumes during interaction.</span>
            </div>
            <button
              class="toggle"
              role="switch"
              type="button"
              :aria-checked="store.state.config.floating_ball_idle_spin"
              :class="{ on: store.state.config.floating_ball_idle_spin }"
              :disabled="!store.state.config.floating_ball_enabled"
              @click="onToggleFloatingBallIdleSpin"
            >
              <span class="toggle-knob"></span>
            </button>
          </div>

          <div class="setting-row fb-btn-row">
            <div class="setting-info">
              <span class="setting-name">Circular menu buttons</span>
              <span class="setting-desc">Choose up to {{ FLOATING_BALL_MAX_BUTTONS }} buttons for the floating ball menu. Add buttons below; move selected buttons up or down, or remove them. “AI Chat” opens a separate window or the main-window panel according to AI Assistant settings.</span>
            </div>
            <div class="fb-btn-cfg">
              <div class="fb-btn-list">
                <span v-for="(id, i) in ballButtons" :key="id" class="fb-chip">
                  {{ FLOATING_BALL_BUTTONS[id]?.label ?? id }}
                  <button type="button" class="fb-chip-btn" :disabled="i === 0" aria-label="Move up" @click="moveBallButton(i, -1)">↑</button>
                  <button type="button" class="fb-chip-btn" :disabled="i === ballButtons.length - 1" aria-label="Move down" @click="moveBallButton(i, 1)">↓</button>
                  <button type="button" class="fb-chip-btn fb-chip-remove" aria-label="Remove" @click="removeBallButton(id)">×</button>
                </span>
                <span v-if="ballButtons.length === 0" class="fb-chip-empty">No buttons selected (the floating ball can still be dragged or double-clicked).</span>
              </div>
              <div class="fb-btn-add">
                <button
                  v-for="(meta, id) in FLOATING_BALL_BUTTONS"
                  :key="id"
                  type="button"
                  class="fb-chip-add"
                  :disabled="ballButtons.includes(id) || ballButtons.length >= FLOATING_BALL_MAX_BUTTONS"
                  @click="addBallButton(id)"
                >
                  + {{ meta.label }}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section id="sv-sec-shortcut" class="sv-sec" aria-label="Keyboard shortcuts">
          <h3 class="sv-sec-title">Keyboard shortcuts</h3>
          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Show or hide the main window</span>
              <span class="setting-desc">Type a shortcut or record one by pressing the keys. Changes save automatically when there is no conflict. Use the switch to disable it temporarily without clearing the assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.global_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="shortcutInputRef"
                  v-model="shortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="shortcutListening"
                  placeholder="Ctrl+Shift+Space"
                  @keydown="onShortcutKeydown"
                  @keydown.enter="commitShortcut"
                  @blur="onShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListeningShortcut">
                  {{ shortcutListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.global_shortcut_enabled"
                :class="{ on: store.state.config.global_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.global_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('main')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="shortcutError" class="shortcut-error">{{ shortcutError }}</p>

          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Open clipboard history</span>
              <span class="setting-desc">Open clipboard history from any app. Use the switch to disable this shortcut temporarily without clearing its assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.clipboard_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="clipInputRef"
                  v-model="clipShortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="clipListening"
                  placeholder="Ctrl+`"
                  @keydown="onClipShortcutKeydown"
                  @keydown.enter="commitClipShortcut"
                  @blur="onClipShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListenClipShortcut">
                  {{ clipListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.clipboard_shortcut_enabled"
                :class="{ on: store.state.config.clipboard_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.clipboard_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('clipboard')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="clipError" class="shortcut-error">{{ clipError }}</p>

          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Open global search</span>
              <span class="setting-desc">Open global search from any app. Shortcut hints update automatically. Use the switch to disable this shortcut temporarily without clearing its assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.search_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="searchInputRef"
                  v-model="searchShortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="searchListening"
                  placeholder="Ctrl+K"
                  @keydown="onSearchShortcutKeydown"
                  @keydown.enter="commitSearchShortcut"
                  @blur="onSearchShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListenSearchShortcut">
                  {{ searchListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.search_shortcut_enabled"
                :class="{ on: store.state.config.search_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.search_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('search')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="searchError" class="shortcut-error">{{ searchError }}</p>

          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Open AI chat</span>
              <span class="setting-desc">Open AI chat from any app. Its drawer or window mode is set under AI Assistant. Use the switch to disable this shortcut temporarily without clearing its assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.chat_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="chatInputRef"
                  v-model="chatShortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="chatListening"
                  placeholder="Ctrl+Shift+K"
                  @keydown="onChatShortcutKeydown"
                  @keydown.enter="commitChatShortcut"
                  @blur="onChatShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListenChatShortcut">
                  {{ chatListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.chat_shortcut_enabled"
                :class="{ on: store.state.config.chat_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.chat_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('chat')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="chatError" class="shortcut-error">{{ chatError }}</p>

          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Create a note</span>
              <span class="setting-desc">Show the main window and create a note from any app. Use the switch to disable this shortcut temporarily without clearing its assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.notes_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="notesInputRef"
                  v-model="notesShortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="notesListening"
                  placeholder="Ctrl+Shift+N"
                  @keydown="onNotesShortcutKeydown"
                  @keydown.enter="commitNotesShortcut"
                  @blur="onNotesShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListenNotesShortcut">
                  {{ notesListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.notes_shortcut_enabled"
                :class="{ on: store.state.config.notes_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.notes_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('notes')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="notesError" class="shortcut-error">{{ notesError }}</p>

          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Take a screenshot</span>
              <span class="setting-desc">Select an area anywhere on screen and copy the screenshot to the clipboard. The image is not saved to disk. Use the switch to disable this shortcut temporarily without clearing its assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.shot_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="shotInputRef"
                  v-model="shotShortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="shotListening"
                  placeholder="Ctrl+Shift+A"
                  @keydown="onShotShortcutKeydown"
                  @keydown.enter="commitShotShortcut"
                  @blur="onShotShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListenShotShortcut">
                  {{ shotListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.shot_shortcut_enabled"
                :class="{ on: store.state.config.shot_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.shot_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('shot')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="shotError" class="shortcut-error">{{ shotError }}</p>

          <div class="setting-row shortcut-row">
            <div class="setting-info">
              <span class="setting-name">Record a GIF</span>
              <span class="setting-desc">Select an area anywhere on screen and record it as a GIF. Frame rate and duration are configurable. Use the switch to disable this shortcut temporarily without clearing its assigned keys.</span>
            </div>
            <div class="shortcut-edit" :class="{ off: !store.state.config.recorder_shortcut_enabled }">
              <div class="shortcut-input-wrap">
                <Keyboard :size="14" :stroke-width="2" class="shortcut-icon" />
                <input
                  ref="recorderInputRef"
                  v-model="recorderShortcut"
                  class="shortcut-input"
                  type="text"
                  spellcheck="false"
                  :readonly="recorderListening"
                  placeholder="Ctrl+Shift+G"
                  @keydown="onRecorderShortcutKeydown"
                  @keydown.enter="commitRecorderShortcut"
                  @blur="onRecorderShortcutBlur"
                />
                <button class="shortcut-record-btn" type="button" @click="startListenRecorderShortcut">
                  {{ recorderListening ? 'Press a key combination…' : 'Record' }}
                </button>
              </div>
              <button
                class="toggle shortcut-toggle"
                role="switch"
                type="button"
                :aria-checked="store.state.config.recorder_shortcut_enabled"
                :class="{ on: store.state.config.recorder_shortcut_enabled }"
                :disabled="shortcutToggleBusy"
                :title="store.state.config.recorder_shortcut_enabled ? 'Disable this shortcut' : 'Enable this shortcut'"
                @click="toggleShortcutEnabled('recorder')"
              >
                <span class="toggle-knob"></span>
              </button>
            </div>
          </div>
          <p v-if="recorderError" class="shortcut-error">{{ recorderError }}</p>
        </section>
</template>
