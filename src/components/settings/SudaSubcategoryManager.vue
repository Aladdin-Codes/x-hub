<script setup lang="ts">
// Manage resource subcategories by type: rename, reorder, set defaults, and delete.
// Deleting a category reassigns its resources to the default category in one backend transaction.
// Names may use slash-separated hierarchy, such as imported bookmark folders.
import { computed, inject, onBeforeUnmount, ref } from 'vue'
import { Check, GripVertical, ListChecks, Plus, Star, Trash2, X } from 'lucide-vue-next'
import { useStore } from '../../stores/workbench'
import type { ResourceSubcategory } from '../../api/tauri'
import { reportClientError } from '../../utils/error-report'

const store = useStore()
const showToast = inject<(msg: string) => void>('showToast', () => {})

const KINDS: { kind: 'app' | 'web' | 'file'; label: string; hint: string }[] = [
  { kind: 'app', label: 'Apps', hint: 'Discovered or added manually' },
  { kind: 'web', label: 'Web', hint: 'Bookmarks; imported folders may use / hierarchy' },
  { kind: 'file', label: 'Files', hint: 'Includes 7 built-in categories' },
]

/** Draft name for each category type. */
const drafts = ref<Record<string, string>>({})
const newName = (kind: string) => drafts.value[kind] ?? ''
function setNewName(kind: string, v: string) {
  drafts.value = { ...drafts.value, [kind]: v }
}

const error = ref('')

async function add(kind: 'app' | 'web' | 'file') {
  const name = newName(kind).trim()
  if (!name) return
  try {
    await store.addSubcategory(kind, name)
    setNewName(kind, '')
    error.value = ''
  } catch (e) {
    error.value = String(e).replace(/^DUP:/, '').trim() || String(e)
  }
}

/** Save inline renames on blur or Enter; restore invalid or unchanged values. */
async function rename(sub: { id: number; name: string }, ev: Event) {
  const el = ev.target as HTMLInputElement
  const next = el.value.trim()
  if (!next || next === sub.name) {
    el.value = sub.name
    return
  }
  try {
    await store.editSubcategory(sub.id, next)
    error.value = ''
  } catch (e) {
    el.value = sub.name
    error.value = String(e).replace(/^DUP:/, '').trim() || String(e)
  }
}

/** Two-step deletion confirmation: confirm within 3 seconds after the first click. */
const confirmDeleteId = ref<number | null>(null)
let confirmTimer = 0
function onDeleteClick(id: number) {
  if (confirmDeleteId.value === id) {
    confirmDeleteId.value = null
    window.clearTimeout(confirmTimer)
    store
      .removeSubcategory(id)
      .then(() => (error.value = ''))
      .catch((e) => (error.value = String(e)))
    return
  }
  confirmDeleteId.value = id
  window.clearTimeout(confirmTimer)
  confirmTimer = window.setTimeout(() => (confirmDeleteId.value = null), 3000)
}

