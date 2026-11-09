<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { FolderOpen, MessageSquare, Trash2, X } from 'lucide-vue-next'
import { useFocusTrap } from '../composables/useFocusTrap'
import { accentOf, iconSrc } from '../composables/useResourceIcon'
import { tauriApi, type ExtensionEntry } from '../api/tauri'
import { useStore } from '../stores/workbench'
import { effectiveKeepAliveIds, EXT_KEEP_ALIVE_CAP, hasKeepAliveSlot } from '../utils/extensionKeepAlive'
import ExtensionFeedbackDialog from './ExtensionFeedbackDialog.vue'

const props = defineProps<{ extension: ExtensionEntry | null }>()
const emit = defineEmits<{
  close: []
  uninstalled: []
}>()

const cardRef = ref<HTMLElement | null>(null)
const visible = computed(() => props.extension !== null)
useFocusTrap(visible, cardRef)

// Esc 关闭：弹窗可见时才响应（同屏多个 Esc 监听互不干扰，各自只管自己的弹窗）
function onKeydown(e: KeyboardEvent) {
  // 反馈弹窗叠在上面时由它自己处理 Esc，别把本弹窗一起关掉
  if (feedbackOpen.value) return
  if (e.key === 'Escape' && visible.value) emit('close')
}
onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))

const showToast = inject<(msg: string, action?: { label: string; onClick: () => void }) => void>(
  'showToast',
  () => {},
)

const ext = computed(() => props.extension)
const accent = computed(() => (ext.value ? accentOf(ext.value.name) : null))
const initial = computed(() => ((ext.value?.name ?? '?').charAt(0) || '?').toUpperCase())

const confirming = ref(false)
const uninstalling = ref(false)
const perms = ref<Record<string, boolean>>({})
const permsLoading = ref(false)
// 反馈只对市场扩展开放：「我的扩展」源码目录没有市场身份，提交会被服务端判为未知扩展
const feedbackOpen = ref(false)
const canFeedback = computed(() => ext.value?.source === 'installed')

watch(
  () => props.extension,
  async (e) => {
    confirming.value = false
    uninstalling.value = false
    perms.value = {}
    feedbackOpen.value = false
    if (!e || e.invalid) return
    permsLoading.value = true
    try {
      perms.value = await tauriApi.getExtensionPermissions(e.id)
    } catch {
      showToast('Could not load permission status. Reopen extension permissions and try again.')
    } finally {
      permsLoading.value = false
    }
  },
)

async function togglePermission(perm: string, granted: boolean) {
  if (!ext.value) return
  perms.value = { ...perms.value, [perm]: granted }
  try {
    await tauriApi.setExtensionPermission(ext.value.id, perm, granted)
  } catch (err) {
    showToast(`Could not update permission: ${String(err)}`)
    perms.value = { ...perms.value, [perm]: !granted }
  }
}

function kindLabel(kind: string): string {
  switch (kind) {
    case 'module':
      return 'Module'
    case 'view':
      return 'View'
    case 'window':
      return 'Window'
    case 'drawer':
      return 'Drawer'
    default:
      return kind || 'View'
  }
}

/** 权限标识 → 用户可读说明（未登记的权限原样展示） */
const PERMISSION_LABELS: Record<string, string> = {
  'data:read': 'Read host data (notes, todos, and quick access)',
  'data:write': 'Write host data',
  fs: 'Save files (to the system Downloads folder)',
  clipboard: 'Access the clipboard',
  network: 'Access the network',
  'open-url': 'Open web pages in the system default browser',
  'service:execute': 'Run a local backend (trust this version)',
  system: 'System-level operations (not currently supported by the host)',
  notify: 'Send system notifications',
  events: 'Broadcast extension events',
  'shared-storage': 'Read and write storage shared across extensions',
  titlebar: 'Show a widget in the app title bar (only while running in the background)',
}

function permissionLabel(perm: string): string {
  return PERMISSION_LABELS[perm] ?? perm
}

function runtimeLabel(runtime: 'web' | 'service'): string {
  return runtime === 'service' ? 'service (includes a local backend process)' : 'web (frontend only)'
}

// ---- AI 助手：manifest.agent.tools 声明的工具 + 每扩展授权（缺省允许） ----
const agentTools = computed(() => ext.value?.agent_tools ?? [])
const agentGranted = computed(() =>
  ext.value ? store.state.config.agent_ext_tool_grants?.[ext.value.id] ?? true : true,
)

