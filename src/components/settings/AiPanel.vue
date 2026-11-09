<script setup lang="ts">
// AI 助手大类（2026-10-16 自「功能」大类整块拉出为一级设置项），分四个子项：
//   助手 —— 权限档与个性化（2026-10-17 自 AiProviders 迁入）
//   面板 —— 对话形态与面板行为
//   额度 —— 平台额度开关（2026-10-17 自 AiProviders 迁入）+ 积分余额（自「账号」并入）
//   供应商 —— 自备供应商与模型管理（AiProviders，平台条目经 prop 回传参与整体保存）
//
// 从 SettingsView.vue 拆出（见该文件顶部说明）：设置页按大类按需加载，
// 首次打开只需外壳 + 当前大类的代码，切大类时才加载对应面板。
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isPlatformModel, isTauri, tauriApi, PLATFORM_ENTRY_NAME, type AccountStatus, type ChatModelConfig } from '../../api/tauri';
import AppSelect from '../AppSelect.vue';
import AiProviders from '../AiProviders.vue';
import { useStore } from '../../stores/workbench';

const showToast = inject<(msg: string) => void>('showToast', () => {})
const store = useStore()

function onChatPanelOpacityInput(e: Event) {
  const v = Number((e.target as HTMLInputElement).value)
  void store.setChatPanelOpacity(v)
}

const CHAT_PANEL_SIDE_OPTIONS = [
  { value: 'right', label: 'Right' },
  { value: 'left', label: 'Left' },
  { value: 'top', label: 'Top' },
  { value: 'bottom', label: 'Bottom' },
] as const

async function onChatPanelSideChange(value: string) {
  const side = value as 'left' | 'right' | 'top' | 'bottom'
  await store.setChatPanelSide(side)
  showToast(`AI chat panel will now slide in from the ${side}`)
}

// AI 对话形态：独立小窗 / 主窗内嵌抽屉（互斥）。开关经后端专用命令落地建窗/隐窗
const chatWindowMode = computed(() => !!store.state.config.chat_window_mode)

async function onToggleChatWindowMode() {
  const next = !chatWindowMode.value
  try {
    await store.setChatWindowMode(next)
    showToast(next ? 'AI chat will now open in a separate window' : 'AI chat will now open in the main window panel')
  } catch {
    showToast('Could not change the setting. Try again.')
  }
}

// ---- 平台额度开关（原 AiProviders 的 .ap-platform 行，2026-10-17 迁入「额度」子项） ----
// 形态：只是一个**开启/关闭**开关 —— 它不是供应商卡片（没有供应商名称/Base URL/
// API Key/测试连通/获取模型这些字段，用户也不需要关心平台侧有哪几个模型）。
// 写入的条目仍按 `platform:<模型名>` 存（后端靠这个前缀识别平台额度，见 chat.rs::is_platform_model），
// Key 写占位符「用账号会话换额度」，Rust 侧请求时再换成真实 token（chat.rs::PLATFORM_KEY_SENTINEL）。
// ⚠️ 平台 Key 一律不在界面展示/复制（占位符没有意义，真 token 也不该出 Rust）。
// AI 对话里这批条目折叠成「x-hub 平台」一个入口，具体用哪个模型由后端在多个平台模型之间负载切换。
// ⚠️ platformModels 会经 prop 传给 AiProviders 的 collectAll——供应商保存是整体覆盖写配置，
//    漏掉平台条目等于把用户开着的平台额度悄悄关掉。
const PLATFORM_SENTINEL = '__xhub_platform__'
const platformBusy = ref(false)
/// 平台条目：不渲染成卡片，但必须原样留在配置里（丢了等于关掉平台额度）
const platformModels = ref<ChatModelConfig[]>([])

const platformEnabled = computed(() => platformModels.value.length > 0)

/// 开关下方的状态说明：只说「开没开、平台现在有几个模型、对话里显示成什么」，
/// 具体是哪些模型不在这里列（用户不需要选，后端会负载切换）
const platformStatusText = computed(() => {
  if (!platformEnabled.value) {
    return 'Sign in to use models provided by the x-hub platform. They use platform credits and don’t require an API key. When enabled, “x-hub platform” appears in the AI chat model list.'
  }
  const n = platformModels.value.length
  return n > 1
    ? `Enabled · ${n} platform models available. They appear as “x-hub platform” in AI chat and are load-balanced automatically.`
    : `Enabled · ${n} platform model available. It appears as “x-hub platform” in AI chat.`
})

