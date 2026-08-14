<script setup lang="ts">
  import { Comment } from 'vue';

  const activeTab = ref(0);
  const slots = useSlots();

  const tabs = computed(() => {
    const defaultSlot = slots.default?.();
    if (!defaultSlot) return [];
    return defaultSlot
      .filter((node) => node.type !== Comment)
      .map((node, index) => ({
        index,
        label:
          (node.props as Record<string, string> | null)?.filename ??
          (node.props as Record<string, string> | null)?.language ??
          `Tab ${index + 1}`,
        node,
      }));
  });

  // Stable per-tab class refs so the inline class array is not recreated on
  // every render inside the v-for subtree. Rebuilt only when the active tab
  // changes.
  const tabClassMap = computed(() =>
    new Map(
      tabs.value.map((tab) => [
        tab.label,
        [
          'px-4 py-2 font-mono text-xs transition-colors',
          activeTab.value === tab.index
            ? 'border-b-2 border-primary text-foreground'
            : 'text-muted-foreground hover:text-foreground',
        ],
      ]),
    ),
  );
</script>

<template>
  <div class="not-prose my-6 border border-border">
    <div
      v-if="tabs.length > 1"
      class="flex border-b border-border bg-muted/30"
    >
      <button
        v-for="tab in tabs"
        :key="tab.label"
        :class="tabClassMap.get(tab.label)"
        @click="activeTab = tab.index"
      >
        {{ tab.label }}
      </button>
    </div>
    <div class="[&_pre]:my-0 [&_pre]:border-0">
      <template
        v-for="(tab, i) in tabs"
        :key="tab.label"
      >
        <div v-show="activeTab === i">
          <component :is="tab.node" />
        </div>
      </template>
    </div>
  </div>
</template>
