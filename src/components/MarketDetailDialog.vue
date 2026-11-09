<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ChevronLeft, ChevronRight, ExternalLink, ImageOff, MessageSquare, Package, PenLine, Reply, Shield, ShieldAlert, ShieldCheck, X } from 'lucide-vue-next'
import { useFocusTrap } from '../composables/useFocusTrap'
import { isTauri, tauriApi, type ExtensionFeedback, type MarketExtension } from '../api/tauri'
import { accentOf } from '../composables/useResourceIcon'
import { displayCategory, installCountText, isOfficialExt } from '../utils/extensionMeta'
import { feedbackKindLabel, feedbackTimeText } from '../utils/feedback'
import ExtensionFeedbackDialog from './ExtensionFeedbackDialog.vue'

const props = defineProps<{
  extension: MarketExtension | null
  /** 底部主按钮文案（安装 / 更新 / 已安装） */
  actionLabel: string
  /** 底部主按钮是否禁用 */
  actionDisabled?: boolean
  /** 底部主按钮标题提示（如宿主要求过高） */
  actionTitle?: string
}>()

const emit = defineEmits<{
  action: []
  close: []
}>()

const visible = computed(() => !!props.extension)
const cardRef = ref<HTMLElement | null>(null)
useFocusTrap(visible, cardRef)

/** 图标是否可显示（https URL 加载失败则回退首字母） */
const iconFailed = ref(false)
/** 截图放大预览：点主图打开，点任意处关闭；开着时 ←/→ 继续翻页 */
const shotPreview = ref(false)
/** 截图展示区当前主图下标（缩略图条/左右箭头切换） */
const shotIndex = ref(0)
/** 加载失败的截图（远端 URL 失效时剔出展示区，不留破图） */
const shotFailed = ref<Set<string>>(new Set())
const m = computed(() => props.extension)

/** 可展示的截图（清单里 screenshots 为空 = 该扩展没传过展示图） */
const shots = computed(() =>
  ((m.value?.screenshots ?? []) as string[]).filter((s) => !shotFailed.value.has(s)),
)
const currentShot = computed(() => shots.value[shotIndex.value] ?? '')

/** 切换主图（缩略图点选 / 左右箭头 / 键盘翻页共用；循环） */
function showShot(i: number) {
  const n = shots.value.length
  if (!n) return
  shotIndex.value = ((i % n) + n) % n
}

function onShotError(url: string) {
  shotFailed.value.add(url)
  // 当前主图的图挂了就退回第一张可用的
  if (shotIndex.value >= shots.value.length) shotIndex.value = 0
}

// 换扩展时重置展示区状态（同一个弹窗实例复用）
// ⚠️ 坏图名单必须一起清空：它是「这张 URL 加载失败过」的记忆，跨扩展留着会让
//    「地址恰好与之前那张坏图相同」的新扩展截图被无辜藏掉，看起来像没传截图
watch(
  () => props.extension?.id,
  () => {
    shotIndex.value = 0
    shotPreview.value = false
    shotFailed.value = new Set()
  },
)

function initial(): string {
  const e = m.value
  return (e?.name || e?.id || '?').charAt(0).toUpperCase()
}

function formatSize(bytes: number): string {
  if (!bytes) return '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function openHomepage() {
  const url = m.value?.homepage
  if (url && isTauri()) void tauriApi.openExternal(url)
}

// ---- 安装前显式告知（无沙箱前提下的唯一承诺，见 ADR 0007 / PRD §7.2 ②） ----
/** 高危能力：审核时逐项批准，需在安装前单独明示 */
const HIGH_RISK_PERMISSIONS = new Set(['fs', 'network', 'system', 'clipboard', 'open-url'])

/** 权限 → 人话（清单里的 permissions 由服务端发布时从 manifest 写入） */
const PERMISSION_LABELS: Record<string, string> = {
  'data:read': 'Read your notes, to-dos, Quick access items, and other data',
  'data:write': 'Modify your notes, to-dos, Quick access items, and other data',
  fs: 'Read and write files on your computer',
  clipboard: 'Read clipboard contents',
  network: 'Connect to the internet',
  'open-url': 'Open web pages in your default browser',
  system: 'Access system information',
  notify: 'Show notifications',
  events: 'Send messages to other extensions',
  'shared-storage': 'Share data with other extensions',
  titlebar: 'Show widgets in the app title bar (only while running in the background)',
}

