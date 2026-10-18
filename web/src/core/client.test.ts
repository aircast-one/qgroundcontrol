import { describe, expect, it } from 'vitest';
import { onPort } from './client';

describe('onPort', () => {
  it('lets a port finish closing before it is opened again, even after a failure', async () => {
    const port = {} as SerialPort;
    const steps: string[] = [];
    const slowClose = onPort(port, async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
      steps.push('closed');
      throw new Error('close failed');
    });
    const reopen = onPort(port, async () => {
      steps.push('opened');
    });
    await expect(slowClose).rejects.toThrow('close failed');
    await reopen;
    expect(steps).toEqual(['closed', 'opened']);
  });
});
