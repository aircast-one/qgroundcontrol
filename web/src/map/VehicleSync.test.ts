import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { VehicleSync, VehicleSyncEntries, nextNotReady, notReadyToSend, planSupport, syncRefusal, uploadGate, uploadLabel, uploadStep, vehicleSyncState } from './VehicleSync';

describe('VehicleSyncTest', () => {
  it('a connected idle vehicle is ready', () => {
    expect(vehicleSyncState(false, false)).toBe(VehicleSync.Ready);
    expect(syncRefusal(VehicleSync.Ready, 'send to')).toBeNull();
  });

  it('no vehicle is refused rather than reported as sent', () => {
    expect(vehicleSyncState(true, false)).toBe(VehicleSync.Offline);
    expect(syncRefusal(VehicleSync.Offline, 'send to')).toBe('No vehicle to send to');
  });

  it('a sync already running is refused', () => {
    expect(vehicleSyncState(false, true)).toBe(VehicleSync.Busy);
    expect(syncRefusal(VehicleSync.Busy, 'send to')).toBe('Already syncing, wait for it to finish');
  });

  it('being offline outranks being busy', () => {
    expect(vehicleSyncState(true, true)).toBe(VehicleSync.Offline);
  });

  it('the refusal names the action it refused', () => {
    expect(syncRefusal(VehicleSync.Offline, 'load from')).toBe('No vehicle to load from');
  });
});

describe('UploadGateTest', () => {
  const view = (upload: Json) => ({ upload });

  it('a clean plan uploads without asking', () => {
    expect(uploadStep(uploadGate(view({ canSend: true, canProceed: false })))).toEqual({ kind: 'Send' });
  });

  it('a vehicle flying this mission is paused first, not silently overwritten', () => {
    const gate = uploadGate(view({ canSend: false, canProceed: true, pausesFirst: true, heading: 'Upload this plan?', proceedTitle: 'Pause and upload', refusal: 'The vehicle is flying this mission.' }));
    const step = uploadStep(gate);
    expect(step.kind).toBe('Confirm');
    if (step.kind !== 'Confirm') return;
    expect(step.gate.pausesFirst).toBe(true);
    expect(step.gate.proceedTitle).toBe('Pause and upload');
  });

  it('a firmware mismatch warns and offers to go ahead', () => {
    const step = uploadStep(uploadGate(view({ canSend: false, canProceed: true, pausesFirst: false, proceedTitle: 'Upload anyway' })));
    expect(step.kind).toBe('Confirm');
    if (step.kind !== 'Confirm') return;
    expect(step.gate.pausesFirst).toBe(false);
  });

  it("no vehicle is a refusal with the core's words", () => {
    expect(uploadStep(uploadGate(view({ canSend: false, canProceed: false, refusal: 'No vehicle is connected.' })))).toEqual({ kind: 'Refuse', reason: 'No vehicle is connected.' });
  });

  it('a view with no upload block refuses rather than sending', () => {
    expect(uploadStep(null).kind).toBe('Refuse');
    expect(uploadStep(uploadGate({})).kind).toBe('Refuse');
  });

  it('upload label follows PlanToolBarIndicators', () => {
    expect(uploadLabel(false, true, false, true)).toBe('Uploading…');
    expect(uploadLabel(false, false, false, true)).toBe('Uploaded');
    expect(uploadLabel(false, false, true, true)).toBe('Upload');
    expect(uploadLabel(true, false, false, true)).toBe('Upload');
    expect(uploadLabel(false, false, false, false)).toBe('Upload');
  });
});

