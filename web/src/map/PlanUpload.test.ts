import { afterEach, describe, expect, it, vi } from 'vitest';
import { PlanBridge } from './PlanBridge';
import { pauseRefusedText, sendPlan } from './PlanUpload';

afterEach(() => vi.restoreAllMocks());

describe('PauseAndUploadTest', () => {
  it('a pause the aircraft refuses stops the upload and says why, so a mission it is still flying is not replaced', async () => {
    const refusal = 'Pause is not available right now.';
    vi.spyOn(PlanBridge, 'pauseRefusal').mockResolvedValue(refusal);
    const sent = vi.spyOn(PlanBridge, 'sendToVehicle').mockResolvedValue(true);
    const said = vi.fn();
    sendPlan(said, () => undefined, true);
    await vi.waitFor(() => expect(said).toHaveBeenCalledWith(pauseRefusedText(refusal)));
    expect(sent).not.toHaveBeenCalled();
  });

  it('a pause the aircraft takes goes on to upload', async () => {
    vi.spyOn(PlanBridge, 'pauseRefusal').mockResolvedValue(null);
    const sent = vi.spyOn(PlanBridge, 'sendToVehicle').mockResolvedValue(true);
    sendPlan(vi.fn(), () => undefined, true);
    await vi.waitFor(() => expect(sent).toHaveBeenCalled());
  });
});
