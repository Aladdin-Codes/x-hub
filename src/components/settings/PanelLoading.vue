<script setup lang="ts">
// Loading placeholder for lazily loaded settings panels. Show it while switching
// categories to avoid a blank content area. Local chunks usually load quickly.
</script>

<template>
  <div class="panel-loading" aria-hidden="true">
    <span v-for="i in 3" :key="i" class="pl-row">
      <span class="pl-name"></span>
      <span class="pl-ctl"></span>
    </span>
  </div>
</template>

<style scoped>
/* Match the layout of a settings row to avoid a visual jump after loading. */
.panel-loading {
  padding: 4px 0;
}
.pl-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 15px 0;
  border-bottom: 1px solid var(--border-soft);
}
.pl-row:first-child {
  border-top: 1px solid var(--border-soft);
}
.pl-name,
.pl-ctl {
  border-radius: 6px;
  background: var(--bg-card-soft);
}
.pl-name {
  width: 132px;
  height: 12px;
}
.pl-ctl {
  width: 76px;
  height: 22px;
}
</style>
