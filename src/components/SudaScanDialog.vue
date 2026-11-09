<script lang="ts">
/** 弹窗的三种扫描来源：已安装应用 / 桌面 / 浏览器书签 */
export type ScanMode = 'apps' | 'desktop' | 'bookmarks'

/** 统一的扫描结果项（folder 仅在桌面模式出现，导入时归入 file 大类） */
export interface ScanItem {
  name: string
  target: string
  icon: string | null
  kind: 'app' | 'web' | 'file' | 'folder'
  /** 书签来源文件夹（按 folder 分组展示） */
  folder?: string
  /** 书签来源浏览器（Chrome/Edge/Brave/Chromium）；书签树顶层按浏览器分组勾选 */
  browser?: string
  /** 导入时归入的速达小类名（书签按文件夹归类时填；null = 默认归类/未归类） */
  category?: string | null
  /** 桌面快捷方式原始路径（仅 .lnk/.url 有），供「导入后清理桌面快捷方式」用 */
  source?: string | null
}
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, toRef, watch } from 'vue'
import { AlertTriangle, Check, ChevronRight, Globe, Loader2, Minus, Search } from 'lucide-vue-next'
import { isTauri, tauriApi } from '../api/tauri'
import type { BrowserProfileStat } from '../api/tauri'
import { useStore } from '../stores/workbench'
import { useFocusTrap } from '../composables/useFocusTrap'
import { accentOf, iconSrc } from '../composables/useResourceIcon'

const props = withDefaults(defineProps<{ visible: boolean; mode?: ScanMode }>(), {
  mode: 'apps',
})

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'imported', items: ScanItem[], cleanShortcuts: boolean): void
}>()

const store = useStore()
const cardRef = ref<HTMLElement | null>(null)
const searchRef = ref<HTMLInputElement | null>(null)

useFocusTrap(toRef(props, 'visible'), cardRef, searchRef)

const loading = ref(false)
const error = ref('')
const items = ref<ScanItem[]>([])
/** 书签模式：数量口径（各配置文件原始条数 / 去重合并 / 无效跳过 / 超限截断），解释「x-hub 计数 ≠ 浏览器收藏夹计数」 */
const bookmarkMeta = ref<{
  profiles: BrowserProfileStat[]
  duplicates: number
  skipped: number
  truncated: number
} | null>(null)
const checked = ref<Set<string>>(new Set())
const keyword = ref('')
const brokenIcons = ref<Set<string>>(new Set())
/** 桌面模式：导入后是否顺手清理桌面上的 .lnk/.url（只清快捷方式，绝不动文件/文件夹/exe） */
const cleanShortcuts = ref(false)
/** 书签模式：是否按浏览器文件夹设置速达小类（默认开，浏览器里已分好的目录直接沿用） */
const groupByFolder = ref(true)
/** 书签模式：按网址去重（默认开；**浏览器内**去重——同一网址在同一浏览器的多配置文件/文件夹间只算一条，
 *  跨浏览器各自展示、不互相合并，导入侧按网址只建一条——dckxx 2026-10-09 二次改口径：
 *  最早的「跨浏览器只留第一条」会把与首个浏览器大量重复的浏览器整个吞掉，分组后直接消失） */
const dedupeBookmarks = ref(true)
/** 文件夹 → 小类的手动覆盖（key=文件夹完整路径，原始输入；空 = 归默认小类）。
 *  未覆盖的走 folderToCategory 自动映射；同名覆盖可把多个文件夹合并进同一个小类 */
const categoryOverrides = ref<Map<string, string>>(new Map())
/** 书签树浏览：各文件夹的展开状态（存完整路径；默认只展开第一层，大书签库先看结构再逐级挑） */
const expandedFolders = ref<Set<string>>(new Set())

const keyOf = (a: { target: string }) => a.target.toLowerCase()

/** 后端小类名的长度上限（commands::validate_subcategory_input）：全路径 60、每段 20 */
const SUBCATEGORY_PATH_MAX = 60
const SUBCATEGORY_NAME_MAX = 20

/** 书签文件夹路径切分（trim + 去空段），folderToCategory / 文件夹树 / 默认展开三处共用 */
function folderSegments(folder: string | undefined | null): string[] {
  if (!folder) return []
  return folder.split('/').map((s) => s.trim()).filter(Boolean)
}

/** 书签文件夹 → 速达小类全路径：剥掉浏览器根名（书签栏/其他书签/移动端），余下路径用 / 连接——
 *  浏览器里已分好的目录直接沿用（「书签栏/前端」→「前端」、「书签栏/开发/前端」→「开发/前端」；
 *  「/」层级在速达小类行里逐级嵌套展示，见 utils/subcategoryTree.ts）；
 *  根下直挂的书签返回 null（走默认小类/未归类）；全路径超 60 字符时退化为末级文件夹名，仍超才硬截断 */
function folderToCategory(folder: string | undefined | null): string | null {
  const segs = folderSegments(folder)
  if (segs.length <= 1) return null
  const full = segs.slice(1).join('/')
  // 全路径 ≤60 且每段 ≤20 才用完整路径；任一段超 20（后端按段校验）退化到末级，
  // 仍超才硬截断——否则小类创建被拒，资源挂着不存在的小类落库、任何 chip 都筛不出来
  if (full.length <= SUBCATEGORY_PATH_MAX && segs.slice(1).every((s) => s.length <= SUBCATEGORY_NAME_MAX)) return full
  const leaf = segs[segs.length - 1]
  if (leaf.length <= SUBCATEGORY_NAME_MAX) return leaf
  return leaf.slice(0, SUBCATEGORY_NAME_MAX)
}

