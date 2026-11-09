<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import { ChevronDown, Copy, ExternalLink, Eye, EyeOff, FlaskConical, ListPlus, Pencil, Plus, Trash2, X } from 'lucide-vue-next'
import { isPlatformModel, isTauri, tauriApi, type ChatModelConfig, type ProviderPreset } from '../api/tauri'
import { useStore } from '../stores/workbench'

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()

// ---- 供应商编辑态（一个供应商 = 一个 base_url + 共享 API Key + 多个模型） ----
interface ProviderEdit {
  key: string
  providerName: string
  baseUrl: string
  apiKey: string
  hasApiKey: boolean
  models: ChatModelConfig[]
  expanded: boolean
  fetched: string[]
  selected: Set<string>
  /** 手动添加模型输入框的草稿（与在线拉取并存，见 addManualModel） */
  manualModel: string
  busy: boolean
  msg: string
  savedKey: string
  keyVisible: boolean
  editingKey: boolean
  /** 点选预设带出的「获取 Key」指引页（空 = 不显示入口；手动填写的供应商没有） */
  keyGuide: string
}

const providers = ref<ProviderEdit[]>([])
const loading = ref(false)
const saving = ref(false)

// ---- 平台条目（prop 回传）----
// 平台额度的开关与逻辑在设置页 AiPanel.vue 的「额度」子项里；这里只经 prop 拿到当前平台
// 条目，供 collectAll 整体保存时原样带回（平台条目不渲染成卡片，漏掉 = 悄悄关掉平台额度）。
// 条目按 `platform:<模型名>` 存（后端靠前缀识别，见 chat.rs::is_platform_model），Key 是
// 占位符，请求时由 Rust 换成账号 token（PLATFORM_KEY_SENTINEL）——绝不在此展示/复制。
const props = defineProps<{ platformModels?: ChatModelConfig[] }>()

function groupKey(m: ChatModelConfig): string {
  const name = (m.provider_name ?? '').trim()
  const base = (m.base_url ?? '').trim()
  // 一个供应商 = 一个供应商名称 + 多个模型；分组以名称为准：
  // 多账号可能共用同一个 base_url，但供应商名称不同，必须各自成组
  return name || base
}

function toProvider(m: ChatModelConfig): ProviderEdit {
  return {
    key: groupKey(m),
    providerName: (m.provider_name ?? '').trim(),
    baseUrl: (m.base_url ?? '').trim(),
    apiKey: '',
    hasApiKey: m.has_api_key,
    models: [m],
    expanded: false,
    fetched: [],
    selected: new Set(),
    manualModel: '',
    busy: false,
    msg: '',
    savedKey: '',
    keyVisible: false,
    editingKey: false,
    keyGuide: '',
  }
}

/// 拉取模型配置并重建供应商卡片列表。
///
/// `silent` = 后台回读（开关切换 / 保存配置之后）：**不进 loading 态**，否则模板会把整块
/// 卡片列表换成「加载中…」再换回来 —— 用户看到的就是开关点下去时界面闪一下。
/// 首次挂载与显式 reload 才走 loading（那时列表本来就还没内容，占位是诚实的）。
async function loadProviders(opts: { silent?: boolean } = {}) {
  if (!isTauri()) return
  if (!opts.silent) loading.value = true
  try {
    const list = await tauriApi.getChatModels()
    // 平台条目不渲染成供应商卡片（开关在设置页「额度」子项），按自备供应商分组
    const prev = new Map(providers.value.map((p) => [p.key, p]))
    const map = new Map<string, ProviderEdit>()
    for (const m of list) {
      if (isPlatformModel(m)) continue
      const p = toProvider(m)
      if (map.has(p.key)) {
        map.get(p.key)!.models.push(m)
      } else {
        map.set(p.key, p)
      }
    }
    // 重建卡片时继承已在界面上的展开态/勾选态：开关切换、保存配置都会回读一次，
    // 若每次回读都把卡片收回折叠态，用户会看到自己刚展开的卡片"自己关上了"
    providers.value = [...map.values()].map((p) => {
      const old = prev.get(p.key)
      if (!old) return p
      return {
        ...p,
        expanded: old.expanded,
        fetched: old.fetched,
        selected: old.selected,
        manualModel: old.manualModel,
        msg: old.msg,
        savedKey: old.savedKey,
        keyVisible: old.keyVisible,
      }
    })
  } catch (e) {
    showToast(`Failed to load: ${String(e)}`)
  } finally {
    if (!opts.silent) loading.value = false
  }
}

