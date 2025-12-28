<script setup lang="ts">
import type { NavigationMenuContentEmits, NavigationMenuContentProps } from 'reka-ui';
import type { HTMLAttributes } from 'vue';
import { reactiveOmit } from '@vueuse/core';
import {
  NavigationMenuContent,
  useForwardPropsEmits,
} from 'reka-ui';
import { cn } from '@/lib/utils';

const props = defineProps<NavigationMenuContentProps & { class?: HTMLAttributes['class'] }>();

const emits = defineEmits<NavigationMenuContentEmits>();

const delegatedProps = reactiveOmit(props, 'class');

const forwarded = useForwardPropsEmits(delegatedProps, emits);
</script>

<template>
  <NavigationMenuContent
    v-bind="forwarded"
    :class="cn(
      `
        top-0 left-0 w-full
        data-[motion=from-end]:slide-in-from-right-52
        data-[motion=from-start]:slide-in-from-left-52
        data-[motion=to-end]:slide-out-to-right-52
        data-[motion=to-start]:slide-out-to-left-52
        data-[motion^=from-]:animate-in data-[motion^=from-]:fade-in
        data-[motion^=to-]:animate-out data-[motion^=to-]:fade-out
        md:absolute md:w-auto
      `,
      props.class,
    )"
  >
    <slot></slot>
  </NavigationMenuContent>
</template>