describe('UploadReadinessTest', () => {
  const view = (ready: boolean, reason: string, canSend = true): Json => ({
    readiness: { ready, reason },
    upload: { canSend, canProceed: false, pausesFirst: false, heading: '', refusal: '', proceedTitle: '' },
  });

  it('a plan with an item still being drawn is not sent', () => {
    const v = view(false, 'An item is still being drawn, so the plan cannot be saved or sent.');
    expect(uploadStep(uploadGate(v), notReadyToSend(v))).toEqual({ kind: 'Refuse', reason: 'An item is still being drawn, so the plan cannot be saved or sent.' });
  });

  it('readiness is checked before the vehicle precheck, because it is about the plan', () => {
    const v = view(false, 'Waiting for terrain heights before the plan can be saved or sent.', true);
    expect(uploadStep(uploadGate(v), notReadyToSend(v)).kind).toBe('Refuse');
  });

  it('a ready plan is sent as before', () => {
    const v = view(true, '');
    expect(notReadyToSend(v)).toBeNull();
    expect(uploadStep(uploadGate(v), notReadyToSend(v)).kind).toBe('Send');
  });

  it('a view with no readiness at all does not block the upload', () => {
    expect(notReadyToSend({ upload: { canSend: true } })).toBeNull();
    expect(notReadyToSend(null)).toBeNull();
  });

  it('an unready plan with no reason still says something', () => {
    expect(notReadyToSend(view(false, ''))).toBe('The plan is not ready to send.');
  });

  it('a refused upload goes to the first item that still needs something', () => {
    expect(nextNotReady({ readiness: { ready: false, next: 3 } })).toBe(3);
    expect(nextNotReady({ readiness: { ready: false, next: null } })).toBeNull();
    expect(nextNotReady({ readiness: { ready: true, next: 3 } })).toBeNull();
  });
});

describe('PlanSupportTest', () => {
  it('a vehicle that takes both offers both', () => {
    const both = planSupport({ kind: 'object', actions: { addFence: true, addRally: true }, fenceSupported: true, rallySupported: true });
    expect(both.fence).toBe(true);
    expect(both.rally).toBe(true);
    expect(both.fenceRefused).toBe(false);
  });

  it('a vehicle that takes neither says so instead of offering buttons that fail', () => {
    const neither = planSupport({ kind: 'object', actions: { addFence: false, addRally: false }, fenceSupported: false, rallySupported: false });
    expect(neither.fence).toBe(false);
    expect(neither.rally).toBe(false);
    expect(neither.fenceRefused).toBe(true);
    expect(neither.rallyRefused).toBe(true);
  });

  it('the two are answered separately, because a vehicle can take one and not the other', () => {
    const fenceOnly = planSupport({ kind: 'object', actions: { addFence: true, addRally: false } });
    expect(fenceOnly.fence).toBe(true);
    expect(fenceOnly.rally).toBe(false);
  });

  it('no answer hides the buttons rather than offering an action the vehicle will refuse', () => {
    expect(planSupport(null).fence).toBe(false);
    expect(planSupport(null).rally).toBe(false);
    expect(planSupport({ kind: 'null' }).fence).toBe(false);
  });

  it('only an explicit refusal hides the empty-layer hint, like RallyPointEditor and GeoFenceEditor', () => {
    const rallyOnlyRefused = planSupport({ kind: 'object', actions: { addFence: true, addRally: false }, fenceSupported: true, rallySupported: false });
    const unread = planSupport({ kind: 'object', actions: { addFence: false, addRally: false } });
    expect(rallyOnlyRefused.rallyRefused).toBe(true);
    expect(rallyOnlyRefused.fenceRefused).toBe(false);
    expect(unread.rallyRefused).toBe(false);
    expect(unread.fenceRefused).toBe(false);
  });
});

describe('PlanSupportGateTest', () => {
  const plan = (addFence: boolean, addRally: boolean): Json => ({ actions: { addFence, addRally } });

  it('a firmware without fences refuses both fence shapes, not just the polygon', () => {
    const support = planSupport(plan(false, true));
    expect(support.fence).toBe(false);
    expect(support.rally).toBe(true);
  });

  it('absent actions refuse rather than assume', () => {
    const nothing = planSupport({});
    expect(nothing.fence).toBe(false);
    expect(nothing.rally).toBe(false);
  });
});

describe('UploadEmphasisTest', () => {
  const emphasised = (offline: boolean, syncing: boolean) => syncRefusal(vehicleSyncState(offline, syncing), 'upload to') === null;

  it('with a vehicle ready to take it, Upload is the primary action', () => {
    expect(emphasised(false, false)).toBe(true);
    expect(syncRefusal(VehicleSync.Ready, 'upload to')).toBeNull();
  });

  it('with no vehicle it is not advertised as the thing to do', () => {
    expect(emphasised(true, false)).toBe(false);
  });

  it('mid-sync it is not advertised either', () => {
    expect(emphasised(false, true)).toBe(false);
  });

  it('it stays tappable in every state, because the refusal is the explanation', () => {
    expect(VehicleSyncEntries.filter((state) => {
      const refusal = syncRefusal(state, 'upload to');
      return refusal !== null && refusal.trim() === '';
    })).toEqual([]);
  });
});