function addProvider() {
  providers.value.push({
    key: 'p' + Date.now(),
    providerName: '',
    baseUrl: '',
    apiKey: '',
    hasApiKey: false,
    models: [],
    expanded: true,
    fetched: [],
    selected: new Set(),
    manualModel: '',
    busy: false,
    msg: '',
    savedKey: '',
    keyVisible: false,
    editingKey: false,
    keyGuide: '',
  })
}

/** 点选预设：填名称/地址 + 带出 Key 指引，备注就地展示（#t47）。只填空卡片，不覆盖已填内容 */
function applyPreset(p: ProviderEdit, preset: ProviderPreset) {
  p.providerName = preset.name
  p.baseUrl = preset.baseUrl
  p.keyGuide = preset.keyGuideUrl
  p.msg = preset.note
  syncProviderFields(p)
}

function removeProvider(index: number) {
  providers.value.splice(index, 1)
}

function expand(p: ProviderEdit) {
  p.expanded = !p.expanded
  if (p.expanded && p.models.length > 0 && !p.hasApiKey) {
    p.hasApiKey = p.models.some((m) => m.has_api_key)
  }
  if (p.expanded && p.hasApiKey && !p.savedKey) {
    void loadSavedKey(p)
  }
}

// 拉取已保存的 API Key（脱敏展示/查看/复制用；真实 Key 存钥匙串，仅本机读取）
async function loadSavedKey(p: ProviderEdit) {
  if (!isTauri()) return
  const id = p.models.find((m) => m.has_api_key)?.id ?? p.models[0]?.id
  if (!id) return
  try {
    p.savedKey = await tauriApi.getChatApiKey(id)
  } catch {
    p.savedKey = ''
  }
}

// 中间脱敏：保留首 4 尾 4 字符，中间以星号掩盖
function maskKey(k: string): string {
  if (!k) return 'Key not found'
  if (k.length <= 8) return '•'.repeat(k.length)
  return k.slice(0, 4) + '•'.repeat(k.length - 8) + k.slice(-4)
}

// 复制文本（剪贴板 API 不可用时回退隐藏 textarea + execCommand）
async function copyText(text: string, okMsg: string): Promise<void> {
  if (!text) {
    showToast('Nothing to copy')
    return
  }
  try {
    await navigator.clipboard.writeText(text)
    showToast(okMsg)
    return
  } catch {
    // 继续尝试 execCommand 回退
  }
  const ta = document.createElement('textarea')
  ta.value = text
  ta.style.position = 'fixed'
  ta.style.opacity = '0'
  document.body.appendChild(ta)
  ta.select()
  let ok = false
  try {
    ok = document.execCommand('copy')
  } catch {
    ok = false
  }
  document.body.removeChild(ta)
  showToast(ok ? okMsg : 'Copy failed')
}

// 复制当前 Key
function copyKey(p: ProviderEdit) {
  const k = p.apiKey.trim() || p.savedKey
  void copyText(k, 'API key copied')
}

// 复制模型名称（tag 形式展示）
function copyModelName(m: ChatModelConfig) {
  void copyText(m.model || m.name, 'Model name copied')
}

// 更换 Key 输入失焦时：未输入内容则回到脱敏展示
function finishKeyEdit(p: ProviderEdit) {
  if (p.hasApiKey && !p.apiKey.trim()) {
    p.editingKey = false
  }
}

// 编辑供应商字段时，同步到该供应商下的所有模型（模型共享 base_url / provider_name）
function syncProviderFields(p: ProviderEdit) {
  for (const m of p.models) {
    m.provider_name = p.providerName.trim()
    m.base_url = p.baseUrl.trim()
  }
}

// 供应商上填写的 API Key：保存时写入该供应商所有模型
function applyKey(p: ProviderEdit) {
  const k = p.apiKey.trim()
  if (!k) return
  for (const m of p.models) {
    m.api_key = k
    m.has_api_key = true
  }
}

