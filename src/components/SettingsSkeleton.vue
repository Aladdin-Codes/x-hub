<script setup lang="ts">
// Loading skeleton for the asynchronously loaded settings view. It mirrors the
// sidebar and content panel so opening Settings does not show a blank screen.
// Styles are local because SettingsView uses scoped styles.
</script>

<template>
  <div class="settings-view">
    <header class="sv-header">
      <h2 class="sv-title">Settings</h2>
    </header>
    <div class="sv-body">
      <nav class="sv-nav" aria-hidden="true">
        <span v-for="i in 5" :key="i" class="sk-bar"></span>
      </nav>
      <div class="sv-content" aria-hidden="true">
        <div v-for="i in 3" :key="i" class="sk-group">
          <span class="sk-bar sk-bar-title"></span>
          <span class="sk-bar"></span>
          <span class="sk-bar sk-bar-short"></span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  overflow: hidden;
}
.sv-header {
  display: flex;
  align-items: center;
  gap: 12px;
}
.sv-title {
  flex: 1;
  margin: 0;
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--text-1);
}
.sv-body {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: var(--space-4);
  overflow: hidden;
}
.sv-nav {
  flex-shrink: 0;
  width: 148px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.sv-content {
  flex: 1;
  min-width: 0;
  min-height: 0;
  padding: var(--space-4) var(--space-6);
  background: var(--frost-surface);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
}
.sk-bar {
  display: block;
  height: 12px;
  border-radius: 6px;
  background: var(--bg-card-soft);
}
.sv-nav .sk-bar {
  height: 32px;
}
.sk-group {
  margin-bottom: var(--space-5);
}
.sk-group .sk-bar {
  margin-bottom: 10px;
}
.sk-bar-title {
  width: 92px;
  height: 14px;
  margin-bottom: var(--space-4);
}
.sk-bar-short {
  width: 55%;
}
</style>