function permLabel(p: string): string {
  return PERMISSION_LABELS[p] ?? p
}

/** 清单里的权限声明（老清单没有该字段 → 空数组，不误报） */
const permissionList = computed<string[]>(() => {
  const raw = (m.value as { permissions?: unknown } | null)?.permissions
  return Array.isArray(raw) ? (raw as string[]) : []
})
const highRiskPermissions = computed(() =>
  permissionList.value.filter((p) => HIGH_RISK_PERMISSIONS.has(p)),
)
const normalPermissions = computed(() =>
  permissionList.value.filter((p) => !HIGH_RISK_PERMISSIONS.has(p)),
)

// ---- 用户反馈（平台托管、公开可见） ----
// 详情弹窗只展示最新 10 条；总数由服务端给（不受截断影响），避免「看起来只有 10 条反馈」
const FEEDBACK_PREVIEW_LIMIT = 10
const feedbackItems = ref<ExtensionFeedback[]>([])
const feedbackTotal = ref(0)
const feedbackLoading = ref(false)
const feedbackError = ref('')
const feedbackDialogOpen = ref(false)
/** 反馈列表的相对时间需要会走的时钟：弹窗开着期间每 60s 推进一次（只影响展示） */
const nowTick = ref(Date.now())
let feedbackClock: number | undefined

watch(
  visible,
  (v) => {
    if (feedbackClock !== undefined) {
      window.clearInterval(feedbackClock)
      feedbackClock = undefined
    }
    if (!v) return
    nowTick.value = Date.now()
    feedbackClock = window.setInterval(() => {
      nowTick.value = Date.now()
    }, 60_000)
  },
  { immediate: true },
)

async function loadFeedback() {
  const id = m.value?.id
  if (!id || !isTauri()) {
    feedbackItems.value = []
    feedbackTotal.value = 0
    return
  }
  feedbackLoading.value = true
  feedbackError.value = ''
  try {
    const page = await tauriApi.listExtensionFeedback(id)
    feedbackItems.value = page.items.slice(0, FEEDBACK_PREVIEW_LIMIT)
    feedbackTotal.value = page.total
  } catch (e) {
    // 公开列表失败不该挡住安装：分区给一行轻提示 + 重试，主流程照旧
    feedbackError.value = 'Could not load feedback.'
    feedbackItems.value = []
    feedbackTotal.value = 0
    console.warn('[feedback] 公开列表加载失败', e)
  } finally {
    feedbackLoading.value = false
  }
}

// 换扩展 / 重新打开（extension 从 null 变回对象）都要重拉
watch(
  () => (props.extension ? `${props.extension.id}@${props.extension.version}` : ''),
  (key) => {
    if (key) void loadFeedback()
  },
  { immediate: true },
)

function kindLabel(k: string): string {
  return feedbackKindLabel(k)
}

/** 相对时间：口径与开发者收件箱共用 utils/feedback（两处显示必须一致） */
function fmtFeedbackTime(ts: number): string {
  return feedbackTimeText(ts, nowTick.value)
}