// 连通性测试：只验证 URL + Key 能否连通
async function testProvider(p: ProviderEdit) {
  if (!isTauri() || p.busy) return
  const baseUrl = p.baseUrl.trim()
  if (!baseUrl) {
    p.msg = 'Enter a Base URL first'
    return
  }
  p.busy = true
  p.msg = ''
  try {
    const keyId = p.hasApiKey && !p.apiKey.trim() && p.models.length > 0 ? p.models[0].id : undefined
    await tauriApi.fetchChatProviderModels(baseUrl, p.apiKey.trim(), keyId)
    p.msg = 'Connection successful'
  } catch (e) {
    p.msg = `Connection failed: ${String(e)}`
  } finally {
    p.busy = false
  }
}

// 获取模型列表：拉取供应商可用模型，勾选后加入当前配置
async function fetchModels(p: ProviderEdit) {
  if (!isTauri() || p.busy) return
  const baseUrl = p.baseUrl.trim()
  if (!baseUrl) {
    p.msg = 'Enter a Base URL first'
    return
  }
  p.busy = true
  p.msg = ''
  try {
    const keyId = p.hasApiKey && !p.apiKey.trim() && p.models.length > 0 ? p.models[0].id : undefined
    const ids = await tauriApi.fetchChatProviderModels(baseUrl, p.apiKey.trim(), keyId)
    p.fetched = ids
    p.selected = new Set()
    if (ids.length === 0) p.msg = 'No available models found'
    else p.msg = `Found ${ids.length} models. Select the ones you want, then click “Add”`
  } catch (e) {
    p.msg = `Failed to fetch models: ${String(e)}`
  } finally {
    p.busy = false
  }
}

function toggleFetched(p: ProviderEdit, id: string) {
  if (p.selected.has(id)) p.selected.delete(id)
  else p.selected.add(id)
}

// 把勾选的模型加入当前供应商（供应商名称不能为空，否则无法保存）
function addSelected(p: ProviderEdit) {
  const name = p.providerName.trim()
  if (!name) {
    p.msg = 'Enter a provider name before adding models'
    return
  }
  const ids = p.fetched.filter((id) => p.selected.has(id))
  if (ids.length === 0) return
  for (const id of ids) {
    const already = p.models.some((m) => m.model === id)
    if (already) continue
    p.models.push({
      id: 'm' + Date.now() + '-' + Math.random().toString(36).slice(2, 6),
      name: id,
      provider_name: name,
      base_url: p.baseUrl.trim(),
      model: id,
      api_key: '',
      is_default: false,
      has_api_key: p.hasApiKey,
    })
  }
  if (p.models.length === 1) p.models[0].is_default = true
  p.fetched = []
  p.selected = new Set()
  p.msg = `Added ${ids.length} models`
}

// 手动添加模型：不少 OpenAI 兼容中转不实现 /models 接口或返回空列表，在线拉取
// 一个模型都拿不到、原来就没有任何添加入口。这里直接按模型名加入当前供应商，
// 与「获取模型」在线拉取并存（勾选流程照旧可用）。
function addManualModel(p: ProviderEdit) {
  const id = p.manualModel.trim()
  if (!id) {
    p.msg = 'Enter a model name'
    return
  }
  const name = p.providerName.trim()
  if (!name) {
    p.msg = 'Enter a provider name before adding models'
    return
  }
  if (p.models.some((m) => m.model === id)) {
    p.msg = `Model “${id}” is already in the list`
    p.manualModel = ''
    return
  }
  p.models.push({
    id: 'm' + Date.now() + '-' + Math.random().toString(36).slice(2, 6),
    name: id,
    provider_name: name,
    base_url: p.baseUrl.trim(),
    model: id,
    api_key: '',
    is_default: false,
    has_api_key: p.hasApiKey,
  })
  if (p.models.length === 1) p.models[0].is_default = true
  p.manualModel = ''
  p.msg = `Added model “${id}”`
}

function removeModel(p: ProviderEdit, id: string) {
  const i = p.models.findIndex((m) => m.id === id)
  if (i >= 0) {
    p.models.splice(i, 1)
    if (p.models.length > 0 && !p.models.some((m) => m.is_default)) p.models[0].is_default = true
  }
}

function collectAll(): ChatModelConfig[] {
  const out: ChatModelConfig[] = []
  for (const p of providers.value) {
    applyKey(p)
    out.push(...p.models)
  }
  // 平台条目不在 providers 里（开关在设置页「额度」子项，条目经 prop 传入），但必须原样带回：
  // saveAll 是整体覆盖写配置，漏掉它等于把用户开着的平台额度悄悄关掉
  out.push(...(props.platformModels ?? []))
  // 全局默认归一
  const hasDefault = out.some((m) => m.is_default)
  if (!hasDefault && out.length > 0) out[0].is_default = true
  return out
}