async function onSetDefault(id: number) {
  try {
    await store.setDefaultSubcategory(id)
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}

// ---- Batch deletion: select categories, select all, and confirm deletion. ----
const batchKind = ref<'app' | 'web' | 'file' | null>(null)
const batchChecked = ref<Set<number>>(new Set())
const batchDeleting = ref(false)
/** Whether batch deletion is awaiting its second confirmation click. */
const batchConfirmArmed = ref(false)
let batchConfirmTimer = 0

function enterBatch(kind: 'app' | 'web' | 'file') {
  batchKind.value = kind
  batchChecked.value = new Set()
  batchConfirmArmed.value = false
}

function exitBatch() {
  batchKind.value = null
  batchChecked.value = new Set()
  batchConfirmArmed.value = false
  window.clearTimeout(batchConfirmTimer)
}

function toggleBatch(id: number) {
  const next = new Set(batchChecked.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  batchChecked.value = next
  batchConfirmArmed.value = false
}

/** Visible rows for the current category type, used by select-all. */
function batchRows(kind: 'app' | 'web' | 'file'): ResourceSubcategory[] {
  const ids = displayIds(kind)
  const map = new Map(store.subcategoriesOf(kind).map((s) => [s.id, s]))
  return ids.map((id) => map.get(id)).filter((s): s is ResourceSubcategory => !!s)
}

function batchToggleAll(kind: 'app' | 'web' | 'file') {
  const rows = batchRows(kind)
  const all = rows.length > 0 && rows.every((s) => batchChecked.value.has(s.id))
  batchChecked.value = all ? new Set() : new Set(rows.map((s) => s.id))
  batchConfirmArmed.value = false
}

/** Delete selected categories one at a time; each backend operation is transactional. */
function onBatchDeleteClick() {
  if (batchDeleting.value) return
  if (batchConfirmArmed.value) {
    batchConfirmArmed.value = false
    window.clearTimeout(batchConfirmTimer)
    void doBatchDelete()
    return
  }
  batchConfirmArmed.value = true
  window.clearTimeout(batchConfirmTimer)
  batchConfirmTimer = window.setTimeout(() => (batchConfirmArmed.value = false), 3000)
}

async function doBatchDelete() {
  const kind = batchKind.value
  if (!kind) return
  // Snapshot before deleting because each operation updates the store.
  const snapshot = batchRows(kind).filter((s) => batchChecked.value.has(s.id))
  if (snapshot.length === 0) return
  batchDeleting.value = true
  let done = 0
  const failed: string[] = []
  for (const s of snapshot) {
    try {
      await store.removeSubcategory(s.id)
      done++
    } catch (e) {
      failed.push(s.name)
      void reportClientError('Failed to delete categories in batch', e)
    }
  }
  batchDeleting.value = false
  batchChecked.value = new Set()
  const parts = [`Deleted ${done} categories. Their resources were reassigned to the default category.`]
  if (failed.length > 0) parts.push(`Failed to delete ${failed.length}: ${failed.join(', ')}`)
  showToast(parts.join(' '))
  if (failed.length > 0) error.value = `Some categories could not be deleted: ${failed.join(', ')}`
}

// ---- Pointer-based sorting (native window file drops conflict with HTML5 drag and drop). ----
const dragKind = ref<'app' | 'web' | 'file' | null>(null)
/** Local preview order for the category being dragged; the store is refreshed after saving. */
const preview = ref<Record<string, number[]>>({})
let dragFromIndex = 0

function displayIds(kind: 'app' | 'web' | 'file'): number[] {
  const override = preview.value[kind]
  if (override) return override
  return store.subcategoriesOf(kind).map((s) => s.id)
}

function onHandleDown(e: PointerEvent, kind: 'app' | 'web' | 'file', index: number) {
  // Disable sorting in selection mode to avoid conflicting gestures.
  if (batchKind.value === kind) return
  e.preventDefault()
  dragKind.value = kind
  dragFromIndex = index
  window.addEventListener('pointermove', onDragMove)
  window.addEventListener('pointerup', onDragUp)
}

function onDragMove(e: PointerEvent) {
  const kind = dragKind.value
  if (!kind) return
  const el = document.elementFromPoint(e.clientX, e.clientY)?.closest('[data-sub-row]')
  if (!(el instanceof HTMLElement)) return
  if (el.dataset.kind !== kind) return
  const to = Number(el.dataset.index)
  const ids = displayIds(kind)
  if (Number.isNaN(to) || to === dragFromIndex || to < 0 || to >= ids.length) return
  const next = [...ids]
  next.splice(dragFromIndex, 1)
  next.splice(to, 0, ids[dragFromIndex])
  dragFromIndex = to
  preview.value = { ...preview.value, [kind]: next }
}

function onDragUp() {
  const kind = dragKind.value
  window.removeEventListener('pointermove', onDragMove)
  window.removeEventListener('pointerup', onDragUp)
  dragKind.value = null
  if (!kind) return
  const ids = displayIds(kind)
  preview.value = {}
  store
    .reorderSubcategories(kind, ids)
    .then(() => (error.value = ''))
    .catch((e) => (error.value = String(e)))
}

onBeforeUnmount(() => {
  window.removeEventListener('pointermove', onDragMove)
  window.removeEventListener('pointerup', onDragUp)
  window.clearTimeout(confirmTimer)
  window.clearTimeout(batchConfirmTimer)
})

/** Display the drag preview order for the active category and store order for the rest. */
const groups = computed(() =>
  KINDS.map(({ kind, label, hint }) => {
    const ids = displayIds(kind)
    const map = new Map(store.subcategoriesOf(kind).map((s) => [s.id, s]))
    const list = ids.map((id) => map.get(id)).filter((s): s is ResourceSubcategory => !!s)
    return { kind, label, hint, list }
  }),
)
</script>

<template>
  <div class="sub-mgr">
    <p v-if="error" class="sub-mgr-error">{{ error }}</p>
    <div v-for="g in groups" :key="g.kind" class="sub-mgr-group">
      <div class="sub-mgr-head">
        <span class="sub-mgr-title">{{ g.label }}</span>
        <span v-if="batchKind !== g.kind" class="sub-mgr-hint">{{ g.hint }}</span>
        <!-- Selection toolbar: count, select all, confirmed delete, and exit. -->
        <template v-else>
          <span class="sub-mgr-hint">{{ batchChecked.size }} selected</span>
          <button class="ghost-btn sub-mgr-batch-btn" type="button" @click="batchToggleAll(g.kind)">
            {{ batchRows(g.kind).length > 0 && batchRows(g.kind).every((s) => batchChecked.has(s.id)) ? 'Deselect all' : 'Select all' }}
          </button>
          <button
            class="ghost-btn sub-mgr-batch-btn danger"
            :class="{ armed: batchConfirmArmed }"
            type="button"
            :disabled="batchChecked.size === 0 || batchDeleting"
            :title="batchConfirmArmed ? 'Click again to confirm deletion' : 'Delete selected categories; resources will move to the default category'"
            @click="onBatchDeleteClick"
          >
            <Check v-if="batchConfirmArmed" :size="12" :stroke-width="2.6" />
            {{ batchDeleting ? 'Deleting…' : batchConfirmArmed ? 'Confirm deletion' : `Delete selected (${batchChecked.size})` }}
          </button>
          <button class="sub-mgr-btn" type="button" title="Exit batch deletion" aria-label="Exit batch deletion" @click="exitBatch">
            <X :size="13" :stroke-width="2.2" />
          </button>
        </template>
        <button
          v-if="batchKind !== g.kind && g.list.length > 0"
          class="ghost-btn sub-mgr-batch-btn"
          type="button"
          title="Select multiple categories to delete them together"
          @click="enterBatch(g.kind)"
        >
          <ListChecks :size="12" :stroke-width="2" />
          Delete multiple
        </button>
      </div>
      <div
        v-for="(s, i) in g.list"
        :key="s.id"
        class="sub-mgr-row"
        data-sub-row
        :data-kind="g.kind"
        :data-index="i"
      >
        <template v-if="batchKind === g.kind">
          <button
            class="sub-mgr-check"
            :class="{ on: batchChecked.has(s.id) }"
            type="button"
            role="checkbox"
            :aria-checked="batchChecked.has(s.id)"
            :aria-label="`Select category ${s.name}`"
            @click="toggleBatch(s.id)"
          >
            <Check v-if="batchChecked.has(s.id)" :size="11" :stroke-width="3" />
          </button>
        </template>
        <span
          v-else
          class="sub-mgr-drag"
          title="Drag to reorder"
          @pointerdown="onHandleDown($event, g.kind, i)"
        >
          <GripVertical :size="13" :stroke-width="2" />
        </span>
        <input
          class="sub-mgr-name"
          :value="s.name"
          spellcheck="false"
          maxlength="60"
          :title="s.name"
          @change="rename(s, $event)"
          @keydown.enter="($event.target as HTMLInputElement).blur()"
        />
        <button
          class="sub-mgr-btn"
          :class="{ 'is-default': s.is_default }"
          type="button"
          :title="s.is_default ? 'Default category: new resources are assigned here unless specified. Click to change.' : 'Set as the default category'"
          @click="onSetDefault(s.id)"
        >
          <Star :size="13" :stroke-width="2.2" />
        </button>
        <button
          class="sub-mgr-btn sub-mgr-del"
          :class="{ 'is-confirm': confirmDeleteId === s.id }"
          type="button"
          :title="
            confirmDeleteId === s.id
              ? 'Click again to confirm deletion; resources will move to the default category'
              : 'Delete category (two clicks to confirm); resources will move to the default category'
          "
          :aria-label="confirmDeleteId === s.id ? 'Confirm category deletion' : 'Delete category'"
          @click="onDeleteClick(s.id)"
        >
          <!-- Replace the trash icon with a check mark after the first click. -->
          <Check v-if="confirmDeleteId === s.id" :size="13" :stroke-width="2.6" />
          <Trash2 v-else :size="13" :stroke-width="2.2" />
        </button>
      </div>
      <div v-if="batchKind !== g.kind" class="sub-mgr-row sub-mgr-add">
        <span class="sub-mgr-drag is-placeholder"><Plus :size="13" :stroke-width="2" /></span>
        <input
          class="sub-mgr-name"
          :value="newName(g.kind)"
          type="text"
          maxlength="60"
          placeholder="New category name; use / for nesting (for example, Development/Frontend). Press Enter to add."
          spellcheck="false"
          @input="setNewName(g.kind, ($event.target as HTMLInputElement).value)"
          @keydown.enter="add(g.kind)"
        />
        <button class="sub-mgr-btn sub-mgr-add-btn" type="button" title="Add category" @click="add(g.kind)">
          Add
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sub-mgr {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-top: 4px;
}

.sub-mgr-error {
  margin: 0;
  font-size: 12px;
  color: var(--c-red);
}

.sub-mgr-group {
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 10px 12px 12px;
  background: var(--bg-card-soft);
}

.sub-mgr-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.sub-mgr-title {
  font-size: 13px;
  font-weight: 650;
  color: var(--text-1);
}

.sub-mgr-hint {
  font-size: 11px;
  color: var(--text-3);
  margin-right: auto;
}

.sub-mgr-batch-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  font-size: 11.5px;
  color: var(--text-2);
}

.sub-mgr-batch-btn.danger {
  color: var(--c-red);
}

.sub-mgr-batch-btn.danger.armed {
  background: var(--c-red);
  border-color: var(--c-red);
  color: var(--text-on-accent);
}

.sub-mgr-batch-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.sub-mgr-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 0;
}