/// 平台模型的列表与地址只能从平台账号接口取。
///
/// 平台中转只实现 `POST /v1/chat/completions`，**没有** OpenAI 的 `GET {base}/models`
/// ——实测该路径恒 404（2026-09-17 用户反馈「连接失败：接口返回 404」）；
/// 平台自己的列表接口是 `/api/v1/ai/models`，鉴权走**账号登录态**而不是 API Key。
/// 所以平台额度一律走这里，绝不能套用自备供应商那条 `GET {base_url}/models` 的通用探测。
async function loadPlatformModels(): Promise<{ ids: string[]; baseUrl: string }> {
  const account = await tauriApi.accountStatus()
  if (!account.loggedIn) throw new Error('Sign in first under Settings → Account.')
  // 服务端地址是内置常量（设置里没有地址入口），后端正常都会回非空值；
  // 这里只留防御性兜底，不再提示用户「去填地址」——那个入口已经不存在了
  const server = (account.serverUrl || '').replace(/\/+$/, '')
  if (!server) throw new Error('The account service is temporarily unavailable. Try again later.')
  const names = await tauriApi.platformModels()
  return { ids: names.map((n) => `platform:${n}`), baseUrl: `${server}/v1` }
}

/// 平台条目的统一构造：`platform:` 前缀是后端认出平台模型的依据（chat.rs::resolve_api_key），
/// Key 写占位符（请求时换成账号 token），base_url 后端也会现算（这里只存一份便于展示）。
function platformModel(id: string, baseUrl: string): ChatModelConfig {
  const name = id.replace(/^platform:/, '')
  return {
    id,
    name: `${name}（平台额度）`,
    base_url: baseUrl,
    model: name,
    api_key: PLATFORM_SENTINEL,
    is_default: false,
    has_api_key: true,
    provider_name: PLATFORM_ENTRY_NAME,
  }
}

/// 开启平台额度：拉平台当前可用模型并整体写入配置。
/// 后续服务端加减模型无需重开开关：启动 8s 对账 + 发送撞 400 自愈
/// （`commands::sync_platform_models`）会自动对齐本地快照并广播 chat-models-changed。
async function enablePlatform() {
  if (!isTauri() || platformBusy.value) return
  platformBusy.value = true
  try {
    const { ids, baseUrl } = await loadPlatformModels()
    if (!ids.length) {
      showToast('No platform models are currently available.')
      return
    }
    const existing = (await tauriApi.getChatModels()).filter((m) => !isPlatformModel(m))
    const saved = await tauriApi.saveChatModels([...existing, ...ids.map((id) => platformModel(id, baseUrl))])
    store.setChatModels(saved)
    // 只更新平台条目本身，**不回读整个列表**：开关与自备供应商列表毫无关系，
    // 回读会把卡片全部重建（展开态要重继承），还可能把用户正在填的新供应商卡片冲掉
    platformModels.value = saved.filter(isPlatformModel)
    showToast(
      ids.length > 1
        ? `Platform credits enabled (${ids.length} models; load-balanced in chat)`
        : 'Platform credits enabled',
    )
  } catch (e) {
    showToast(`Could not enable platform credits: ${e}`)
  } finally {
    platformBusy.value = false
  }
}

/// 关闭平台额度：把平台条目从配置里整体移除（对话里的「x-hub 平台」入口随之消失）
async function disablePlatform() {
  if (!isTauri() || platformBusy.value) return
  platformBusy.value = true
  const kept = platformModels.value
  platformModels.value = []
  try {
    const saved = await tauriApi.saveChatModels(await tauriApi.getChatModels().then((list) => list.filter((m) => !isPlatformModel(m))))
    store.setChatModels(saved)
    // 同上：平台条目已在前面就地清空（platformModels.value = []），无需回读整表
    showToast('Platform credits disabled')
  } catch (e) {
    // 保存失败就把条目放回去，别让界面显示成「已关闭」而配置里其实还在
    platformModels.value = kept
    showToast(`Could not disable platform credits: ${e}`)
  } finally {
    platformBusy.value = false
  }
}

// ---- 助手：权限档与个性化（#t47：Rust send_agent_message 缺省回落这套持久化值） ----
// 2026-10-17 自 AiProviders 迁入「助手」子项
const AGENT_PERM_OPTIONS = [
  { value: 'readonly', label: 'Read only' },
  { value: 'ask', label: 'Ask every time' },
  { value: 'full', label: 'Full access' },
] as const
type AgentPermMode = (typeof AGENT_PERM_OPTIONS)[number]['value']

const agentPermMode = ref<AgentPermMode>('ask')
const agentPromptOnDangerous = ref(false)
const agentAboutMe = ref('')
const agentExtTools = ref(true)
const agentSaving = ref(false)

function normalizePermMode(v: string | undefined): AgentPermMode {
  return v === 'readonly' || v === 'full' ? v : 'ask'
}

/** 权限档下拉换值：归一后即时落盘（约定 30：下拉一律 AppSelect，禁原生 select） */
function onPermSelect(v: string) {
  agentPermMode.value = normalizePermMode(v)
  void saveAgentSettings({ agent_permission_mode: agentPermMode.value })
}

