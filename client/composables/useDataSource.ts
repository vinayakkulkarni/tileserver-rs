import type { Data } from '~/types';

const useDataSource = async () => {
  const route = useRoute();
  const data = ref<Data>({} as Data);
  try {
    const { data: response } = await useFetch(
      `/data/${route.params.data}.json`,
    );
    data.value = response.value as Data;
  } catch (error) {
    console.error('Error fetching data: ', error);
    data.value = {} as Data;
  }
  return { data };
};

export { useDataSource };
