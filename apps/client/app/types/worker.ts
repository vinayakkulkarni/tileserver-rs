export interface WorkerSuccess<T> {
  ok: true;
  result: T;
}

export interface WorkerFailure {
  ok: false;
  error: string;
}

export type WorkerResponse<T> = WorkerSuccess<T> | WorkerFailure;