/** 文件夹最终归入的小类：手动覆盖优先（trim；空 = 明确归默认小类），否则自动映射。
 *  覆盖输入按分段 trim + 拼回全路径——后端校验「分段首尾禁空格 + 每段 1–20 + 全路径 ≤60」，
 *  「开发 / 前端」这类顺手的输入不归一会让小类创建被拒、资源挂着不存在的小类落库，
 *  在任何小类 chip 下都筛不出来（Suda.vue onScanImported 的创建失败兜底只报错不拦截） */
function effectiveCategory(folder: string | undefined | null): string | null {
  if (!folder) return null
  const ov = categoryOverrides.value.get(folder)
  if (ov === undefined) return folderToCategory(folder)
  const t = ov
    .split('/')
    .map((s) => s.trim().slice(0, SUBCATEGORY_NAME_MAX))
    .filter(Boolean)
    .join('/')
  return t === '' ? null : t.slice(0, SUBCATEGORY_PATH_MAX)
}

/** 编辑文件夹的小类映射（存原始输入不即时 trim，避免输入中途空格被绑定值吃掉） */
function onCatInput(path: string, e: Event) {
  const v = (e.target as HTMLInputElement).value
  categoryOverrides.value = new Map(categoryOverrides.value).set(path, v)
}

const CONF = {
  apps: {
    title: 'Scan installed apps',
    sub: 'Select the apps to add to Quick Launch. Unselected apps will be ignored.',
    placeholder: 'Search apps…',
    loading: 'Scanning installed apps…',
    loadingHint: 'Extracting app icons for the first scan. This may take a moment.',
    empty: 'No apps available to import.',
    confirm: 'Add selected',
    aria: 'Scan installed apps',
  },
  desktop: {
    title: 'Scan desktop',
    sub: 'Scans only the top level of your desktop; subfolders are not included.',
    placeholder: 'Search desktop items…',
    loading: 'Scanning desktop…',
    loadingHint: 'Resolving shortcuts and extracting icons. This may take a moment.',
    empty: 'No desktop items available to import.',
    confirm: 'Add to Quick Launch',
    aria: 'Scan desktop',
  },
  bookmarks: {
    title: 'Import browser bookmarks',
    sub: 'Detected browsers are selected by default. Click a browser to select or clear all its bookmarks. Expand folders to choose items, or search.',
    placeholder: 'Search bookmarks…',
    loading: 'Reading browser bookmarks…',
    loadingHint: 'Scanning browser profile folders.',
    empty: 'No browser bookmarks found.',
    confirm: 'Import selected',
    aria: 'Import browser bookmarks',
  },
} as const

const conf = computed(() => CONF[props.mode])

// 已在速达中的目标（按目标路径判重，跨大类）→ 列表中禁用勾选
const existingTargets = computed(() => {
  const s = new Set<string>()
  for (const r of store.state.resources) {
    if (r.target) s.add(r.target.toLowerCase())
  }
  return s
})

const filtered = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return items.value
  return items.value.filter((a) => a.name.toLowerCase().includes(kw))
})

const KIND_ORDER: ScanItem['kind'][] = ['app', 'web', 'file', 'folder']
const KIND_LABEL: Record<ScanItem['kind'], string> = {
  app: 'App',
  web: 'Website',
  file: 'File',
  folder: 'Folder',
}

// 分组展示：应用模式单组；桌面模式按类别分组；书签模式按来源文件夹分组
const groups = computed(() => {
  const list = filtered.value
  if (props.mode === 'desktop') {
    return KIND_ORDER.map((k) => ({
      key: `kind-${k}`,
      label: KIND_LABEL[k],
      items: list.filter((a) => a.kind === k),
    })).filter((g) => g.items.length > 0)
  }
  if (props.mode === 'bookmarks') {
    const map = new Map<string, ScanItem[]>()
    for (const a of list) {
      const f = a.folder ?? 'Uncategorized'
      const arr = map.get(f)
      if (arr) arr.push(a)
      else map.set(f, [a])
    }
    return [...map.entries()].map(([folder, items]) => ({
      key: `folder-${folder}`,
      label: folder,
      items,
    }))
  }
  return [{ key: 'all', label: '', items: list }]
})

// ---- 书签文件夹树（大书签库逐条滚不现实：顶层 = 浏览器（勾选整组导入），其下按真实目录层级折叠浏览）----
interface BookmarkNode {
  /** 完整路径（顶层 = 浏览器名；其下带浏览器前缀，如「Edge/书签栏/前端」，保证跨浏览器同名目录各自独立） */
  path: string
  name: string
  depth: number
  children: BookmarkNode[]
  /** 直挂书签 */
  items: ScanItem[]
  /** 子树书签总数（含直挂） */
  total: number
  /** 不带浏览器前缀的原始目录路径（供小类映射 folderToCategory 剥根名的口径不变；顶层浏览器节点为空） */
  rawFolder: string
}

/** 浏览器展示排序（检测不到的排在后面），顶层浏览器节点按此排序 */
const BROWSER_ORDER = ['Chrome', 'Edge', 'Brave', 'Chromium']

