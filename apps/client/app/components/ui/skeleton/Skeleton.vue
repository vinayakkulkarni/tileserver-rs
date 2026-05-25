<script setup lang="ts">
  import { cn } from '~/lib/utils';

  const props = defineProps<{ class?: string }>();
</script>

<template>
  <div :class="cn('skel', props.class)" aria-hidden="true"></div>
</template>
