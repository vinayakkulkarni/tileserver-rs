import type { Data } from '~/types';

const useDataSources = async () => {
  const data = ref<Data[]>([]);
  try {
    const { data: response } = await useFetch('/data.json');
    data.value = response.value as Data[];
  } catch (error) {
    console.error('Error fetching data: ', error);
    data.value = [];
  }
  return { data };
};

export { useDataSources };
