<script setup lang="ts">
  import { Info, AlertTriangle, Lightbulb, CheckCircle } from '@lucide/vue';
  import { computed } from 'vue';

  const props = withDefaults(
    defineProps<{
      type?: 'info' | 'warning' | 'tip' | 'success';
    }>(),
    { type: 'info' },
  );

  const config = computed(() => {
    switch (props.type) {
      case 'warning':
        return {
          icon: AlertTriangle,
          border: 'border-yellow-500/30',
          bg: 'bg-yellow-500/5',
          label: 'Warning',
          labelColor: 'text-yellow-500',
        };
      case 'tip':
        return {
          icon: Lightbulb,
          border: 'border-green-500/30',
          bg: 'bg-green-500/5',
          label: 'Tip',
          labelColor: 'text-green-500',
        };
      case 'success':
        return {
          icon: CheckCircle,
          border: 'border-emerald-500/30',
          bg: 'bg-emerald-500/5',
          label: 'Success',
          labelColor: 'text-emerald-500',
        };
      default:
        return {
          icon: Info,
          border: 'border-primary/30',
          bg: 'bg-primary/5',
          label: 'Info',
          labelColor: 'text-primary',
        };
    }
  });
</script>

<template>
  <div :class="['my-6 border-l-2 p-4', config.border, config.bg]">
    <div class="mb-1 flex items-center gap-2">
      <component
        :is="config.icon"
        :class="['size-4', config.labelColor]"
      />
      <span
        :class="[
          'font-mono text-xs font-medium uppercase tracking-wider',
          config.labelColor,
        ]"
      >
        {{ config.label }}
      </span>
    </div>
    <div class="text-sm leading-relaxed text-muted-foreground [&_p]:mb-0">
      <slot />
    </div>
  </div>
</template>