/** 条目在树中的路径分段：顶层 = 浏览器，其后为原始目录分段（空目录归「未分类」） */
function treeSegments(it: ScanItem): string[] {
  const rest = folderSegments(it.folder)
  if (rest.length === 0) rest.push('Uncategorized')
  return [it.browser || 'Other', ...rest]
}

function buildTree(list: ScanItem[]): BookmarkNode[] {
  const roots: BookmarkNode[] = []
  const byPath = new Map<string, BookmarkNode>()
  for (const it of list) {
    const segs = treeSegments(it)
    let path = ''
    let parent: BookmarkNode | null = null
    for (let d = 0; d < segs.length; d++) {
      path = d === 0 ? segs[0] : `${path}/${segs[d]}`
      let node = byPath.get(path)
      if (!node) {
        node = {
          path,
          name: segs[d],
          depth: d,
          children: [],
          items: [],
          total: 0,
          rawFolder: segs.slice(1, d + 1).join('/'),
        }
        byPath.set(path, node)
        if (parent) parent.children.push(node)
        else roots.push(node)
      }
      parent = node
    }
    parent!.items.push(it)
  }
  const calcTotal = (n: BookmarkNode): number => {
    n.total = n.items.length
    for (const c of n.children) n.total += calcTotal(c)
    return n.total
  }
  roots.forEach(calcTotal)
  const rank = (browser: string) => {
    const i = BROWSER_ORDER.indexOf(browser)
    return i === -1 ? BROWSER_ORDER.length : i
  }
  roots.sort((a, b) => rank(a.name) - rank(b.name))
  return roots
}

const bookmarkTree = computed<BookmarkNode[]>(() => buildTree(items.value))

/** 文件夹子树内的全部书签（直挂 + 各级子目录） */
function subtreeItems(n: BookmarkNode): ScanItem[] {
  const out = [...n.items]
  const walk = (nodes: BookmarkNode[]) => {
    for (const c of nodes) {
      out.push(...c.items)
      walk(c.children)
    }
  }
  walk(n.children)
  return out
}

/** 每个文件夹节点的勾选统计：一次遍历算全树并按 path 存表——模板里三态框每行要查 3 次
 *  （类名/title/图标），逐行现算子树是 O(n²)，大书签库每勾一次卡一帧 */
const folderStats = computed(() => {
  const stats = new Map<string, { sel: number; selChecked: number }>()
  const walk = (n: BookmarkNode): { sel: number; selChecked: number } => {
    let sel = 0
    let selChecked = 0
    for (const it of n.items) {
      if (existingTargets.value.has(keyOf(it))) continue
      sel++
      if (checked.value.has(keyOf(it))) selChecked++
    }
    for (const c of n.children) {
      const s = walk(c)
      sel += s.sel
      selChecked += s.selChecked
    }
    stats.set(n.path, { sel, selChecked })
    return { sel, selChecked }
  }
  bookmarkTree.value.forEach(walk)
  return stats
})

/** 文件夹勾选三态：all 全勾 / part 部分 / none 全不勾（已在速达中的条目不可勾，不参与判定） */
function folderCheckState(n: BookmarkNode): 'all' | 'part' | 'none' {
  const s = folderStats.value.get(n.path)
  if (!s || s.sel === 0 || s.selChecked === 0) return 'none'
  return s.selChecked === s.sel ? 'all' : 'part'
}

/** 文件夹子树里还有无可勾选条目（全都在速达里 → 文件夹行整体置灰） */
function hasSelectable(n: BookmarkNode): boolean {
  return (folderStats.value.get(n.path)?.sel ?? 0) > 0
}

/** 点文件夹勾选框：未全勾 → 勾整棵子树；已全勾 → 清空整棵子树 */
function toggleFolderCheck(n: BookmarkNode) {
  const sel = subtreeItems(n).filter((it) => !existingTargets.value.has(keyOf(it)))
  if (sel.length === 0) return
  const checkAll = folderCheckState(n) !== 'all'
  const next = new Set(checked.value)
  for (const it of sel) {
    if (checkAll) next.add(keyOf(it))
    else next.delete(keyOf(it))
  }
  checked.value = next
}

function toggleExpand(path: string) {
  const next = new Set(expandedFolders.value)
  if (next.has(path)) next.delete(path)
  else next.add(path)
  expandedFolders.value = next
}

function setExpandAll(open: boolean) {
  if (!open) {
    expandedFolders.value = new Set()
    return
  }
  const all = new Set<string>()
  const walk = (nodes: BookmarkNode[]) => {
    for (const n of nodes) {
      if (n.children.length > 0) {
        all.add(n.path)
        walk(n.children)
      }
    }
  }
  walk(bookmarkTree.value)
  expandedFolders.value = all
}

/** 文件夹行的数量标注：有子目录显示子树总数，纯书签文件夹显示直挂数（明细放 title） */
function folderCountText(n: BookmarkNode): string {
  return n.children.length > 0 ? `${n.total} total` : `${n.items.length}`
}

/** 浏览器行的配置文件原始条数（去重前），如「Default 612 · Profile 3 238」；无数据返回 null */
function browserRawText(name: string): string | null {
  const rows = bookmarkMeta.value?.profiles.filter((p) => p.browser === name) ?? []
  if (rows.length === 0) return null
  return rows.map((p) => `${p.profile} ${p.count}`).join(' · ')
}