/// 即时落盘（select/toggle 点下就存）；textarea 走失焦保存，避免每个按键都写盘
async function saveAgentSettings(patch: Parameters<typeof store.setAgentSettings>[0]) {
  agentSaving.value = true
  try {
    await store.setAgentSettings(patch)
  } catch (e) {
    showToast(`Could not save settings: ${String(e)}`)
  } finally {
    agentSaving.value = false
  }
}

// ---- 积分余额（自「账号」大类并入）：只读展示；兑换邀请码的入口仍在「账号」大类 ----
const account = ref<AccountStatus | null>(null)
const accountFailed = ref(false)

// 平台模型清单后台对账（chat-models-changed）的监听注销句柄
let unlistenModelsChanged: UnlistenFn | null = null

onMounted(async () => {
  if (!isTauri()) return
  // 权限与个性化：从持久化配置读初始值（#t47）
  agentPermMode.value = normalizePermMode(store.state.config.agent_permission_mode)
  agentPromptOnDangerous.value = !!store.state.config.agent_prompt_on_dangerous
  agentAboutMe.value = store.state.config.agent_about_me ?? ''
  agentExtTools.value = store.state.config.agent_ext_tools_enabled !== false
  try {
    // 平台开关的初始状态与积分余额同源（getChatModels / accountStatus），挂载时一并拉取
    platformModels.value = (await tauriApi.getChatModels()).filter(isPlatformModel)
  } catch {
    // 后端不可用（浏览器预览）时保持关闭态
  }
  try {
    account.value = await tauriApi.accountStatus()
  } catch {
    accountFailed.value = true
  }
  // 设置页开着时后台对账完成（启动 8s / 发送 400 自愈）：开关态跟随最新清单，
  // 不然会出现「配置里平台条目已被换血、开关还显示旧的开启数」
  unlistenModelsChanged = await listen('chat-models-changed', async () => {
    try {
      platformModels.value = (await tauriApi.getChatModels()).filter(isPlatformModel)
    } catch {
      // 静默：下一次挂载会重查
    }
  })
})

onBeforeUnmount(() => {
  unlistenModelsChanged?.()
})
</script>