.sub-mgr-drag {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 22px;
  color: var(--text-4);
  cursor: grab;
  touch-action: none;
}

.sub-mgr-drag:active {
  cursor: grabbing;
}

.sub-mgr-drag.is-placeholder {
  cursor: default;
  opacity: 0.45;
}

/* 勾选模式的行首选择框（占拖拽把手位） */
.sub-mgr-check {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  margin: 0 2px;
  border: 1.5px solid var(--border-strong);
  border-radius: 5px;
  background: var(--bg-card-solid);
  color: var(--text-on-accent);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.sub-mgr-check.on {
  background: var(--brand-500);
  border-color: var(--brand-500);
}

.sub-mgr-name {
  flex: 1;
  min-width: 0;
  height: 28px;
  padding: 0 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-1);
  font-size: 12.5px;
  outline: none;
  transition: border-color 0.15s, background 0.15s;
}

.sub-mgr-name:hover {
  background: var(--bg-card);
}

.sub-mgr-name:focus {
  border-color: var(--brand-500);
  background: var(--bg-card);
  box-shadow: 0 0 0 2px var(--brand-50);
}

.sub-mgr-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-4);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.sub-mgr-btn:hover {
  background: var(--bg-card);
  color: var(--text-2);
}

.sub-mgr-btn.is-default {
  color: var(--c-yellow);
}

.sub-mgr-del.is-confirm {
  background: var(--c-red);
  color: var(--text-on-accent);
}

.sub-mgr-add {
  margin-top: 4px;
}

.sub-mgr-add .sub-mgr-name {
  border-color: var(--border-soft);
  background: var(--bg-card);
}

.sub-mgr-add-btn {
  width: auto;
  padding: 0 10px;
  font-size: 11.5px;
  color: var(--text-2);
  border: 1px solid var(--border-soft);
}
</style>
