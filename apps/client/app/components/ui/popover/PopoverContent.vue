<script setup lang="ts">
  import type { PopoverContentEmits, PopoverContentProps } from 'reka-ui';
  import type { HTMLAttributes } from 'vue';
  import { reactiveOmit } from '@vueuse/core';
  import { PopoverContent, PopoverPortal, useForwardPropsEmits } from 'reka-ui';
  import { cn } from '@/lib/utils';

  defineOptions({ inheritAttrs: false });

  const props = withDefaults(
    defineProps<PopoverContentProps & { class?: HTMLAttributes['class'] }>(),
    { align: 'start', sideOffset: 4 },
  );
  const emits = defineEmits<PopoverContentEmits>();
  const delegatedProps = reactiveOmit(props, 'class');
  const forwarded = useForwardPropsEmits(delegatedProps, emits);
</script>

<template>
  <PopoverPortal>
    <PopoverContent
      v-bind="{ ...forwarded, ...$attrs }"
      :class="
        cn(
          'z-50 w-72 border border-border bg-popover p-4 text-popover-foreground shadow-lg outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0',
          props.class,
        )
      "
    >
      <slot></slot>
    </PopoverContent>
  </PopoverPortal>
</template>