/** 浏览器行悬浮提示：x-hub 显示条数随「按网址去重」开关变化，配置文件原始数供与浏览器收藏夹管理器对账 */
function browserRowTitle(n: BookmarkNode): string {
  const raw = browserRawText(n.name)
  const base = dedupeBookmarks.value
    ? `${n.name}: ${n.total} bookmarks (deduplicated within this browser)`
    : `${n.name}: ${n.total} bookmarks (not deduplicated)`
  return raw ? `${base}; raw counts by profile: ${raw}` : base
}

/** 数量口径说明文案：开头说明去重开关状态，其后列出合并 / 跳过 / 截断明细 */
const bookmarkNote = computed(() => {
  const m = bookmarkMeta.value
  if (!m) return ''
  const head = dedupeBookmarks.value
    ? 'Browsers are listed separately. A URL found in multiple profiles or folders within one browser is counted once; URLs are not merged across browsers.'
    : 'Browsers are listed separately without URL deduplication, so the same URL may appear more than once.'
  const parts: string[] = []
  if (dedupeBookmarks.value && m.duplicates > 0) parts.push(`Merged ${m.duplicates} duplicate URLs within browsers`)
  if (m.skipped > 0) parts.push(`Skipped ${m.skipped} bookmarks with empty names or URLs, or script URLs`)
  if (m.truncated > 0) parts.push(`Limit exceeded; showing the first ${items.value.length}`)
  return `Count details: ${head}${parts.length ? ` ${parts.join('; ')}` : ''} Importing creates only one entry per URL.`
})

/** 统一的列表行模型：树浏览时 folder/item 交替，扁平分组时 group/item */
type ListRow =
  | { type: 'group'; key: string; label: string; count: number; cat: string | null }
  | { type: 'folder'; key: string; node: BookmarkNode }
  | { type: 'item'; key: string; item: ScanItem; indent: number }

const rows = computed<ListRow[]>(() => {
  // 书签 + 无搜索词：可折叠文件夹树（子文件夹在前、直挂书签在后，与浏览器书签管理器一致）
  if (props.mode === 'bookmarks' && !keyword.value.trim()) {
    const out: ListRow[] = []
    const walk = (nodes: BookmarkNode[]) => {
      for (const n of nodes) {
        out.push({ type: 'folder', key: `folder:${n.path}`, node: n })
        if (expandedFolders.value.has(n.path)) {
          walk(n.children)
          for (const it of n.items) {
            out.push({ type: 'item', key: it.target, item: it, indent: n.depth + 1 })
          }
        }
      }
    }
    walk(bookmarkTree.value)
    return out
  }
  // 其余：扁平分组（应用/桌面模式 + 书签搜索结果，搜索时跨文件夹的命中平铺最好认）
  const catPreview = props.mode === 'bookmarks' && groupByFolder.value
  return groups.value.flatMap((g) => {
    const head: ListRow[] = g.label
      ? [
          {
            type: 'group',
            key: g.key,
            label: g.label,
            count: g.items.length,
            cat: catPreview ? effectiveCategory(g.label) : null,
          },
        ]
      : []
    return [
      ...head,
      ...g.items.map<ListRow>((it) => ({ type: 'item', key: it.target, item: it, indent: 0 })),
    ]
  })
})

// 桌面模式：顶部统计各类别数量
const stats = computed(() => {
  const c: Record<ScanItem['kind'], number> = { app: 0, web: 0, file: 0, folder: 0 }
  for (const a of items.value) c[a.kind]++
  return c
})

const selectedCount = computed(() => {
  // 跨浏览器同网址共享勾选键（keyOf=网址）：按唯一网址计数，
  // 否则按钮显示的条数会大于实际导入条数（confirm 按网址去重传参）
  const keys = new Set(items.value.map(keyOf))
  let n = 0
  for (const k of keys) {
    if (checked.value.has(k) && !existingTargets.value.has(k)) n++
  }
  return n
})

/** 已勾选且可清理的桌面快捷方式数量（决定「清理桌面快捷方式」是否可选） */
const selectedShortcuts = computed(() =>
  items.value.filter(
    (a) => checked.value.has(keyOf(a)) && !existingTargets.value.has(keyOf(a)) && !!a.source,
  ).length,
)

/** 书签模式：勾选项按文件夹归类时，需要新建的速达小类数量（已存在的不计；含手动改名的覆盖） */
const newCategoryCount = computed(() => {
  if (props.mode !== 'bookmarks' || !groupByFolder.value) return 0
  const existing = new Set(store.subcategoriesOf('web').map((s) => s.name))
  const wanted = new Set<string>()
  for (const a of items.value) {
    if (!checked.value.has(keyOf(a)) || existingTargets.value.has(keyOf(a))) continue
    const c = effectiveCategory(a.folder)
    if (c && !existing.has(c)) wanted.add(c)
  }
  return wanted.size
})

const allVisibleChecked = computed(() => {
  const visible = filtered.value.filter((a) => !existingTargets.value.has(keyOf(a)))
  return visible.length > 0 && visible.every((a) => checked.value.has(keyOf(a)))
})

watch(
  () => props.visible,
  (v) => {
    if (!v) return
    void startScan()
  },
)

