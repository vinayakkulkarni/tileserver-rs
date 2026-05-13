import type {
  WorkerFailure,
  WorkerResponse,
  WorkerSuccess,
} from '~/types/worker';

export function runWorker<TArgs extends unknown[], TResult>(
  worker: Worker,
  ...args: TArgs
): Promise<TResult> {
  return new Promise<TResult>((resolve, reject) => {
    worker.addEventListener(
      'message',
      (event: MessageEvent<WorkerResponse<TResult>>) => {
        worker.terminate();
        const data = event.data;
        if (data.ok) resolve(data.result);
        else reject(new Error(data.error));
      },
      { once: true },
    );
    worker.addEventListener(
      'error',
      (event: ErrorEvent) => {
        worker.terminate();
        reject(new Error(event.message || 'Worker crashed'));
      },
      { once: true },
    );
    worker.postMessage(args);
  });
}

export function registerWorkerHandler<TArgs extends unknown[], TResult>(
  handler: (...args: TArgs) => TResult | Promise<TResult>,
): void {
  self.addEventListener('message', async (event: MessageEvent<TArgs>) => {
    try {
      const result = await handler(...event.data);
      const response: WorkerSuccess<TResult> = { ok: true, result };
      (self as DedicatedWorkerGlobalScope).postMessage(response);
    } catch (err) {
      const response: WorkerFailure = {
        ok: false,
        error: err instanceof Error ? err.message : String(err),
      };
      (self as DedicatedWorkerGlobalScope).postMessage(response);
    }
  });
}