function onKeydown(e: KeyboardEvent) {
  if (!visible.value) return
  // 反馈弹窗叠在上面时由它自己消费 Esc（及方向键），别把详情弹窗一起关掉
  if (feedbackDialogOpen.value) return
  if (shotPreview.value) {
    if (e.key === 'Escape') shotPreview.value = false
    if (e.key === 'ArrowLeft') showShot(shotIndex.value - 1)
    if (e.key === 'ArrowRight') showShot(shotIndex.value + 1)
    return
  }
  if (e.key === 'Escape') emit('close')
}
onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  if (feedbackClock !== undefined) window.clearInterval(feedbackClock)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="mask">
      <div v-if="visible" class="modal-mask">
        <div ref="cardRef" class="modal-card md-card" role="dialog" aria-label="Extension details" aria-modal="true">
          <div class="md-head">
            <div class="md-title">
              <span class="md-icon" :style="m!.icon && !iconFailed ? {} : { background: accentOf(m!.name).soft }">
                <img
                  v-if="m!.icon && !iconFailed"
                  :src="m!.icon"
                  :alt="m!.name"
                  draggable="false"
                  @error="iconFailed = true"
                />
                <span v-else :style="{ color: accentOf(m!.name).text }">{{ initial() }}</span>
              </span>
              <div class="md-title-text">
                <h2 class="dialog-title">
                  <!-- 名字包一层可收缩 span：flex 容器上 text-overflow 不生效，省略号必须落在文本项自己身上 -->
                  <span class="dialog-title-text">{{ m!.name }}</span>
                  <!-- 官方标识：与市场卡片同一判定口径（清单 official 标记 / com.x-hub.* 保留命名空间） -->
                  <span v-if="isOfficialExt(m!)" class="md-official" title="Official extension (maintained by the x-hub team)">
                    <ShieldCheck :size="12" :stroke-width="2.2" aria-hidden="true" />
                    Official
                  </span>
                </h2>
                <span class="md-version">v{{ m!.version }}</span>
              </div>
            </div>
            <button class="icon-btn" title="Close" aria-label="Close" @click="emit('close')">
              <X :size="14" :stroke-width="2" />
            </button>
          </div>

          <div class="md-body">
            <p class="md-desc">{{ m!.description || 'No description available.' }}</p>

            <!-- 图片展示区（作者发布时上传的截图）：主图 + 缩略图切换，点击主图放大 -->
            <div class="md-section md-shots">
              <div class="md-section-title">
                Screenshots
                <span v-if="shots.length > 1" class="md-shots-count">{{ shotIndex + 1 }} / {{ shots.length }}</span>
              </div>

              <div v-if="shots.length" class="md-showcase">
                <button
                  class="md-stage"
                  type="button"
                  :aria-label="`View screenshot ${shotIndex + 1} at full size`"
                  @click="shotPreview = true"
                >
                  <img
                    :src="currentShot"
                    :alt="`${m!.name} screenshot ${shotIndex + 1}`"
                    @error="onShotError(currentShot)"
                  />
                </button>
                <button
                  v-if="shots.length > 1"
                  class="md-nav md-nav-prev"
                  type="button"
                  aria-label="Previous screenshot"
                  @click="showShot(shotIndex - 1)"
                >
                  <ChevronLeft :size="16" :stroke-width="2.2" aria-hidden="true" />
                </button>
                <button
                  v-if="shots.length > 1"
                  class="md-nav md-nav-next"
                  type="button"
                  aria-label="Next screenshot"
                  @click="showShot(shotIndex + 1)"
                >
                  <ChevronRight :size="16" :stroke-width="2.2" aria-hidden="true" />
                </button>
              </div>

              <div v-if="shots.length > 1" class="md-shot-strip">
                <button
                  v-for="(s, i) in shots"
                  :key="s"
                  class="md-shot"
                  :class="{ active: i === shotIndex }"
                  type="button"
                  :aria-label="`View screenshot ${i + 1}`"
                  @click="showShot(i)"
                >
                  <img :src="s" :alt="`Screenshot ${i + 1}`" loading="lazy" @error="onShotError(s)" />
                </button>
              </div>

              <!-- ⚠️ 空态必须写「=== 0」，不能挂 v-else 到缩略图条上：缩略图条的条件是 `> 1`（单图不显示），
                   v-else 会在「只有 1 张截图」时与主图同时渲染 → 明明有图却提示「作者还没有上传截图」 -->
              <div v-if="!shots.length" class="md-shot-empty">
                <ImageOff :size="16" :stroke-width="1.8" aria-hidden="true" />
                <span>The author hasn’t added screenshots yet. Install the extension and open it from My Extensions to see how it works.</span>
              </div>
            </div>

            <div class="md-meta">
              <div class="md-meta-item">
                <span class="md-meta-label">Type</span>
                <span class="md-meta-value">{{ m!.runtime === 'service' ? 'Service' : 'Web' }}</span>
              </div>
              <div class="md-meta-item">
                <span class="md-meta-label">Category</span>
                <span class="md-meta-value">{{ displayCategory(m!.category) }}</span>
              </div>
              <div class="md-meta-item">
                <span class="md-meta-label">Size</span>
                <span class="md-meta-value">{{ formatSize(m!.size) }}</span>
              </div>
              <div class="md-meta-item">
                <span class="md-meta-label">Installs</span>
                <span class="md-meta-value">{{ installCountText(m!.installCount) }}</span>
              </div>
              <div class="md-meta-item">
                <span class="md-meta-label">Author</span>
                <span class="md-meta-value">{{ m!.author || '—' }}</span>
              </div>
              <div v-if="m!.minAppVersion" class="md-meta-item">
                <span class="md-meta-label">Requires host</span>
                <span class="md-meta-value">v{{ m!.minAppVersion }}+</span>
              </div>
            </div>

            <!-- 服务型扩展：它就是跑在你电脑上的本机程序，必须说清楚 -->
            <div v-if="m!.runtime === 'service'" class="md-risk">
              <ShieldAlert :size="15" :stroke-width="2" aria-hidden="true" />
              <div class="md-risk-text">
                <strong>This is a service extension</strong>
                <p>
                  It runs a program on your computer that can read and write files and connect to the internet. The platform has manually reviewed its code, but make sure you trust the author before installing.
                </p>
              </div>
            </div>

            <div v-if="permissionList.length" class="md-section">
              <div class="md-section-title">
                <Shield :size="13" :stroke-width="2" aria-hidden="true" />Requested permissions
              </div>
              <ul class="md-perms">
                <li v-for="p in highRiskPermissions" :key="p" class="md-perm md-perm-high">
                  <span class="md-perm-name">{{ permLabel(p) }}</span>
                  <span class="md-perm-tag">High risk</span>
                </li>
                <li v-for="p in normalPermissions" :key="p" class="md-perm">
                  <span class="md-perm-name">{{ permLabel(p) }}</span>
                </li>
              </ul>
            </div>

            <button
              v-if="m!.homepage"
              class="md-homepage"
              type="button"
              :disabled="!isTauri()"
              @click="openHomepage"
            >
              <span class="md-homepage-text">Website and documentation</span>
              <span class="md-homepage-url" :title="m!.homepage">{{ m!.homepage }}</span>
              <ExternalLink :size="13" :stroke-width="2" aria-hidden="true" />
            </button>

            <template v-if="m!.changelog">
              <div class="md-sep" />
              <div class="md-section">
                <div class="md-section-title">
                  <Package :size="13" :stroke-width="2" aria-hidden="true" />Changelog
                </div>
                <p class="md-section-text">{{ m!.changelog }}</p>
              </div>
            </template>

            <!-- 用户反馈（公开板）：安装前就能看到已知问题与开发者回复，减少重复反馈。
                 公开列表只含类型 / 用户名 / 正文 / 回复；联系方式与诊断在公开接口里根本不存在 -->
            <div class="md-sep" />
            <div class="md-section md-feedback">
              <div class="md-section-title">
                <MessageSquare :size="13" :stroke-width="2" aria-hidden="true" />
                User feedback
                <span v-if="feedbackTotal > 0" class="md-fb-count">{{ feedbackTotal }}</span>
                <button class="md-fb-write" type="button" @click="feedbackDialogOpen = true">
                  <PenLine :size="12" :stroke-width="2" aria-hidden="true" />
                  Leave feedback
                </button>
              </div>

              <p v-if="feedbackLoading" class="md-fb-state">Loading feedback…</p>
              <p v-else-if="feedbackError" class="md-fb-state">
                {{ feedbackError }}
                <button class="md-fb-retry" type="button" @click="loadFeedback">Retry</button>
              </p>
              <p v-else-if="feedbackItems.length === 0" class="md-fb-state">
                No feedback yet. Have an issue? Let the developer know.
              </p>
              <div v-else class="md-fb-list">
                <article v-for="f in feedbackItems" :key="f.id" class="md-fb-item">
                  <div class="md-fb-top">
                    <span class="md-fb-type" :class="f.kind">{{ kindLabel(f.kind) }}</span>
                    <span class="md-fb-author" :class="{ anon: !f.author_name }">
                      {{ f.author_name || '匿名用户' }}
                    </span>
                    <span class="md-fb-time">{{ fmtFeedbackTime(f.created_at) }}</span>
                    <span v-if="f.status !== 'open'" class="md-fb-status" :class="f.status">
                      {{ f.status === 'resolved' ? 'Resolved' : 'Replied' }}
                    </span>
                  </div>
                  <p class="md-fb-body">{{ f.body }}</p>
                  <div v-if="f.reply" class="md-fb-reply">
                    <span class="md-fb-reply-label">
                      <Reply :size="11" :stroke-width="2" aria-hidden="true" />Developer reply
                    </span>
                    <p>{{ f.reply }}</p>
                  </div>
                </article>
                <p v-if="feedbackTotal > feedbackItems.length" class="md-fb-more">
                  Showing the {{ feedbackItems.length }} most recent
                </p>
              </div>
            </div>

            <div v-if="m!.sha256" class="md-hash">
              <span class="md-hash-head">
                <Shield :size="13" :stroke-width="2" aria-hidden="true" />SHA-256 integrity check
              </span>
              <code class="md-hash-code" :title="m!.sha256">{{ m!.sha256.slice(0, 32) }}…</code>
            </div>
          </div>

          <div class="md-foot">
            <button class="ghost-btn" type="button" @click="emit('close')">Close</button>
            <button
              class="pill-btn"
              type="button"
              :disabled="actionDisabled"
              :title="actionTitle || ''"
              @click="emit('action')"
            >
              {{ actionLabel }}
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 截图放大灯箱：必须挂在 Teleport 根、与遮罩平级，不能放进 .md-card——
         .modal-card 带 backdrop-filter（玻璃弹窗），会把自己变成 fixed 后代的包含块，
         灯箱放卡片里 inset:0 只能铺到弹窗大小（实测就是「点开还在弹窗内，看不清」的根因）；
         挂出来 fixed 才相对视口定位，铺满整个软件窗口。点任意处关闭；多图时 ←/→ 翻页 -->
    <div v-if="visible && shotPreview" class="md-lightbox" @click="shotPreview = false">
      <img :src="currentShot" :alt="`${m!.name} 截图 ${shotIndex + 1}`" />
      <template v-if="shots.length > 1">
        <button
          class="md-lb-nav md-lb-prev"
          type="button"
          aria-label="Previous screenshot"
          @click.stop="showShot(shotIndex - 1)"
        >
          <ChevronLeft :size="20" :stroke-width="2.2" aria-hidden="true" />
        </button>
        <button
          class="md-lb-nav md-lb-next"
          type="button"
          aria-label="Next screenshot"
          @click.stop="showShot(shotIndex + 1)"
        >
          <ChevronRight :size="20" :stroke-width="2.2" aria-hidden="true" />
        </button>
        <span class="md-lb-count">{{ shotIndex + 1 }} / {{ shots.length }}</span>
      </template>
    </div>

    <!-- 提交反馈弹窗：与详情弹窗叠加（Teleport 到 body，层级自然更高；
         焦点陷阱用栈顶让位，见 composables/useFocusTrap） -->
    <ExtensionFeedbackDialog
      v-model:visible="feedbackDialogOpen"
      :ext-id="m?.id ?? ''"
      :ext-name="m?.name ?? null"
      :ext-version="m?.version ?? null"
      :runtime="m?.runtime ?? null"
      @submitted="loadFeedback"
    />
  </Teleport>
