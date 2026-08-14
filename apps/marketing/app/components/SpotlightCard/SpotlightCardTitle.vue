<script setup lang="ts">
  import { cn } from '~/lib/utils';

  const props = withDefaults(defineProps<{ class?: string }>(), { class: '' });
</script>

<template>
  <h3
    :class="
      cn(
        `
          text-xl leading-none font-semibold tracking-tight text-foreground
        `,
        props.class,
      )
    "
  >
    <slot></slot>
  </h3>
</template>
