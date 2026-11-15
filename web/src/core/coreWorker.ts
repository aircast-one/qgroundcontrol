export const coreWorker = (): Worker => new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