function agentLevelLabel(level: 'read' | 'write' | 'dangerous'): string {
  return level === 'write' ? 'Write' : level === 'dangerous' ? 'Dangerous' : 'Read'
}

function toggleAgentGrant() {
  if (!ext.value) return
  store.setExtensionAgentGrant(ext.value.id, !agentGranted.value)
}

const store = useStore()

// 固定到侧栏：点击左栏菜单即在主区打开该扩展（view 形态）
const pinnedSidebar = computed(() =>
  ext.value ? (store.state.config.sidebar_extensions ?? []).includes(ext.value.id) : false,
)

function togglePinned() {
  if (!ext.value) return
  store.setSidebarExtension(ext.value.id, !pinnedSidebar.value)
}

// 默认打开方式：软件内（主区视图）/ 独立窗口 / 抽屉——
// 扩展中心点开、左栏固定菜单等入口都按此打开（默认「软件内」）
const openMode = computed(() => store.state.config.extension_open_modes?.[ext.value?.id ?? ''] ?? 'view')

const OPEN_MODES = [
  { value: 'view', label: 'In app' },
  { value: 'window', label: 'Separate window' },
  { value: 'drawer', label: 'Drawer' },
] as const

function setOpenMode(mode: (typeof OPEN_MODES)[number]['value']) {
  if (!ext.value) return
  store.setExtensionOpenMode(ext.value.id, mode)
  const label = OPEN_MODES.find((m) => m.value === mode)?.label ?? mode
  showToast(`Default open mode set to ${label}.`)
}

// 链接打开方式：扩展页里的外链（xhub.openExternal）怎么打开——
// 软件内 = 应用内置浏览器窗口（默认）；浏览器 = 系统默认浏览器
const linkMode = computed(() => store.state.config.extension_link_modes?.[ext.value?.id ?? ''] ?? 'inapp')

const LINK_MODES = [
  { value: 'inapp', label: 'In app' },
  { value: 'browser', label: 'Browser' },
] as const

function setLinkMode(mode: (typeof LINK_MODES)[number]['value']) {
  if (!ext.value) return
  store.setExtensionLinkMode(ext.value.id, mode)
  const label = LINK_MODES.find((m) => m.value === mode)?.label ?? mode
  showToast(`Extension links will now open in ${label}.`)
}

// ---- 常驻后台：view 实例切页不销毁（保状态/后台跑/免重载）；「随应用启动」是它的加强项 ----
const keepAliveOn = computed(() =>
  ext.value ? effectiveKeepAliveIds(store.state.config).includes(ext.value.id) : false,
)
const autostartOn = computed(() =>
  ext.value ? (store.state.config.extension_autostart ?? []).includes(ext.value.id) : false,
)

function toggleKeepAlive() {
  if (!ext.value) return
  const next = !keepAliveOn.value
  if (next && !hasKeepAliveSlot(store.state.config, ext.value.id)) {
    showToast(`You can keep up to ${EXT_KEEP_ALIVE_CAP} extensions running in the background. Stop one first.`)
    return
  }
  store.setExtensionKeepAlive(ext.value.id, next)
  // 常驻关了，随应用启动必然一起关（它隐含常驻）
  if (!next && autostartOn.value) store.setExtensionAutostart(ext.value.id, false)
  showToast(next ? 'Background running enabled. The extension stays active when you switch views.' : 'Background running disabled.')
}

function toggleAutostart() {
  if (!ext.value) return
  const next = !autostartOn.value
  if (!next) {
    store.setExtensionAutostart(ext.value.id, false)
    showToast('Launch at startup disabled. Background running remains enabled.')
    return
  }
  // 按钮在未开常驻时 disabled：能走到这里常驻必已开启，id 已在生效集合里——
  // 上限检查排除自己（已常驻的扩展加「随启动」不新增名额，满员也不该误拒）
  if (!hasKeepAliveSlot(store.state.config, ext.value.id)) {
    showToast(`You can keep up to ${EXT_KEEP_ALIVE_CAP} extensions running in the background. Stop one first.`)
    return
  }
  store.setExtensionAutostart(ext.value.id, true)
  showToast('Launch at startup enabled. This extension will run in the background when x-hub opens.')
}

/** 在系统文件管理器中打开扩展目录（开发调试用） */
const openingDir = ref(false)

