<script setup lang="ts">
import type { NavigationMenuViewportProps } from 'reka-ui';
import type { HTMLAttributes } from 'vue';
import { reactiveOmit } from '@vueuse/core';
import {
  NavigationMenuViewport,
  useForwardProps,
} from 'reka-ui';
import { cn } from '@/lib/utils';

const props = defineProps<NavigationMenuViewportProps & { class?: HTMLAttributes['class'] }>();

const delegatedProps = reactiveOmit(props, 'class');

const forwardedProps = useForwardProps(delegatedProps);
</script>

<template>
  <div class="absolute top-full left-0 flex justify-center">
    <NavigationMenuViewport
      v-bind="forwardedProps"
      :class="
        cn(
          `
            origin-top-center relative
            left-(--reka-navigation-menu-viewport-left) mt-1.5
            h-(--reka-navigation-menu-viewport-height) w-full overflow-hidden
            rounded-md border bg-popover text-popover-foreground shadow-sm
            data-[state=closed]:animate-out data-[state=closed]:zoom-out-95
            data-[state=open]:animate-in data-[state=open]:zoom-in-90
            md:w-(--reka-navigation-menu-viewport-width)
          `,
          props.class,
        )
      "
    />
  </div>
</template>