async function saveAll() {
  if (!isTauri() || saving.value) return
  // ① 每张供应商卡片都必须至少有一个模型。
  // 关键：以前这类「填了名称/地址却忘了点获取模型」的卡片会被**静默丢弃**——
  // 保存提示成功，实际什么都没配上；用户以为配好了，回到对话里却还是没模型可选。
  for (const p of providers.value) {
    if (p.models.length === 0) {
      const label = p.providerName.trim() || p.baseUrl.trim() || 'Unnamed provider'
      showToast(`Provider “${label}” has no models. Click “Fetch models” to load them or enter a model name manually. To remove this provider, click the delete button in the upper-right corner.`)
      return
    }
  }
  // ② 供应商名称约束：不能为空、不能重复
  const seen = new Set<string>()
  for (const p of providers.value) {
    const n = p.providerName.trim()
    if (!n) {
      showToast('Provider name can’t be empty')
      return
    }
    if (seen.has(n)) {
      showToast(`Provider name already exists: ${n}`)
      return
    }
    seen.add(n)
  }
  // ③ 一个模型都没有也不让保存（一张卡片都没有、平台额度也没开）：
  // 空配置会让 AI 对话里没有任何可选模型
  const all = collectAll()
  if (all.length === 0) {
    showToast('No models are available. Click “Fetch models” to load them, enter a model name manually, or enable “x-hub platform credits” above.')
    return
  }
  saving.value = true
  try {
    const saved = await tauriApi.saveChatModels(all)
    // 同步进内存快照：后续任意 saveConfig 都带着最新模型，不会被旧快照覆盖
    store.setChatModels(saved)
    showToast('Provider settings saved')
    // 同样走静默回读：保存后把卡片列表换成「加载中…」再换回来，是一次没必要的整块闪烁
    await loadProviders({ silent: true })
  } catch (e) {
    showToast(`Failed to save: ${String(e)}`)
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  void loadProviders()
  void loadPresets()
})

defineExpose({ reload: () => void loadProviders() })