</template>

<style scoped>
.md-card {
  width: min(640px, 92vw);
  max-height: min(800px, 88vh);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  /* 弹窗外壳自带 24px padding；本卡片由头/体/脚各自排内边距，清零后才与
     扩展设置弹窗（已安装 / 我的扩展里那个）的 18px 完全对齐 */
  padding: 0;
}
.md-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 18px 18px 0;
  flex-shrink: 0;
}
.md-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.md-icon {
  width: 42px;
  height: 42px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  font-size: 1.125rem;
  font-weight: 700;
  overflow: hidden;
}
.md-icon img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.md-title-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.md-title-text .dialog-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 1.0625rem;
  font-weight: 700;
  color: var(--text-1);
  overflow: hidden;
}
.dialog-title-text {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* 官方标识：与市场卡片同款填充药丸，标题旁一眼认出 */
.md-official {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  background: var(--brand-500);
  color: #fff;
  font-size: 0.68rem;
  font-weight: 700;
  line-height: 1.4;
}
.md-version {
  font-size: 0.75rem;
  color: var(--text-3);
}
.md-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 13px;
  padding: 14px 18px 16px;
}
/* 截图展示区：主图（16:10 舞台）+ 缩略图切换；点击进放大灯箱（Teleport 根级，铺满整窗） */
.md-shots {
  gap: 8px;
}
.md-shots-count {
  margin-left: auto;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-4, var(--text-3));
  font-variant-numeric: tabular-nums;
}
.md-showcase {
  position: relative;
  border-radius: var(--radius-md);
  overflow: hidden;
}
.md-stage {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 10;
  padding: 0;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--bg-card-soft);
  cursor: zoom-in;
}
.md-stage img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.md-stage:hover {
  border-color: var(--brand-500);
}
/* 左右翻页：叠在主图两侧，默认半透明、hover 主图才明显 */
.md-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--scrim);
  color: #fff;
  opacity: 0.55;
  cursor: pointer;
  transition: opacity 150ms ease-out;
}
.md-nav:hover {
  opacity: 1;
}
.md-nav-prev {
  left: 8px;
}
.md-nav-next {
  right: 8px;
}
.md-shot-strip {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  padding: 2px;
}
.md-shot {
  flex: 0 0 auto;
  width: 108px;
  height: 64px;
  padding: 0;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--bg-card-soft);
  cursor: pointer;
  transition: border-color 150ms ease-out;
}
.md-shot img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.md-shot:hover {
  border-color: var(--brand-500);
}
.md-shot.active {
  border-color: var(--brand-500);
  box-shadow: 0 0 0 1px var(--brand-500);
}
.md-shot-empty {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 12px;
  border: 1px dashed var(--border-soft);
  border-radius: var(--radius-md);
  color: var(--text-3);
  font-size: 0.75rem;
  line-height: 1.5;
}
.md-lightbox {
  position: fixed;
  inset: 0;
  z-index: 220;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--scrim);
  cursor: zoom-out;
  animation: mask-in 0.18s ease-out;
}
.md-lightbox img {
  max-width: 100%;
  max-height: 100%;
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-card);
}
.md-lb-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--scrim);
  color: #fff;
  opacity: 0.7;
  cursor: pointer;
  transition: opacity 150ms ease-out;
}
.md-lb-nav:hover {
  opacity: 1;
}
.md-lb-prev {
  left: 20px;
}
.md-lb-next {
  right: 20px;
}
.md-lb-count {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  background: var(--scrim);
  color: #fff;
  font-size: 0.75rem;
  font-variant-numeric: tabular-nums;
}
.md-desc {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--text-2);
  line-height: 1.7;
  white-space: pre-wrap;
}
.md-meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
  gap: 8px;
}
.md-meta-item {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 8px 10px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-sm);
  background: var(--bg-card-soft);
}
.md-meta-label {
  font-size: 0.6875rem;
  color: var(--text-4, var(--text-3));
}
.md-meta-value {
  font-size: 0.8125rem;
  font-weight: 650;
  color: var(--text-1);
  word-break: break-all;
}
.md-homepage {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--brand-500);
  font-size: 0.75rem;
  text-align: left;
  cursor: pointer;
  transition: background 150ms ease-out, border-color 150ms ease-out;
}
.md-homepage-text {
  font-weight: 600;
  white-space: nowrap;
}
.md-homepage-url {
  color: var(--text-3);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.md-homepage:hover {
  background: var(--brand-50);
  border-color: var(--brand-500);
}
.md-homepage:disabled {
  opacity: 0.6;
  cursor: default;
}
/* 服务型扩展风险提示 + 权限分级清单（安装前显式告知） */
.md-risk {
  display: flex;
  gap: 8px;
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  background: var(--c-orange-soft);
  color: var(--c-orange-ink);
  font-size: 0.75rem;
  line-height: 1.5;
}
.md-risk-text strong {
  display: block;
  margin-bottom: 2px;
}
.md-risk-text p {
  margin: 0;
}
.md-perms {
  margin: 6px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.md-perm {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--text-2);
}
.md-perm-high {
  color: var(--c-red-ink);
}
.md-perm-tag {
  padding: 0 6px;
  border-radius: var(--radius-pill);
  background: var(--c-red-soft);
  color: var(--c-red-ink);
  font-size: 0.6875rem;
  font-weight: 600;
}

.md-sep {
  height: 1px;
  background: var(--border-soft);
  margin: 2px 0;
}
.md-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.md-section-title {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.75rem;
  font-weight: 650;
  color: var(--text-2);
}
.md-section-text {
  margin: 0;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-card-soft);
  font-size: 0.75rem;
  color: var(--text-3);
  line-height: 1.7;
  white-space: pre-wrap;
}
.md-hash {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-card-soft);
}
.md-hash-head {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 0.6875rem;
  color: var(--text-4, var(--text-3));
}
.md-hash-code {
  font-size: 0.6875rem;
  font-family: var(--font-mono, ui-monospace, monospace);
  color: var(--text-3);
  word-break: break-all;
}
.md-foot {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 18px 18px;
  border-top: 1px solid var(--border-soft);
  flex-shrink: 0;
}