async function openDir() {
  if (!ext.value) return
  openingDir.value = true
  try {
    await tauriApi.openExtensionDir(ext.value.id)
  } catch (e) {
    showToast(`Could not open folder: ${String(e)}`)
  } finally {
    openingDir.value = false
  }
}

async function confirmUninstall() {
  if (!ext.value) return
  uninstalling.value = true
  try {
    await tauriApi.uninstallExtension(ext.value.id)
    showToast(`Uninstalled “${ext.value.name}”.`)
    emit('uninstalled')
    emit('close')
  } catch (e) {
    showToast(`Could not uninstall extension: ${String(e)}`)
  } finally {
    uninstalling.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="mask">
      <div v-if="ext" class="modal-mask" @click.self="emit('close')">
        <div
          ref="cardRef"
          class="modal-card es-card"
          role="dialog"
          aria-label="Extension settings"
          aria-modal="true"
        >
          <header class="es-head">
            <div class="es-head-left">
              <div class="es-icon" :style="ext.icon ? {} : { background: accent?.soft }">
                <img
                  v-if="ext.icon"
                  :src="iconSrc(ext.icon)"
                  :alt="ext.name"
                  draggable="false"
                />
                <span v-else :style="{ color: accent?.text }">{{ initial }}</span>
              </div>
              <div class="es-head-meta">
                <h2 class="dialog-title">{{ ext.name }}</h2>
                <p class="es-sub">v{{ ext.version }} · {{ kindLabel(ext.kind) }} · {{ ext.id }}</p>
              </div>
            </div>
            <button class="icon-btn" title="Close" aria-label="Close" @click="emit('close')">
              <X :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
          </header>

          <div class="es-body">
            <section class="es-block">
              <h3 class="es-block-title">Info</h3>
              <dl class="es-kv">
                <div class="es-kv-row"><dt>Runtime</dt><dd>{{ runtimeLabel(ext.runtime) }}</dd></div>
                <div class="es-kv-row"><dt>Primary surface</dt><dd>{{ kindLabel(ext.kind) }}</dd></div>
                <div v-if="ext.surfaces.length" class="es-kv-row">
                  <dt>Available surfaces</dt><dd>{{ ext.surfaces.map(kindLabel).join(' / ') }}</dd>
                </div>
                <div v-if="ext.dir" class="es-kv-row es-dir-row">
                  <dt>Folder</dt>
                  <dd>
                    <span class="es-dir-path" :title="ext.dir">{{ ext.dir }}</span>
                    <button
                      class="es-dir-btn"
                      type="button"
                      :disabled="openingDir"
                      :title="`Open in File Explorer: ${ext.dir}`"
                      @click="openDir"
                    >
                      <FolderOpen :size="13" :stroke-width="2" aria-hidden="true" />
                      Open folder
                    </button>
                  </dd>
                </div>
              </dl>
              <p v-if="ext.description" class="es-desc">{{ ext.description }}</p>
            </section>

            <section class="es-block">
              <h3 class="es-block-title">Permissions</h3>
              <p v-if="ext.runtime === 'service'" class="es-empty">
                The local backend can access files you have permission to use and connect to the network. It is not isolated by the system sandbox. Enable it only if you trust the author and this version. Revoking network access also stops the backend.
                <template v-if="store.state.config.service_auto_trust">
                  “Automatically trust service extensions” is enabled in Settings → Extensions. New or updated backends can run without individual approval. Turn this extension off here to block its backend, even when the global setting is enabled.
                </template>
                <template v-else>Permission must be granted again after an extension update.</template>
              </p>
              <div v-if="Object.keys(perms).length" class="es-perm-list">
                <div v-for="p in Object.keys(perms)" :key="p" class="es-perm-row" :title="p">
                  <span class="es-perm-name">{{ permissionLabel(p) }}</span>
                  <button
                    class="toggle"
                    role="switch"
                    type="button"
                    :aria-checked="perms[p] ?? false"
                    :class="{ on: perms[p] ?? false }"
                    :disabled="permsLoading"
                    @click="togglePermission(p, !(perms[p] ?? false))"
                  >
                    <span class="toggle-knob"></span>
                  </button>
                </div>
              </div>
              <p v-else class="es-empty">No permissions requested.</p>
            </section>

            <!-- AI 助手：manifest.agent.tools 声明的工具展示 + 每扩展授权开关。
                 声明了工具才渲染整块（web 扩展一期不支持声明，永远看不到这块） -->
            <section v-if="!ext.invalid && agentTools.length > 0" class="es-block">
              <h3 class="es-block-title">AI assistant</h3>
              <p class="es-empty es-agent-hint">
                This extension declares {{ agentTools.length }} tools for the AI assistant. Each call follows the approval level declared by the tool; write and dangerous actions require your confirmation.
              </p>
              <ul class="es-agent-tools">
                <li v-for="t in agentTools" :key="t.name" class="es-agent-tool">
                  <span class="es-agent-level" :class="`lv-${t.level}`">{{ agentLevelLabel(t.level) }}</span>
                  <span class="es-agent-body">
                    <span class="es-agent-name">{{ t.name }}</span>
                    <span class="es-agent-desc">{{ t.description }}</span>
                  </span>
                </li>
              </ul>
              <div class="es-perm-row">
                <span class="es-setting-name">Allow the AI assistant to use this extension’s tools</span>
                <button
                  class="toggle"
                  role="switch"
                  type="button"
                  :aria-checked="agentGranted"
                  :class="{ on: agentGranted }"
                  @click="toggleAgentGrant"
                >
                  <span class="toggle-knob"></span>
                </button>
              </div>
              <p class="es-empty es-openmode-hint">When disabled, this extension’s tools are removed from the AI assistant’s available tools. Calls already approved for the current conversation are unaffected.</p>
            </section>

            <section v-if="!ext.invalid" class="es-block">
              <h3 class="es-block-title">Sidebar</h3>
              <div class="es-kv-row es-openmode-row">
                <dt>Open mode</dt>
                <dd>
                  <div class="es-seg" role="group" aria-label="Default open mode">
                    <button
                      v-for="m in OPEN_MODES"
                      :key="m.value"
                      class="es-seg-btn"
                      :class="{ active: openMode === m.value }"
                      type="button"
                      @click="setOpenMode(m.value)"
                    >
                      {{ m.label }}
                    </button>
                  </div>
                </dd>
              </div>
              <div class="es-kv-row es-openmode-row">
                <dt>Link open mode</dt>
                <dd>
                  <div class="es-seg" role="group" aria-label="Link open mode">
                    <button
                      v-for="m in LINK_MODES"
                      :key="m.value"
                      class="es-seg-btn"
                      :class="{ active: linkMode === m.value }"
                      type="button"
                      @click="setLinkMode(m.value)"
                    >
                      {{ m.label }}
                    </button>
                  </div>
                </dd>
              </div>
              <div class="es-perm-row">
                <span class="es-setting-name">Pin this extension in the sidebar</span>
                <button
                  class="toggle"
                  role="switch"
                  type="button"
                  :aria-checked="pinnedSidebar"
                  :class="{ on: pinnedSidebar }"
                  @click="togglePinned"
                >
                  <span class="toggle-knob"></span>
                </button>
              </div>
              <p class="es-empty es-openmode-hint">Opening this extension from the Extension Center or its sidebar shortcut uses the open mode above (in-app by default). External links opened by the extension use the link open mode above (the in-app browser by default).</p>
            </section>

            <section v-if="!ext.invalid" class="es-block">
              <h3 class="es-block-title">Background</h3>
              <div class="es-perm-row">
                <span class="es-setting-name">Keep running in the background</span>
                <button
                  class="toggle"
                  role="switch"
                  type="button"
                  :aria-checked="keepAliveOn"
                  :class="{ on: keepAliveOn }"
                  @click="toggleKeepAlive"
                >
                  <span class="toggle-knob"></span>
                </button>
              </div>
              <p class="es-empty es-openmode-hint">When enabled, opening this extension in-app keeps it running as you switch views, preserving unfinished work and background tasks. Right-click its sidebar shortcut to reload it (useful if it goes blank or encounters an error) or end its current background session (the setting stays on, so it resumes next time you open it). Each running extension uses a separate renderer and tens of MB of memory; up to {{ EXT_KEEP_ALIVE_CAP }} can run at once. After restarting the app, open the extension once to resume background running. Turn this setting off to stop it from staying active.</p>
              <div class="es-perm-row">
                <span class="es-setting-name">Run at startup</span>
                <button
                  class="toggle"
                  role="switch"
                  type="button"
                  :aria-checked="autostartOn"
                  :class="{ on: autostartOn }"
                  :disabled="!keepAliveOn"
                  @click="toggleAutostart"
                >
                  <span class="toggle-knob"></span>
                </button>
              </div>
              <p class="es-empty es-openmode-hint">{{ keepAliveOn ? 'Run this extension in the background whenever x-hub starts. It will resume automatically after an app restart.' : 'Enable “Keep running in the background” first.' }}</p>
            </section>
          </div>

          <footer class="es-foot">
            <template v-if="!confirming">
              <div class="es-foot-row">
                <button
                  v-if="canFeedback"
                  class="ghost-btn"
                  type="button"
                  @click="feedbackOpen = true"
                >
                  <MessageSquare :size="14" :stroke-width="2" aria-hidden="true" />
                  Send feedback
                </button>
                <span class="es-foot-spacer"></span>
                <button class="es-danger-btn" type="button" @click="confirming = true">
                  <Trash2 :size="14" :stroke-width="2" aria-hidden="true" />
                  Uninstall extension
                </button>
              </div>
            </template>
            <template v-else>
              <p class="es-confirm-tip">
                {{ ext.runtime === 'service' ? 'This will uninstall the extension and stop its local backend process.' : 'This will uninstall the extension.' }}
                This action cannot be undone.
              </p>
              <div class="es-confirm-actions">
                <button
                  class="ghost-btn"
                  type="button"
                  :disabled="uninstalling"
                  @click="confirming = false"
                >
                  Cancel
                </button>
                <button
                  class="es-danger-btn"
                  type="button"
                  :disabled="uninstalling"
                  @click="confirmUninstall"
                >
                  {{ uninstalling ? 'Uninstalling…' : 'Confirm uninstall' }}
                </button>
              </div>
            </template>
          </footer>
        </div>
      </div>
    </Transition>

    <!-- 提交反馈弹窗：与设置弹窗叠加（焦点陷阱按栈顶让位，Esc 由它自己消费） -->
    <ExtensionFeedbackDialog
      v-model:visible="feedbackOpen"
      :ext-id="ext?.id ?? ''"
      :ext-name="ext?.name ?? null"
      :ext-version="ext?.version ?? null"
      :runtime="ext?.runtime ?? null"
    />
  </Teleport>
</template>

<style scoped>
.es-card {
  width: min(640px, 92vw);
  max-height: min(800px, 88vh);
  display: flex;
  flex-direction: column;
  gap: 16px;
  /* 弹窗外壳自带 24px padding，本卡片自排内边距：清零避免与头/体/脚叠加成双份 */
  padding: 0;
}
.es-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-shrink: 0;
  /* 卡片 padding 清零后头/体/脚各自排 18px 内边距；这一条原先漏了，导致图标与关闭按钮
     贴死在卡片左上/右上角（体/脚都有 18px，表头没有） */
  padding: 18px 18px 0;
}
.es-head-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}
.es-icon {
  width: 44px;
  height: 44px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  font-size: 1.125rem;
  font-weight: 700;
  overflow: hidden;
}
.es-icon img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.es-head-meta {
  min-width: 0;
}
.es-head-meta .dialog-title {
  margin: 0;
}
.es-sub {
  margin: 2px 0 0;
  font-size: 0.75rem;
  color: var(--text-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.es-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  /* 卡片 padding 清零后由头/体/脚各自排 18px 内边距，与市场详情、发布弹窗全一致 */
  padding: 0 18px;
}
.es-block {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-lg);
  background: var(--bg-card-soft);
}
.es-block-title {
  margin: 0;
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.es-kv {
  margin: 0;
  display: flex;
  flex-direction: column;
}
.es-kv-row {
  display: flex;
  gap: 12px;
  padding: 6px 0;
  font-size: 0.8125rem;
}
.es-kv-row + .es-kv-row {
  border-top: 1px solid var(--border-soft);
}
.es-kv-row dt {
  flex-shrink: 0;
  width: 72px;
  color: var(--text-3);
}
.es-kv-row dd {
  margin: 0;
  color: var(--text-1);
}
/* 目录行：路径等宽省略 + 「打开目录」按钮（开发调试快速定位扩展所在目录） */
.es-dir-row dd {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  width: 100%;
}
.es-dir-path {
  flex: 1;
  min-width: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.75rem;
  line-height: 1.5;
  color: var(--text-2);
  /* 最多两行：路径尾部（目录名本身）比开头更需要看得见，故不做单行省略号截断 */
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  word-break: break-all;
  overflow: hidden;
}
.es-dir-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  padding: 3px 10px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--text-2);
  font-size: 0.6875rem;
  font-weight: 600;
  line-height: 1.5;
  white-space: nowrap;
  cursor: pointer;
  transition: background 150ms ease-out, color 150ms ease-out, border-color 150ms ease-out;
}
.es-dir-btn:hover:not(:disabled) {
  background: var(--brand-50);
  border-color: var(--brand-500);
  color: var(--brand-500);
}
.es-dir-btn:disabled {
  opacity: 0.6;
  cursor: default;
}
.es-desc {
  margin: 2px 0 0;
  padding-top: 10px;
  border-top: 1px solid var(--border-soft);
  font-size: 0.8125rem;
  color: var(--text-2);
  line-height: 1.55;
}
.es-perm-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.es-perm-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 7px 0;
}
.es-perm-row + .es-perm-row {
  border-top: 1px solid var(--border-soft);
}
.es-perm-name {
  font-size: 0.8125rem;
  color: var(--text-1);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
.es-agent-hint {
  padding: 2px 0 6px;
}
.es-agent-tools {
  list-style: none;
  margin: 0 0 4px;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.es-agent-tool {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.es-agent-level {
  flex-shrink: 0;
  margin-top: 1px;
  font-size: 0.6875rem;
  line-height: 1;
  padding: 3px 6px;
  border-radius: var(--radius-pill);
  font-weight: 600;
}
.es-agent-level.lv-read {
  color: var(--c-green-ink);
  background: var(--c-green-soft);
}
.es-agent-level.lv-write {
  color: var(--c-orange-ink);
  background: var(--c-orange-soft);
}
.es-agent-level.lv-dangerous {
  color: var(--c-red-ink);
  background: var(--c-red-soft);
}
.es-agent-body {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.es-agent-name {
  font-size: 0.8125rem;
  color: var(--text-1);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
.es-agent-desc {
  font-size: 0.75rem;
  color: var(--text-3);
  line-height: 1.45;
}
.toggle {
  flex-shrink: 0;
  width: 40px;
  height: 22px;
  border: none;
  border-radius: var(--radius-pill);
  background: var(--border-strong);
  position: relative;
  cursor: pointer;
  padding: 0;
  transition: background 0.18s;
}
.toggle.on {
  background: var(--brand-500);
}
.toggle:disabled {
  opacity: 0.6;
  cursor: default;
}
.toggle-knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  box-shadow: var(--shadow-dock);
  transition: transform 0.18s;
}
.toggle.on .toggle-knob {
  transform: translateX(18px);
}
.es-empty {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--text-3);
}
.es-openmode-hint {
  font-size: 0.75rem;
  color: var(--text-4, var(--text-3));
}
/* 侧边栏区块：普通字体（避免复用权限行的等宽字），打开方式分段选择 */
.es-setting-name {
  font-size: 0.8125rem;
  color: var(--text-1);
}
/* 打开方式 / 链接打开方式：标签在左、分段控件贴右，与「在左侧栏固定此扩展」那行对齐；
   dt 不锁 72px（「链接打开方式」六字会折行），改 nowrap 自适应 */
.es-openmode-row {
  align-items: center;
  justify-content: space-between;
}
.es-openmode-row dt {
  width: auto;
  white-space: nowrap;
}
.es-seg {
  display: inline-flex;
  gap: 2px;
  padding: 2px;
  border-radius: var(--radius-pill);
  background: var(--bg-card-soft);
  border: 1px solid var(--border-soft);
}
.es-seg-btn {
  padding: 3px 12px;
  border: 0;
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--text-3);
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 150ms ease-out, color 150ms ease-out;
}
.es-seg-btn:hover {
  color: var(--text-1);
}
.es-seg-btn.active {
  background: var(--brand-500);
  color: #fff;
}
.es-foot {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex-shrink: 0;
  padding: 4px 18px 18px;
}
.es-foot-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.es-foot-spacer {
  flex: 1;
}
.es-confirm-tip {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--c-red);
}
.es-confirm-actions {
  display: flex;
  gap: 8px;
}
.es-danger-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 16px;
  border: 1px solid color-mix(in srgb, var(--c-red) 45%, transparent);
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--c-red);
  font-size: 0.8125rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.18s, color 0.18s, transform 0.18s;
}
.es-danger-btn:hover:not(:disabled) {
  background: var(--c-red);
  color: #fff;
  transform: translateY(-1px);
}
.es-danger-btn:active:not(:disabled) {
  transform: scale(0.96);
}
.es-danger-btn:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