async function runScan(mode: ScanMode): Promise<ScanItem[]> {
  if (mode === 'desktop') {
    const list = await tauriApi.scanDesktop()
    return list.map((d) => ({ ...d, kind: d.kind }))
  }
  if (mode === 'bookmarks') {
    const scan = await tauriApi.scanBrowserBookmarks(dedupeBookmarks.value)
    bookmarkMeta.value = {
      profiles: scan.profiles,
      duplicates: scan.duplicates,
      skipped: scan.skipped,
      truncated: scan.truncated,
    }
    return scan.items.map((b) => ({
      name: b.name,
      target: b.target,
      icon: null,
      kind: 'web' as const,
      folder: b.folder,
      browser: b.browser,
    }))
  }
  const apps = await tauriApi.scanInstalledApps()
  return apps.map((a) => ({ ...a, kind: 'app' as const }))
}

/** 默认勾选规则：桌面模式下文件/文件夹噪音大，默认不勾；书签默认全部勾选
 *  （顶层浏览器整组亮勾，不要的浏览器/目录再手动取消——dckxx 2026-10-09 拍板的口径）；其余全勾 */
function defaultChecked(list: ScanItem[]): Set<string> {
  const s = new Set<string>()
  for (const it of list) {
    if (props.mode === 'desktop' && (it.kind === 'file' || it.kind === 'folder')) continue
    s.add(keyOf(it))
  }
  return s
}

/** 书签树初始展开：只展开浏览器层（其下根目录行可见但收起），先看「哪些浏览器、各多少条」 */
function defaultExpanded(list: ScanItem[]): Set<string> {
  const s = new Set<string>()
  for (const it of list) s.add(treeSegments(it)[0])
  return s
}

async function startScan() {
  if (!isTauri()) return
  loading.value = true
  error.value = ''
  items.value = []
  bookmarkMeta.value = null
  checked.value = new Set()
  keyword.value = ''
  brokenIcons.value = new Set()
  cleanShortcuts.value = false
  groupByFolder.value = true
  dedupeBookmarks.value = true
  categoryOverrides.value = new Map()
  try {
    const list = await runScan(props.mode)
    items.value = list
    checked.value = defaultChecked(list)
    expandedFolders.value = defaultExpanded(list)
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
    // 列表渲染后聚焦搜索框（无列表时为 no-op）
    requestAnimationFrame(() => searchRef.value?.focus())
  }
}

function isExisting(a: ScanItem) {
  return existingTargets.value.has(keyOf(a))
}

/** 切换「按网址去重」：重扫书签刷新列表与数量口径（保留小类设置；勾选/展开按新列表回默认）。
 *  守卫先于翻开关：扫描进行中/非 Tauri 环境不切口径，并把勾选框视觉态拨回去——
 *  否则开关已翻、列表还是旧口径，两边对不上直到下次重扫 */