/* ---- 用户反馈（公开板） ---- */
.md-fb-count {
  font-size: 0.625rem;
  font-weight: 650;
  padding: 1px 6px;
  border-radius: var(--radius-pill);
  background: var(--bg-card-soft);
  color: var(--text-3);
}
.md-fb-write {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border: 0;
  border-radius: var(--radius-sm);
  background: var(--brand-50);
  color: var(--brand-500);
  font-size: 0.6875rem;
  font-weight: 650;
  cursor: pointer;
  transition: background 150ms ease-out, color 150ms ease-out;
}
.md-fb-write:hover {
  background: var(--brand-500);
  color: #fff;
}
.md-fb-state {
  margin: 0;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-card-soft);
  font-size: 0.75rem;
  color: var(--text-3);
}
.md-fb-retry {
  margin-left: 6px;
  padding: 0;
  border: 0;
  background: none;
  color: var(--brand-500);
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
}
.md-fb-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  /* 内层定高滚动：详情弹窗本身可滚，反馈多了不给它无限拉长（超出由外层继续滚） */
  max-height: 320px;
  overflow-y: auto;
  padding-right: 3px;
}
.md-fb-item {
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  padding: 8px 11px;
  background: var(--bg-card-soft);
}
.md-fb-top {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
  margin-bottom: 4px;
}
.md-fb-type {
  font-size: 0.625rem;
  font-weight: 650;
  padding: 1px 6px;
  border-radius: 5px;
}
.md-fb-type.bug {
  background: color-mix(in srgb, var(--c-red) 12%, transparent);
  color: var(--c-red);
}
.md-fb-type.idea {
  background: color-mix(in srgb, var(--c-orange) 15%, transparent);
  color: var(--c-orange);
}
.md-fb-type.question {
  background: var(--brand-50);
  color: var(--brand-500);
}
.md-fb-author {
  font-size: 0.6875rem;
  font-weight: 650;
  color: var(--text-2);
}
.md-fb-author.anon {
  color: var(--text-3);
  font-weight: 500;
}
.md-fb-time {
  font-size: 0.625rem;
  color: var(--text-3);
}
.md-fb-status {
  margin-left: auto;
  font-size: 0.625rem;
  font-weight: 650;
  padding: 1px 6px;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--c-green) 14%, transparent);
  color: var(--c-green);
}
.md-fb-body {
  margin: 0;
  font-size: 0.75rem;
  line-height: 1.6;
  color: var(--text-2);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.md-fb-reply {
  margin-top: 7px;
  padding: 6px 9px;
  border-left: 2px solid var(--brand-500);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  background: var(--brand-50);
}
.md-fb-reply-label {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.625rem;
  font-weight: 700;
  color: var(--brand-500);
  margin-bottom: 2px;
}
.md-fb-reply p {
  margin: 0;
  font-size: 0.6875rem;
  line-height: 1.55;
  color: var(--text-2);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.md-fb-more {
  margin: 0;
  font-size: 0.625rem;
  color: var(--text-3);
  text-align: center;
}
</style>