// ---- 供应商预设（#t47：后台可配；未登录/接口挂了回落内置清单，不阻塞配置） ----
const PRESET_FALLBACK: ProviderPreset[] = [
  { name: 'DeepSeek', baseUrl: 'https://api.deepseek.com/v1', keyGuideUrl: 'https://platform.deepseek.com/api_keys', note: 'Official DeepSeek API with competitive pricing' },
  { name: 'OpenAI', baseUrl: 'https://api.openai.com/v1', keyGuideUrl: 'https://platform.openai.com/api-keys', note: '' },
  { name: 'Moonshot AI', baseUrl: 'https://api.moonshot.cn/v1', keyGuideUrl: 'https://platform.moonshot.cn/console/api-keys', note: '' },
  { name: 'Alibaba Cloud Bailian', baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1', keyGuideUrl: 'https://bailian.console.aliyun.com/?apiKey=1', note: 'Qwen models, direct connection in China' },
  { name: 'Ollama (Local)', baseUrl: 'http://localhost:11434/v1', keyGuideUrl: '', note: 'Local models; no API key required' },
]

const presets = ref<ProviderPreset[]>(PRESET_FALLBACK)

async function loadPresets() {
  if (!isTauri()) return
  try {
    const list = await tauriApi.platformProviderPresets()
    if (list.length) presets.value = list
  } catch {
    // 未登录 / 网络失败：保持内置清单，不打扰用户
  }
}

/// 「获取 Key」指引页走系统浏览器（设置页内嵌跳转没有必要，也更安全）
function openGuide(url: string) {
  if (!url) return
  void tauriApi.openExternal(url).catch(() => showToast('Couldn’t open the browser. Copy the link manually.'))
}
</script>

<template>
  <div class="ai-providers">
    <p class="ai-intro">Configure OpenAI-compatible model providers such as DeepSeek, OpenAI, or Ollama. Enter a Base URL and API key to test the connection, fetch available models, and select which ones to add. API keys are stored only in the system keychain.</p>

    <!-- 只有「还没有内容」时才让占位顶掉列表：已有卡片时的刷新一律静默，避免整块闪一下 -->
    <div v-if="loading && providers.length === 0" class="ai-loading">Loading…</div>

    <template v-else>
      <!-- 供应商折叠卡片列表 -->
      <div v-for="(p, pi) in providers" :key="p.key + pi" class="prov-card" :class="{ open: p.expanded }">
        <!-- 折叠头部：列表式展示 -->
        <div class="prov-head" @click="expand(p)">
          <ChevronDown class="prov-chevron" :size="15" :class="{ on: p.expanded }" />
          <div class="prov-title">
            <span class="prov-name" :title="p.providerName || p.baseUrl || 'Unnamed provider'">{{ p.providerName || p.baseUrl || 'Unnamed provider' }}</span>
            <span class="prov-meta" :title="p.baseUrl || 'No address set'">
              {{ p.baseUrl || 'No address set' }}
              <template v-if="p.models.length"> · {{ p.models.length }} models</template>
              <template v-if="p.models.find((m) => m.is_default)"> · Default: {{ p.models.find((m) => m.is_default)?.name }}</template>
            </span>
          </div>
          <button class="ghost-btn prov-del" title="Delete provider" @click.stop="removeProvider(pi)">
            <Trash2 :size="13" />
          </button>
        </div>

        <!-- 展开体：编辑 + 获取模型 + 模型列表 -->
        <div v-if="p.expanded" class="prov-body">
          <!-- 供应商预设（#t47：后台可配；只对全新空卡片显示，不覆盖已填内容） -->
          <div v-if="!p.models.length && !p.baseUrl.trim()" class="preset-row">
            <span class="preset-label">Quick fill</span>
            <button
              v-for="ps in presets"
              :key="ps.name"
              class="preset-chip"
              type="button"
              :title="ps.note || ps.baseUrl"
              @click="applyPreset(p, ps)"
            >
              {{ ps.name }}
            </button>
          </div>
          <div class="ai-field">
            <label class="ai-label">Provider name</label>
            <input v-model="p.providerName" class="field-input" placeholder="e.g. DeepSeek" @input="syncProviderFields(p)" />
          </div>
          <div class="ai-field">
            <label class="ai-label">Base URL</label>
            <input v-model="p.baseUrl" class="field-input" placeholder="https://api.deepseek.com/v1" @input="syncProviderFields(p)" />
          </div>
          <div class="ai-field">
            <label class="ai-label">API Key</label>
            <div class="ai-key-row">
              <!-- 已保存 Key：脱敏展示 + 眼睛查看全部 + 复制 -->
              <template v-if="p.hasApiKey && !p.editingKey">
                <div class="key-display">
                  <span class="key-text" :title="p.keyVisible ? p.savedKey : 'Click the eye to reveal the full key'">
                    {{ p.keyVisible ? p.savedKey : maskKey(p.savedKey) }}
                  </span>
                  <button
                    class="key-btn"
                    :title="p.keyVisible ? 'Hide' : 'Show full key'"
                    @click="p.keyVisible = !p.keyVisible"
                  >
                    <EyeOff v-if="p.keyVisible" :size="13" />
                    <Eye v-else :size="13" />
                  </button>
                  <button class="key-btn" title="Copy current key" @click="copyKey(p)">
                    <Copy :size="13" />
                  </button>
                  <button class="key-btn" title="Change key" @click="p.editingKey = true">
                    <Pencil :size="13" />
                  </button>
                </div>
              </template>
              <!-- 输入新 Key -->
              <input
                v-else
                v-model="p.apiKey"
                class="field-input"
                :type="p.hasApiKey && !p.apiKey ? 'password' : 'text'"
                :placeholder="p.hasApiKey ? 'Enter a new key to replace it (leave blank to keep the current key)' : 'sk-…'"
                autocomplete="off"
                @blur="finishKeyEdit(p)"
              />
              <button class="ghost-btn prov-test" :disabled="p.busy" @click="testProvider(p)">
                <FlaskConical :size="13" />
                {{ p.busy ? 'Testing…' : 'Test connection' }}
              </button>
              <button class="ghost-btn prov-fetch" :disabled="p.busy" @click="fetchModels(p)">
                <ListPlus :size="13" />
                {{ p.busy ? 'Fetching…' : 'Fetch models' }}
              </button>
            </div>
            <!-- 预设带出的「获取 Key」指引：走系统浏览器（#t47） -->
            <a v-if="p.keyGuide" class="key-guide" :href="p.keyGuide" @click.prevent="openGuide(p.keyGuide)">
              <ExternalLink :size="12" /> Get {{ p.providerName || 'provider' }} API key
            </a>
          </div>

          <p v-if="p.msg" class="prov-msg" :class="{ ok: p.msg.startsWith('Connection successful') || p.msg.startsWith('Added') || p.msg.startsWith('Found') }">{{ p.msg }}</p>

          <!-- 获取到的模型列表：勾选添加 -->
          <div v-if="p.fetched.length" class="fetch-list">
            <div class="fetch-head">Available models (select to add)</div>
            <label v-for="id in p.fetched" :key="id" class="fetch-item">
              <input type="checkbox" :checked="p.selected.has(id)" @change="toggleFetched(p, id)" />
              <span class="fetch-id" :title="id">{{ id }}</span>
            </label>
            <button class="ghost-btn fetch-add" :disabled="p.selected.size === 0" @click="addSelected(p)">
              Add selected ({{ p.selected.size }})
            </button>
          </div>

          <!-- 手动添加模型：中转不实现 /models 或返回空时，直接填模型名加入（回车或点按钮） -->
          <div class="manual-model-row">
            <input
              v-model="p.manualModel"
              class="field-input"
              placeholder="Add a model name (e.g. gpt-4o-mini), then press Enter"
              aria-label="Add model name"
              @keydown.enter.prevent="addManualModel(p)"
            />
            <button class="ghost-btn manual-add" :disabled="!p.manualModel.trim()" @click="addManualModel(p)">
              <Plus :size="13" />
              Add
            </button>
          </div>

          <!-- 当前供应商下已配置的模型：tag 形式并排展示，可复制名称/删除 -->
          <div class="models-block">
            <div class="models-head">Configured models</div>
            <div v-if="p.models.length === 0" class="models-empty">No models yet. Click “Fetch models” to load them or enter a model name above.</div>
            <div v-else class="model-tags">
              <span
                v-for="m in p.models"
                :key="m.id"
                class="model-tag"
                :class="{ on: m.is_default }"
                :title="`${m.name}${m.is_default ? ' (default)' : ''}`"
              >
                <span class="model-tag-name" :title="m.name">{{ m.name }}</span>
                <button class="model-tag-btn" title="Copy model name" @click="copyModelName(m)">
                  <Copy :size="11" />
                </button>
                <button class="model-tag-btn del" title="Remove model" @click="removeModel(p, m.id)">
                  <X :size="11" />
                </button>
              </span>
            </div>
          </div>
        </div>
      </div>

      <div class="ai-actions">
        <button class="ghost-btn" @click="addProvider">
          <Plus :size="13" /> Add provider
        </button>
        <button class="ghost-btn ai-save-btn" :disabled="saving" @click="saveAll">
          {{ saving ? 'Saving…' : 'Save settings' }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ---- Agent 权限与个性化样式（#t47）已随设置项迁往 AiPanel「助手」子项 ---- */

/* ---- 供应商预设 chips（#t47） ---- */
.preset-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}
.preset-label {
  font-size: 0.75rem;
  color: var(--text-3);
}
.preset-chip {
  padding: 3px 10px;
  font-size: 0.75rem;
  border: 1px solid var(--border-soft);
  border-radius: 999px;
  background: var(--bg-card);
  color: var(--text-2);
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease;
}
.preset-chip:hover {
  border-color: color-mix(in srgb, var(--brand-500) 45%, transparent);
  color: var(--text-1);
}

/* 预设带出的「获取 Key」指引链接（系统浏览器打开） */
.key-guide {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 6px;
  font-size: 0.75rem;
  color: var(--brand-500);
  text-decoration: none;
}
.key-guide:hover {
  text-decoration: underline;
}

.ai-intro {
  margin: 0 0 var(--space-4);
  font-size: 0.78125rem;
  line-height: 1.6;
  color: var(--text-3);
}
.ai-loading {
  padding: 18px 0;
  font-size: 0.8125rem;
  color: var(--text-3);
}

/* ---- 供应商卡片 ---- */
.prov-card {
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-lg);
  background: var(--bg-card-soft);
  overflow: hidden;
}
.prov-card + .prov-card {
  margin-top: 12px;
}
.prov-card.open {
  border-color: color-mix(in srgb, var(--brand-500) 30%, transparent);
}
.prov-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  cursor: pointer;
  user-select: none;
}
.prov-chevron {
  flex-shrink: 0;
  color: var(--text-3);
  transition: transform 0.18s ease-out;
}
.prov-chevron.on {
  transform: rotate(180deg);
}
.prov-title {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.prov-name {
  font-size: 0.875rem;
  font-weight: 700;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.prov-meta {
  font-size: 0.75rem;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.prov-del {
  flex-shrink: 0;
  padding: 5px 10px;
  font-size: 0.75rem;
}
.prov-del:hover {
  background: var(--c-red-soft);
  color: var(--c-red-ink);
  border-color: var(--c-red-soft);
}

.prov-body {
  padding: 0 14px 14px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  border-top: 1px solid var(--border-soft);
  padding-top: 12px;
}
.ai-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.ai-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-3);
}
.ai-key-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ai-key-row .field-input {
  flex: 1;
}
.key-display {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 10px;
  background: var(--input-bg);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-sm);
}
.key-text {
  flex: 1;
  min-width: 0;
  font-size: 0.78125rem;
  color: var(--text-2);
  font-family: var(--font-mono, ui-monospace, 'Cascadia Code', Consolas, monospace);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.key-btn {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  border-radius: 5px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  cursor: pointer;
}
.key-btn:hover {
  background: var(--bg-card-soft);
  color: var(--text-1);
}
.prov-test {
  flex-shrink: 0;
  padding: 8px 12px;
  font-size: 0.75rem;
}
.prov-fetch {
  flex-shrink: 0;
  padding: 8px 12px;
  font-size: 0.75rem;
  background: var(--brand-50);
  color: var(--brand-500);
  border-color: color-mix(in srgb, var(--brand-500) 35%, transparent);
}
.prov-fetch:hover {
  background: var(--brand-500);
  color: var(--text-on-accent);
  border-color: var(--brand-500);
}
/* 手动添加模型行：与 ai-key-row 同款 flex 布局 */
.manual-model-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.manual-model-row .field-input {
  flex: 1;
}
.manual-add {
  flex-shrink: 0;
  padding: 8px 12px;
  font-size: 0.75rem;
  background: var(--brand-50);
  color: var(--brand-500);
  border-color: color-mix(in srgb, var(--brand-500) 35%, transparent);
}
.manual-add:hover:not(:disabled) {
  background: var(--brand-500);
  color: var(--text-on-accent);
  border-color: var(--brand-500);
}
.prov-msg {
  margin: 0;
  font-size: 0.75rem;
  color: var(--c-red-ink);
}
.prov-msg.ok {
  color: var(--c-green-ink);
}

/* ---- 获取模型列表 ---- */
.fetch-list {
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius-md);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.fetch-head,
.models-head {
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--text-2);
}
.fetch-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.8125rem;
  color: var(--text-1);
  cursor: pointer;
}
.fetch-item input[type='checkbox'] {
  accent-color: var(--brand-500);
}
.fetch-id {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fetch-add {
  align-self: flex-start;
  padding: 5px 12px;
  font-size: 0.75rem;
}

/* ---- 已配置模型：tag 并排展示 ---- */
.models-block {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.models-empty {
  font-size: 0.75rem;
  color: var(--text-3);
}
.model-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.model-tag {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  max-width: 100%;
  padding: 3px 4px 3px 10px;
  font-size: 0.75rem;
  color: var(--brand-500);
  background: var(--brand-50);
  border: 1px solid color-mix(in srgb, var(--brand-500) 45%, transparent);
  border-radius: var(--radius-pill);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
}
.model-tag-name {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
}
.model-tag-btn {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  border: none;
  background: transparent;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  cursor: pointer;
}
.model-tag-btn:hover {
  background: var(--bg-card-soft);
  color: var(--text-1);
}
.model-tag-btn.del:hover {
  background: var(--c-red-soft);
  color: var(--c-red-ink);
}

/* ---- 底部操作 ---- */
.ai-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 16px;
}
.ai-save-btn {
  background: var(--brand-500);
  color: var(--text-on-accent);
  border-color: var(--brand-500);
}
.ai-save-btn:hover {
  background: var(--brand-500);
  color: var(--text-on-accent);
  border-color: var(--brand-500);
}
.ai-save-btn:disabled {
  opacity: 0.55;
}
</style>