<template>
  <section id="sv-sec-assistant" class="sv-sec" aria-label="Assistant">
    <h3 class="sv-sec-title">Assistant</h3>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Default AI assistant permission level</span>
        <span class="setting-desc">You can change this anytime in the chat. This setting determines the level used each time it opens.</span>
      </div>
      <AppSelect
        class="ap-perm-select"
        :model-value="agentPermMode"
        :options="AGENT_PERM_OPTIONS.map((o) => ({ value: o.value, label: o.label, group: '' }))"
        :menu-min-width="132"
        aria-label="Default AI assistant permission level"
        :title="`Default AI assistant permission level: ${AGENT_PERM_OPTIONS.find((o) => o.value === agentPermMode)?.label ?? ''}`"
        @update:model-value="onPermSelect"
      />
    </div>
    <div v-if="agentPermMode === 'full'" class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Confirm dangerous actions</span>
        <span class="setting-desc">When enabled, deletion and launch actions still require confirmation in Full access mode. Off by default; Full access means actions are performed at your own risk.</span>
      </div>
      <button
        class="toggle"
        role="switch"
        type="button"
        :aria-checked="agentPromptOnDangerous"
        :class="{ on: agentPromptOnDangerous }"
        :disabled="agentSaving"
        @click="agentPromptOnDangerous = !agentPromptOnDangerous; saveAgentSettings({ agent_prompt_on_dangerous: agentPromptOnDangerous })"
      >
        <span class="toggle-knob"></span>
      </button>
    </div>
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Allow tools provided by extensions</span>
        <span class="setting-desc">Service extensions can declare AI tools (such as data queries) in their manifest. The AI follows each tool’s declared permission level and the same approval rules. You can also disable tools for individual extensions in their settings.</span>
      </div>
      <button
        class="toggle"
        role="switch"
        type="button"
        :aria-checked="agentExtTools"
        :class="{ on: agentExtTools }"
        :disabled="agentSaving"
        @click="agentExtTools = !agentExtTools; saveAgentSettings({ agent_ext_tools_enabled: agentExtTools })"
      >
        <span class="toggle-knob"></span>
      </button>
    </div>
    <div class="setting-row">
      <div class="setting-info ap-about">
        <span class="setting-name">About me</span>
        <span class="setting-desc">Share your role, preferences, and useful context with the AI. This is included in the system prompt during chats. Leave blank to omit it.</span>
        <textarea
          v-model="agentAboutMe"
          class="field-input ap-about-input"
          rows="3"
          maxlength="1000"
          placeholder="For example: I’m a backend engineer who uses Go and TypeScript. Keep answers concise and use PowerShell for commands…"
          @blur="saveAgentSettings({ agent_about_me: agentAboutMe })"
        ></textarea>
      </div>
    </div>
  </section>

  <section id="sv-sec-ai" class="sv-sec" aria-label="Panel">
    <h3 class="sv-sec-title">Panel</h3>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Open AI chat in a separate window</span>
        <span class="setting-desc">Open chat in a resizable, always-on-top window. The title bar button, AI chat shortcut (Settings → Shortcuts), and floating ball will open it. The panel in the main window will be disabled; these modes are mutually exclusive.</span>
      </div>
      <button
        class="toggle"
        role="switch"
        type="button"
        :aria-checked="chatWindowMode"
        :class="{ on: chatWindowMode }"
        @click="onToggleChatWindowMode"
      >
        <span class="toggle-knob"></span>
      </button>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">AI chat panel opacity</span>
        <span class="setting-desc">{{ chatWindowMode ? 'Applies only to the embedded panel (currently using a separate window)' : 'Overall opacity of the chat panel (50%–100%)' }}</span>
      </div>
      <div class="opacity-edit">
        <input
          class="opacity-slider"
          type="range"
          min="0.5"
          max="1"
          step="0.05"
          :value="store.state.config.chat_panel_opacity ?? 1"
          :aria-label="'AI chat panel opacity'"
          :disabled="chatWindowMode"
          @input="onChatPanelOpacityInput"
        />
        <span class="opacity-value">{{ Math.round((store.state.config.chat_panel_opacity ?? 1) * 100) }}%</span>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">AI chat panel position</span>
        <span class="setting-desc">{{ chatWindowMode ? 'Applies only to the embedded panel (the separate window can be dragged anywhere)' : 'Choose which edge the panel slides in from. Drag its side to adjust width, or its top or bottom to adjust height.' }}</span>
      </div>
      <AppSelect
        :model-value="store.state.config.chat_panel_side ?? 'right'"
        :options="CHAT_PANEL_SIDE_OPTIONS"
        aria-label="AI chat panel position"
        class="chat-panel-side"
        :disabled="chatWindowMode"
        @update:model-value="onChatPanelSideChange"
      />
    </div>
  </section>

  <section id="sv-sec-credit" class="sv-sec" aria-label="Credits">
    <h3 class="sv-sec-title">Credits</h3>

    <!-- 平台额度开关（自 AiProviders 迁入，原「使用 x-hub 平台免费额度」去掉「免费」二字）：
         与自备供应商刻意区分——它不是供应商，没有需要用户填的东西 -->
    <div class="setting-row ap-platform">
      <div class="setting-info">
        <span class="setting-name">Use x-hub platform credits</span>
        <span class="setting-desc">{{ platformStatusText }}</span>
      </div>
      <button
        class="toggle"
        role="switch"
        type="button"
        :aria-checked="platformEnabled"
        :class="{ on: platformEnabled }"
        :disabled="platformBusy"
        :title="platformEnabled ? 'Disabling this removes platform models from your configuration' : 'Enabling this automatically fetches available platform models'"
        @click="platformEnabled ? disablePlatform() : enablePlatform()"
      >
        <span class="toggle-knob"></span>
      </button>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">AI Credits</span>
        <span class="setting-desc">
          {{
            account
              ? account.inviteRedeemed
                ? `${account.creditRemaining} of ${account.creditTotal} credits remaining · Platform models are charged based on token usage`
                : 'No invite code redeemed yet, so you have no credits. Redeem one under Account.'
              : accountFailed
                ? 'Could not load account status. Try again later.'
                : 'Loading account status…'
          }}
        </span>
      </div>
    </div>
  </section>

  <section id="sv-sec-providers" class="sv-sec" aria-label="Providers">
    <h3 class="sv-sec-title">Providers</h3>

    <!-- platformModels 经 prop 回传：供应商保存是整体覆盖写，平台条目必须原样带回 -->
    <AiProviders :platform-models="platformModels" />
  </section>
</template>

<style scoped>
/* 权限档下拉固定宽（自 AiProviders 迁入） */
.ap-perm-select {
  width: 132px;
  flex-shrink: 0;
}
/* 关于我：说明与输入框纵向排布 */
.ap-about {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ap-about-input {
  width: 100%;
  resize: vertical;
  min-height: 64px;
  font-size: 0.8125rem;
  line-height: 1.55;
}
/* 平台额度开关行（自 AiProviders 迁入）：与供应商卡片刻意区分——它不是供应商 */
.ap-platform {
  margin: 4px 0 14px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-soft);
}
/* 说明文字用满整行剩余宽度：不给 max-width（ch 单位对中文只有半个字宽，会把话提前截在半行处） */
.ap-platform .setting-info {
  flex: 1;
}
</style>
