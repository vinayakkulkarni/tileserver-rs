import type { Style } from '~/types';
import type { Ref } from 'vue';

const useMapStyles = async () => {
  const data = ref<Style[]>([]);
  try {
    const { data: response } = await useFetch('/styles.json');
    data.value = response.value as Style[];
  } catch (error) {
    console.error('Error fetching styles: ', error);
    data.value = [];
  }
  return { data };
};

export { useMapStyles };
