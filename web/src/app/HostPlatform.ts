import { startCore } from '@/core/client';
import { GcsLocation } from './GcsLocation';

const state = { started: false };

export const HostPlatform = {
  async start(): Promise<void> {
    if (state.started) return;
    state.started = true;
    await startCore();
    GcsLocation.start();
  },

  stop(): void {
    GcsLocation.stop();
  },
};
