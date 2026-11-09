<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import { ExternalLink, RefreshCw } from 'lucide-vue-next'
import { isTauri, tauriApi } from '../api/tauri'
import { useStore } from '../stores/workbench'

const store = useStore()
const showToast = inject<(msg: string) => void>('showToast', () => {})

const version = ref('')
const loading = ref(true)

// 版本历史的唯一线上出处：GitHub Releases（客户端内不再内置历史列表）
const RELEASES_URL = 'https://github.com/dckxx/x-hub/releases'

function openReleases() {
  if (isTauri()) {
    void tauriApi.openExternal(RELEASES_URL).catch(() => {
      showToast('Failed to open the browser')
    })
  } else {
    window.open(RELEASES_URL, '_blank', 'noopener')
  }
}

// ---- 应用更新：仅保留「检查更新」按钮；发现新版本由全局弹窗（UpdateCheckDialog）接管 ----
const checking = ref(false)

async function onCheckUpdate() {
  if (!isTauri()) {
    showToast('Updates are only available in the desktop app')
    return
  }
  if (checking.value) return
  checking.value = true
  try {
    // manual=true：手动检查忽略「跳过此版本」记录，用户主动查看能再次取到该版本
    const info = await tauriApi.checkForUpdate(true)
    if (info.available) {
      // 后端已广播 update-available → 全局弹窗自动弹出并展示版本/说明
      if (info.ready) showToast(`Version v${info.version} is ready. Click “Restart now” in the dialog`)
    } else {
      showToast(`You're up to date (v${info.current})`)
    }
  } catch (e) {
    showToast(`Failed to check for updates: ${String(e)}`)
  } finally {
    checking.value = false
  }
}

onMounted(async () => {
  try {
    const info = await tauriApi.getAppInfo()
    version.value = info.version
  } catch {
    version.value = ''
  } finally {
    loading.value = false
  }
})

function onToggleAutoUpdate() {
  void store.setAutoUpdateEnabled(!store.state.config.auto_update_enabled)
}
</script>

<template>
  <div class="about-section">
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Current version</span>
        <span class="setting-desc">The version number is maintained in README and synced with the app build</span>
      </div>
      <div class="about-version-wrap">
        <span class="about-version">{{ loading ? '…' : `v${version}` }}</span>
        <button class="ghost-btn upd-check-btn" type="button" :disabled="checking" @click="onCheckUpdate">
          <RefreshCw
            :size="13"
            :stroke-width="2"
            class="upd-check-icon"
            :class="{ spinning: checking }"
          />
          {{ checking ? 'Checking…' : 'Check for updates' }}
        </button>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Automatically check for updates</span>
        <span class="setting-desc">The app silently checks for updates periodically after startup and notifies you in a dialog when one is available. Downloads start only after you click “Update now”; downloaded updates are installed automatically the next time you start the app. When disabled, automatic checks and reminders stop. The “Check for updates” button and actions in the update dialog are not affected.</span>
      </div>
      <button
        class="toggle"
        role="switch"
        type="button"
        :aria-checked="store.state.config.auto_update_enabled"
        :class="{ on: store.state.config.auto_update_enabled }"
        @click="onToggleAutoUpdate"
      >
        <span class="toggle-knob"></span>
      </button>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Open-source license</span>
        <span class="setting-desc">This project is open source under the MIT License</span>
      </div>
      <a
        class="ghost-btn about-license"
        href="https://github.com/dckxx/x-hub"
        target="_blank"
        rel="noopener noreferrer"
      >
        MIT License
      </a>
    </div>

    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-name">Version history</span>
        <span class="setting-desc">Release notes for each version are hosted on GitHub Releases. Click to view them.</span>
      </div>
      <button class="ghost-btn about-releases" type="button" @click="openReleases">
        <ExternalLink :size="13" :stroke-width="2" />
        GitHub Releases
      </button>
    </div>
  </div>
</template>

<style scoped>
/* 与 SettingsView 保持一致的设置行布局（scoped 隔离，需在此自绘） */
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 0;
}
.setting-row + .setting-row {
  border-top: 1px solid var(--border-soft);
}
.setting-info {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.setting-name {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-1);
}
.setting-desc {
  flex-basis: 100%;
  font-size: 0.75rem;
  color: var(--text-3);
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

.about-version {
  font-size: 0.875rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--brand-500);
}

/* 应用更新区块 */
.about-version-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
}
.ghost-btn.upd-check-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  padding: 4px 10px;
}
.upd-check-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.upd-check-icon {
  color: var(--text-3);
}
.upd-check-icon.spinning {
  animation: upd-spin 1s linear infinite;
  color: var(--brand-500);
}
@keyframes upd-spin {
  to {
    transform: rotate(360deg);
  }
}

.about-license {
  text-decoration: none;
}

.about-releases {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  text-decoration: none;
}
</style>
