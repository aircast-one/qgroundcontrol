import { startCore } from '@/core/client';
import { restoreKeptFiles } from '@/core/files';
import { GcsLocation } from './GcsLocation';

const state = { started: false };

export const HostPlatform = {
  async start(): Promise<void> {
    if (state.started) return;
    state.started = true;
    await startCore();
    await restoreKeptFiles();
    GcsLocation.start();
  },

  stop(): void {
    GcsLocation.stop();
  },
};
