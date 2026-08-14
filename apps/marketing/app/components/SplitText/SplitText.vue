<script setup lang="ts">
  import { useIntersectionObserver } from '@vueuse/core';
  import { motion } from 'motion-v';
  import { cn } from '~/lib/utils';

  const props = withDefaults(
    defineProps<{
      text: string;
      by?: 'chars' | 'words';
      delay?: number;
      duration?: number;
      from?: Record<string, unknown>;
      to?: Record<string, unknown>;
      threshold?: number;
      class?: string;
    }>(),
    {
      by: 'chars',
      delay: 50,
      duration: 0.6,
      from: () => ({ opacity: 0, y: 40 }),
      to: () => ({ opacity: 1, y: 0 }),
      threshold: 0.1,
      class: '',
    },
  );

  const el = ref<HTMLElement>();
  const isInView = ref(false);

  useIntersectionObserver(
    el,
    ([entry]) => {
      if (entry?.isIntersecting) {
        isInView.value = true;
      }
    },
    { threshold: props.threshold },
  );

  // Split-text tokens have no intrinsic identity, so each word/char is keyed
  // by its position-derived id (stable for a given props.text) and carries its
  // own precomputed transition, avoiding inline object literals in the v-for
  // subtree and index keys that defeat DOM reuse.
  const words = computed(() => {
    const tokens = props.text.split(' ');
    const result: Array<{
      id: string;
      needsSpace: boolean;
      characters: Array<{
        id: string;
        char: string;
        transition: {
          duration: number;
          delay: number;
          type: 'spring';
          damping: number;
          stiffness: number;
        };
      }>;
    }> = [];
    let globalIndex = 0;
    for (let i = 0; i < tokens.length; i += 1) {
      const token = tokens[i];
      const chars = props.by === 'words' ? [token] : token.split('');
      result.push({
        id: `word-${i}`,
        needsSpace: i < tokens.length - 1,
        characters: chars.map((char, ci) => ({
          id: `char-${globalIndex + ci}`,
          char,
          transition: {
            duration: props.duration,
            delay: getDelay(globalIndex + ci),
            type: 'spring' as const,
            damping: 25,
            stiffness: 300,
          },
        })),
      });
      globalIndex += chars.length;
    }
    return result;
  });

  function getDelay(globalIndex: number): number {
    return (globalIndex * props.delay) / 1000;
  }
</script>

<template>
  <p ref="el" :class="cn('flex flex-wrap whitespace-pre-wrap', props.class)">
    <span v-for="word in words" :key="word.id" class="inline-flex">
      <component
        :is="motion.span"
        v-for="char in word.characters"
        :key="char.id"
        class="inline-block"
        :initial="from"
        :animate="isInView ? to : from"
        :transition="char.transition"
        style="will-change: transform, opacity"
      >
        {{ char.char }}
      </component>
      <span v-if="word.needsSpace" class="whitespace-pre">&nbsp;</span>
    </span>
  </p>
</template>
