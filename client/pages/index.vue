<template>
  <section
    class="min-h-screen bg-no-repeat bg-contain bg-gradient-26 overflow-auto pb-24"
  >
    <div class="flex flex-col items-center justify-start space-y-20">
      <div class="flex flex-col mt-32 items-center space-y-10">
        <h1 class="font-light text-5xl">Tileserver RS</h1>
        <p class="font-light text-3xl">Vector maps with GL styles</p>
      </div>
      <!-- Styles -->
      <div class="bg-white divide-y max-w-screen-lg w-full rounded shadow">
        <h1 class="px-8 py-4 font-bold text-xl uppercase">Styles</h1>
        <div
          class="p-8"
          :class="{ 'bg-slate-50': idx % 2 === 0 }"
          v-for="(style, idx) in styles"
          :key="idx"
        >
          <div class="flex items-center justify-between">
            <section
              id="details"
              class="flex justify-between items-center space-x-6"
            >
              <img
                :src="`/styles/${style.id}/0/0/0.png`"
                :alt="`${style.name} preview`"
                class="w-32 h-32 object-cover rounded border shadow"
              />
              <div class="space-y-2">
                <h3 class="font-bold text-lg">{{ style.name }}</h3>
                <p class="text-sm font-light">identifier: {{ style.id }}</p>
                <p class="text-sm divide-x">
                  services:
                  <button
                    type="button"
                    aria-label="GL Style"
                    class="text-blue-600 hover:underline pr-1"
                    @click="goto('gl-style', style)"
                  >
                    GL Style
                  </button>
                  <button
                    type="button"
                    aria-label="TileJSON"
                    class="text-blue-600 hover:underline px-1"
                    @click="goto('tilejson', style)"
                  >
                    TileJSON
                  </button>
                  <button
                    type="button"
                    aria-label="WMTS"
                    class="text-blue-600 hover:underline pl-1"
                    @click="goto('wmts', style)"
                  >
                    WMTS
                  </button>
                  <!-- <a class="text-blue-600 hover:underline pl-2"> XYZ </a>
                  <input
                    :id="`xyz_style_${style.id}`"
                    type="text"
                    :value="/styles/${style.id}/{z}/{x}/{y}.webp"
                    style="display: none"
                  /> -->
                </p>
              </div>
            </section>
            <section
              id="viewers"
              class="flex flex-col items-center justify-center space-y-2"
            >
              <a
                :href="`/styles/${style.id}/#2/0.00000/0.00000`"
                class="inline-flex items-center rounded-md border border-transparent bg-blue-600 px-4 py-2 text-base font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
              >
                Viewer
              </a>
            </section>
          </div>
        </div>
      </div>
      <!-- Data -->
      <div class="bg-white divide-y max-w-screen-lg w-full rounded shadow">
        <h1 class="px-8 py-4 font-bold text-xl uppercase">Data</h1>
        <div
          class="p-8"
          :class="{ 'bg-slate-50': i % 2 === 0 }"
          v-for="(d, i) in data"
          :key="i"
        >
          <div class="flex items-center justify-between">
            <section
              id="details"
              class="flex justify-between items-center space-x-6"
            >
              <img
                :src="`/images/placeholder.png`"
                :alt="`${d.name} preview`"
                class="w-32 h-32 object-cover rounded border shadow"
              />
              <div class="space-y-2">
                <h3 class="font-bold text-lg">{{ d.basename }}</h3>
                <p class="text-sm font-light">
                  identifier: {{ d.id }} | type: vector data
                </p>
                <p class="text-sm font-light">
                  services:
                  <a
                    :href="`/styles/${d.id}.json`"
                    class="text-blue-600 hover:underline"
                  >
                    TileJSON
                  </a>
                </p>
              </div>
            </section>
            <section
              id="viewers"
              class="flex flex-col items-center justify-center space-y-2"
            >
              <a
                :href="`/data/${d.id}/#2/0.00000/0.00000`"
                class="inline-flex items-center rounded-md border border-transparent bg-blue-600 px-4 py-2 text-base font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
              >
                Inspect
              </a>
            </section>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<script lang="ts">
  import type { Style } from '~/types';

  export default defineComponent({
    name: 'HomePage',
    async setup() {
      const router = useRouter();
      const { data: mapDatasets } = await useDataSources();
      const { data: mapStyles } = await useMapStyles();
      const goto = (type: string, style: Style) => {
        if (type === 'gl-style') {
          router.push({
            path: `/styles/${style.id}/style.json`,
            params: { raster: '' },
          });
        }
        if (type === 'tilejson') {
          router.push({
            path: `/styles/${style.id}.json`,
            params: { raster: '' },
          });
        }
        if (type === 'wmts') {
          router.push({
            path: `/styles/${style.id}/wmts.xml`,
          });
        }
      };
      return {
        styles: mapStyles,
        data: mapDatasets,
        goto,
      };
    },
  });
</script>