async function toggleDedupe(e: Event) {
  if (loading.value || !isTauri()) {
    ;(e.target as HTMLInputElement).checked = dedupeBookmarks.value
    return
  }
  dedupeBookmarks.value = !dedupeBookmarks.value
  loading.value = true
  error.value = ''
  try {
    const list = await runScan(props.mode)
    items.value = list
    checked.value = defaultChecked(list)
    expandedFolders.value = defaultExpanded(list)
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

function showImg(a: ScanItem) {
  return !!a.icon && !brokenIcons.value.has(keyOf(a))
}

function onImgError(a: ScanItem) {
  brokenIcons.value.add(keyOf(a))
}

function toggleItem(a: ScanItem) {
  if (isExisting(a)) return
  const k = keyOf(a)
  const next = new Set(checked.value)
  if (next.has(k)) next.delete(k)
  else next.add(k)
  checked.value = next
}

function toggleAll() {
  const next = new Set(checked.value)
  const visible = filtered.value.filter((a) => !isExisting(a))
  const willSelect = !allVisibleChecked.value
  for (const a of visible) {
    const k = keyOf(a)
    if (willSelect) next.add(k)
    else next.delete(k)
  }
  checked.value = next
}

function confirm() {
  // 跨浏览器同网址共享勾选键：按唯一网址取第一条（排序在前浏览器的文件夹决定小类），
  // 否则同网址会传两条给 Suda.vue 重复建资源
  const selected: ScanItem[] = []
  const seenKeys = new Set<string>()
  for (const a of items.value) {
    const k = keyOf(a)
    if (seenKeys.has(k)) continue
    seenKeys.add(k)
    if (checked.value.has(k) && !existingTargets.value.has(k)) selected.push(a)
  }
  if (selected.length === 0) return
  // 书签按文件夹归类时把小类名随条目带回（Suda.vue 负责补建缺失的小类再落库）；
  // 小类取「手动覆盖 ?? 自动映射」，覆盖可改名/合并/清空（清空 = 归默认小类）
  const withCategory = selected.map((a) => ({
    ...a,
    category:
      props.mode === 'bookmarks' && groupByFolder.value ? effectiveCategory(a.folder) : null,
  }))
  emit('imported', withCategory, props.mode === 'desktop' && cleanShortcuts.value)
  emit('close')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.visible) emit('close')
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Teleport to="body">
    <Transition name="mask">
      <div v-if="visible" class="modal-mask">
        <div
          ref="cardRef"
          class="modal-card scan-card"
          role="dialog"
          :aria-label="conf.aria"
          aria-modal="true"
        >
          <header class="scan-head">
            <h2 class="dialog-title">{{ conf.title }}</h2>
            <p class="scan-sub">{{ conf.sub }}</p>
          </header>

          <!-- 桌面模式：统计 + 醒目提醒（避免误以为「加入=复制」后去删桌面原文件） -->
          <template v-if="mode === 'desktop' && !loading && !error && items.length > 0">
            <p class="scan-stats">
              Desktop: {{ items.length }} items — {{ stats.app }} apps · {{ stats.web }} websites ·
              {{ stats.file }} files · {{ stats.folder }} folders
            </p>
            <p class="scan-warn">
              <AlertTriangle :size="14" :stroke-width="2.2" aria-hidden="true" />
              <span>
                Adding an item to Quick Launch only creates a shortcut; it <b>does not copy or delete</b> the original on your desktop. Manage your desktop files as needed.
              </span>
            </p>
            <label class="scan-clean" :class="{ disabled: selectedShortcuts === 0 }">
              <input
                type="checkbox"
                :checked="cleanShortcuts"
                :disabled="selectedShortcuts === 0"
                @change="cleanShortcuts = !cleanShortcuts"
              />
              <span>
                <b>Remove desktop shortcuts after import</b> (delete these {{ selectedShortcuts }} .lnk / .url files; regular files, folders, and .exe files are <b>left untouched</b>)
              </span>
            </label>
          </template>

          <!-- 书签模式：按浏览器文件夹设置速达小类（浏览器里已分好的目录直接沿用） -->
          <template v-if="mode === 'bookmarks' && !loading && !error && items.length > 0">
            <label class="scan-clean">
              <input
                type="checkbox"
                :checked="groupByFolder"
                @change="groupByFolder = !groupByFolder"
              />
              <span>
                Use <b>browser folders</b> as Quick Launch categories, preserving the folder hierarchy (for example, Bookmarks bar/Development/Frontend → Development/Frontend, shown as nested categories in Quick Launch). Edit each folder's category in the list; leave it blank to use the default, or use the same name to merge. Missing categories are created automatically<template v-if="newCategoryCount > 0">; {{ newCategoryCount }} will be created</template>.
              </span>
            </label>
            <!-- 数量口径：解释「x-hub 计数 ≠ 浏览器收藏夹管理器计数」（所有配置文件合并 + 同网址只算一条） -->
            <label class="scan-clean">
              <input
                type="checkbox"
                :checked="dedupeBookmarks"
                @change="toggleDedupe"
              />
              <span><b>Deduplicate by URL</b>: Count a URL once across profiles and folders within each browser. Browsers are shown separately, and <b>only one entry per URL is imported</b>. Clear this option to show every occurrence.</span>
            </label>
            <p v-if="bookmarkNote" class="scan-note">{{ bookmarkNote }}</p>
          </template>

          <!-- 搜索 + 全选 -->
          <div v-if="!loading && items.length > 0" class="scan-toolbar">
            <div class="scan-search-wrap">
              <Search :size="14" :stroke-width="2" class="scan-search-icon" aria-hidden="true" />
              <input
                ref="searchRef"
                v-model="keyword"
                class="field-input scan-search"
                type="text"
                :placeholder="conf.placeholder"
                @keydown="onKeydown"
              />
            </div>
            <template v-if="mode === 'bookmarks' && !keyword.trim()">
              <button class="ghost-btn scan-select-all" @click="setExpandAll(false)">
                Collapse all
              </button>
              <button class="ghost-btn scan-select-all" @click="setExpandAll(true)">
                Expand all
              </button>
            </template>
            <button class="ghost-btn scan-select-all" @click="toggleAll">
              {{ allVisibleChecked ? 'Clear selection' : 'Select all' }}
            </button>
          </div>

          <!-- 扫描中 -->
          <div v-if="loading" class="scan-state">
            <Loader2 :size="26" :stroke-width="1.5" class="spin" />
            <p>{{ conf.loading }}</p>
            <span>{{ conf.loadingHint }}</span>
          </div>

          <!-- 错误 -->
          <div v-else-if="error" class="scan-state">
            <p class="scan-error">{{ error }}</p>
          </div>

          <!-- 空结果 -->
          <div v-else-if="items.length === 0" class="scan-state">
            <p>{{ conf.empty }}</p>
          </div>

          <!-- 列表：书签无搜索词时为可折叠文件夹树（大库按文件夹整组挑），其余为扁平分组 -->
          <div v-else class="scan-list">
            <template v-for="row in rows" :key="row.key">
              <!-- 扁平分组头（应用/桌面 + 书签搜索结果） -->
              <p v-if="row.type === 'group'" class="scan-group">
                {{ row.label }}（{{ row.count }}）<span v-if="row.cat" class="scan-group-cat"
                  >→ Category: {{ row.cat }}</span
                >
              </p>
              <!-- 文件夹行：点行展开/收起，勾选框三态整组选入；顶层行 = 浏览器（整组勾选/取消） -->
              <div
                v-else-if="row.type === 'folder'"
                class="scan-folder"
                :class="{ disabled: !hasSelectable(row.node), browser: row.node.depth === 0 }"
                :style="{ paddingLeft: 10 + row.node.depth * 16 + 'px' }"
                :title="row.node.path"
                role="button"
                tabindex="0"
                @click="toggleExpand(row.node.path)"
                @keydown.enter.prevent="toggleExpand(row.node.path)"
                @keydown.space.prevent="toggleExpand(row.node.path)"
              >
                <ChevronRight
                  :size="14"
                  :stroke-width="2.2"
                  class="scan-chev"
                  :class="{ open: expandedFolders.has(row.node.path) }"
                  aria-hidden="true"
                />
                <span
                  class="scan-tri"
                  :class="folderCheckState(row.node)"
                  :title="
                    folderCheckState(row.node) === 'all'
                      ? row.node.depth === 0
                        ? `Clear all bookmarks in ${row.node.name}`
                        : 'Clear group (including subfolders)'
                      : row.node.depth === 0
                        ? `Select all bookmarks in ${row.node.name}`
                        : 'Select group (including subfolders)'
                  "
                  @click.stop="toggleFolderCheck(row.node)"
                >
                  <Check v-if="folderCheckState(row.node) === 'all'" :size="12" :stroke-width="3" />
                  <Minus
                    v-else-if="folderCheckState(row.node) === 'part'"
                    :size="12"
                    :stroke-width="3"
                  />
                </span>
                <Globe
                  v-if="row.node.depth === 0"
                  :size="13"
                  :stroke-width="2"
                  class="scan-browser-ico"
                  aria-hidden="true"
                />
                <span class="scan-folder-name">{{ row.node.name }}</span>
                <!-- 小类映射：默认自动推导，可直接改（留空=归默认小类，改同名=合并多个文件夹）；浏览器顶层行不参与 -->
                <span v-if="groupByFolder && row.node.depth > 0" class="scan-cat-wrap" @click.stop>
                  <span class="scan-cat-label">小类</span>
                  <input
                    class="scan-cat-input"
                    type="text"
                    maxlength="60"
                    :value="
                      categoryOverrides.get(row.node.rawFolder) ??
                      folderToCategory(row.node.rawFolder) ??
                      ''
                    "
                    placeholder="Default"
                    title="Quick Launch category for imported bookmarks. Use / for nested categories (e.g. Development/Frontend). Edit as needed; leave blank to use the default. Folders with the same category name are merged."
                    @keydown.stop
                    @input="onCatInput(row.node.rawFolder, $event)"
                  />
                </span>
                <span
                  class="scan-folder-count"
                  :title="
                    row.node.depth === 0
                      ? browserRowTitle(row.node)
                      : `${row.node.items.length} here · ${row.node.total} in subfolders`
                  "
                  >{{ folderCountText(row.node) }}</span
                >
              </div>
              <!-- 书签行（树内按层级缩进） -->
              <label
                v-else
                class="scan-row"
                :class="{ disabled: isExisting(row.item), selected: checked.has(keyOf(row.item)) }"
                :style="row.indent > 0 ? { paddingLeft: 10 + row.indent * 16 + 'px' } : undefined"
              >
                <input
                  type="checkbox"
                  class="scan-checkbox"
                  :checked="checked.has(keyOf(row.item))"
                  :disabled="isExisting(row.item)"
                  @change="toggleItem(row.item)"
                />
                <span
                  class="scan-icon"
                  :style="showImg(row.item) ? {} : { background: accentOf(row.item.name).soft }"
                >
                  <img
                    v-if="showImg(row.item)"
                    class="scan-img"
                    :src="iconSrc(row.item.icon!)"
                    alt=""
                    @error="onImgError(row.item)"
                  />
                  <span
                    v-else
                    class="scan-letter"
                    :style="{ color: accentOf(row.item.name).text }"
                  >
                    {{ row.item.name.charAt(0).toUpperCase() }}
                  </span>
                </span>
                <span class="scan-info">
                  <span class="scan-name" :title="row.item.name">{{ row.item.name }}</span>
                  <span class="scan-target" :title="row.item.target">{{ row.item.target }}</span>
                </span>
                <span v-if="isExisting(row.item)" class="scan-added">Already added</span>
                <span v-else class="scan-check" :class="{ on: checked.has(keyOf(row.item)) }">
                  <Check v-if="checked.has(keyOf(row.item))" :size="13" :stroke-width="3" />
                </span>
              </label>
            </template>
          </div>

          <!-- 底部操作 -->
          <footer v-if="!loading && items.length > 0" class="scan-footer">
            <span class="scan-count">{{ selectedCount }} selected</span>
            <div class="scan-actions">
              <button class="ghost-btn btn" @click="emit('close')">Cancel</button>
              <button class="pill-btn btn" :disabled="selectedCount === 0" @click="confirm">
                {{ conf.confirm }}（{{ selectedCount }}）
              </button>
            </div>
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.scan-card {
  width: 560px;
  max-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
  padding: 20px;
}
.dialog-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-1);
}
.scan-sub {
  margin-top: 2px;
  font-size: 0.75rem;
  color: var(--text-3);
}
.scan-stats {
  margin-top: 10px;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-2);
}
/* 书签数量口径说明（合并/跳过/截断非零才出现） */
.scan-note {
  margin-top: 6px;
  padding: 0 2px;
  font-size: 0.6875rem;
  line-height: 1.5;
  color: var(--text-3);
}
.scan-warn {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  margin-top: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  background: var(--c-orange-soft);
  border: 1px solid var(--c-orange);
  color: var(--text-1);
  font-size: 0.75rem;
  line-height: 1.5;
}
.scan-warn svg {
  flex-shrink: 0;
  margin-top: 2px;
  color: var(--c-orange);
}
.scan-warn b {
  font-weight: 700;
}
.scan-clean {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  margin-top: 8px;
  padding: 0 2px;
  font-size: 0.75rem;
  line-height: 1.5;
  color: var(--text-2);
  cursor: pointer;
  user-select: none;
}
.scan-clean input {
  margin-top: 2px;
  flex-shrink: 0;
  accent-color: var(--brand-500);
  cursor: pointer;
}
.scan-clean b {
  color: var(--text-1);
  font-weight: 600;
}
.scan-clean.disabled {
  opacity: 0.5;
  cursor: default;
}
.scan-clean.disabled input {
  cursor: default;
}
.scan-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 14px 0 10px;
}
.scan-search-wrap {
  position: relative;
  flex: 1;
  min-width: 0;
}
.scan-search {
  padding-left: 32px;
}
.scan-search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-3);
  pointer-events: none;
}
.scan-select-all {
  flex-shrink: 0;
  padding: 6px 14px;
}
.scan-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  padding: 4px;
}
.scan-group {
  position: sticky;
  top: 0;
  z-index: 1;
  margin: 2px 0;
  padding: 6px 10px 4px;
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-3);
  background: var(--bg-card-solid);
  letter-spacing: 0.02em;
}
/* 书签分组头：映射到的速达小类名预览（品牌色弱化，不与分组名抢视觉） */
.scan-group-cat {
  margin-left: 6px;
  font-weight: 500;
  color: var(--brand-500);
}
/* 文件夹行的小类映射输入：可改（留空=归默认小类，同名=合并），品牌色呼应预览语义 */
.scan-cat-wrap {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  flex-shrink: 0;
}
.scan-cat-label {
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-3);
}
.scan-cat-input {
  width: 106px;
  padding: 3px 8px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--brand-500);
  font-size: 0.75rem;
  font-weight: 500;
  transition: border-color 0.15s;
}
.scan-cat-input:focus {
  outline: none;
  border-color: var(--brand-500);
}
.scan-cat-input::placeholder {
  color: var(--text-4);
  font-weight: 400;
}
/* 文件夹树行（书签模式）：chevron + 三态勾选框 + 名称 + 小类预览 + 子树计数 */
.scan-folder {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  margin: 2px 0;
  border-radius: var(--radius-md);
  cursor: pointer;
  user-select: none;
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-1);
}
.scan-folder:hover {
  background: var(--bg-card-soft);
}
.scan-folder.disabled {
  opacity: 0.45;
  cursor: default;
}
.scan-folder.disabled:hover {
  background: transparent;
}
/* 顶层浏览器行：整组勾选的宿主，名称略强于目录行 */
.scan-folder.browser {
  font-weight: 700;
}
.scan-browser-ico {
  flex-shrink: 0;
  color: var(--text-3);
}
.scan-chev {
  flex-shrink: 0;
  color: var(--text-3);
  transition: transform 0.15s;
}
.scan-chev.open {
  transform: rotate(90deg);
}
/* 三态勾选框（原生 checkbox 表达不了半选态，自绘）：all=品牌色实底勾 / part=品牌色描边横杠 / none=空 */
.scan-tri {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  border: 1.5px solid var(--border-strong);
  border-radius: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-card-solid);
  color: var(--text-on-accent);
  transition: background 0.15s, border-color 0.15s;
  cursor: pointer;
}
.scan-tri.all {
  background: var(--brand-500);
  border-color: var(--brand-500);
}
.scan-tri.part {
  border-color: var(--brand-500);
  color: var(--brand-500);
}
.scan-folder.disabled .scan-tri {
  cursor: default;
}
.scan-folder-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.scan-folder-count {
  margin-left: auto;
  flex-shrink: 0;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-3);
}
.scan-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background 0.12s;
}
.scan-row:hover {
  background: var(--bg-card-soft);
}
.scan-row.disabled {
  opacity: 0.45;
  cursor: default;
}
.scan-row.disabled:hover {
  background: transparent;
}
.scan-checkbox {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}
.scan-icon {
  width: 38px;
  height: 38px;
  border-radius: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
}
.scan-img {
  width: 38px;
  height: 38px;
  object-fit: contain;
  background: var(--bg-card);
}
.scan-letter {
  font-size: 1.0625rem;
  font-weight: 700;
}
.scan-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.scan-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.scan-target {
  font-size: 0.6875rem;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.scan-added {
  flex-shrink: 0;
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--c-green);
  background: var(--c-green-soft);
  padding: 2px 8px;
  border-radius: var(--radius-pill);
}
.scan-check {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border: 1.5px solid var(--border-strong);
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-card-solid);
  color: var(--text-on-accent);
  transition: background 0.15s, border-color 0.15s;
}
.scan-check.on {
  background: var(--brand-500);
  border-color: var(--brand-500);
}
.scan-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 14px;
}
.scan-count {
  font-size: 0.75rem;
  color: var(--text-3);
}
.scan-actions {
  display: flex;
  gap: 10px;
}
.btn {
  padding: 7px 16px;
}
.scan-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 52px 16px;
  color: var(--text-3);
  font-size: 0.8125rem;
}
.scan-state span {
  font-size: 0.6875rem;
  color: var(--text-4);
}
.scan-error {
  color: var(--c-red);
}
.spin {
  animation: scan-spin 0.9s linear infinite;
}
@keyframes scan-spin {
  to {
    transform: rotate(360deg);
  }
}

.mask-enter-active,
.mask-leave-active {
  transition: opacity 0.18s ease-out;
}
.mask-enter-from,
.mask-leave-to {
  opacity: 0;
}
</style>
