// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { call } from '@/core/client';
import { createDocument, openDocument } from '@/kotlin/Platform';
import { LocalOpenPlan, OpenPlanDocument } from '@/ui/AppNavigation';
import { DOWNLOAD_UNFINISHED, useRememberPlanFileActions } from './PlanFiles';

vi.mock('@/core/client', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/core/client')>()), call: vi.fn() }));
vi.mock('idb-keyval', () => ({ entries: () => Promise.resolve([]), del: () => Promise.resolve(), set: () => Promise.resolve() }));
vi.mock('@/kotlin/Platform', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/kotlin/Platform')>()), openDocument: vi.fn(), createDocument: vi.fn() }));

const called = vi.mocked(call);
const picker = vi.mocked(openDocument);
const written = vi.mocked(createDocument);

const READY = '{"readiness":{"ready":true},"sync":{"state":"idle"},"filePath":"/saving.plan"}';
const WAITING = '{"readiness":{"ready":false,"reason":"Waiting for terrain heights before the plan can be saved or sent."}}';
const SYNCING = '{"readiness":{"ready":true},"sync":{"state":"busy","downloading":true}}';

const routed = (routes: Readonly<Record<string, unknown>>) =>
  called.mockImplementation((name: string, path?: unknown) => Promise.resolve((routes[`${name} ${String(path)}`] ?? routes[name] ?? '{}') as never));

const actions = () => {
  const said = vi.fn();
  const openPlan = new OpenPlanDocument();
  const wrapper = ({ children }: { children: ReactNode }) => <LocalOpenPlan.Provider value={openPlan}>{children}</LocalOpenPlan.Provider>;
  const view = renderHook(() => useRememberPlanFileActions(said), { wrapper });
  return { said, openPlan, view };
};

const loads = () => called.mock.calls.filter(([name, path]) => name === 'invoke' && path === 'plan.loadFromFile');

describe('plan files', () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    called.mockReset();
    picker.mockReset();
    written.mockReset();
  });

  it('Save and open asks for the file first, inside the click, and a save the plan refuses opens nothing', async () => {
    routed({ 'get view.plan': WAITING });
    picker.mockResolvedValue(new File(['{}'], 'ridge.plan'));
    const { said, view } = actions();
    await act(async () => view.result.current.saveAndOpen());
    expect(picker).toHaveBeenCalledOnce();
    expect(picker.mock.invocationCallOrder[0]).toBeLessThan(called.mock.invocationCallOrder[0] ?? Infinity);
    expect(loads()).toEqual([]);
    expect(said).toHaveBeenLastCalledWith('Waiting for terrain heights before the plan can be saved or sent.');
  });

  it('a cancelled file choice saves and opens nothing', async () => {
    routed({ 'get view.plan': READY });
    picker.mockResolvedValue(null);
    const { said, view } = actions();
    await act(async () => view.result.current.saveAndOpen());
    expect([called.mock.calls.length, said.mock.calls.length]).toEqual([0, 0]);
  });

  it('a download the vehicle refuses says so', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    routed({ 'invoke plan.loadFromVehicle': '{"ok":false}' });
    const { said, view } = actions();
    await act(async () => view.result.current.download());
    expect(said).toHaveBeenLastCalledWith('The plan could not be downloaded from the vehicle.');
  });

  it('a download that never settles says so instead of going quiet', async () => {
    vi.useFakeTimers();
    routed({ 'invoke plan.loadFromVehicle': '{"ok":true}', 'get view.plan': SYNCING });
    const { said, view } = actions();
    await act(async () => view.result.current.download());
    await act(async () => vi.advanceTimersByTimeAsync(61_000));
    expect(said).toHaveBeenLastCalledWith(DOWNLOAD_UNFINISHED);
    expect(view.result.current.opened()).toBe(0);
  });

  it('an empty KML or one that cannot be copied out is reported as failed, not exported', async () => {
    routed({ 'get view.plan': READY, vfsRead: new Uint8Array([]) });
    const { said, view } = actions();
    await act(async () => view.result.current.exportKml());
    await vi.waitFor(() => expect(said).toHaveBeenLastCalledWith('The plan could not be exported.'));
    routed({ 'get view.plan': READY, vfsRead: new Uint8Array([60, 107, 109, 108, 62]) });
    written.mockImplementation(() => {
      throw new Error('blocked');
    });
    await act(async () => view.result.current.exportKml());
    await vi.waitFor(() => expect(said).toHaveBeenLastCalledWith('The KML was written but could not be copied out.'));
  });

  it('a downloaded plan has one name, for its title and for the file Save writes', async () => {
    routed({
      'invoke plan.loadFromVehicle': '{"ok":true}',
      'get view.plan': READY,
      'get view.missionItems': '{"items":[{"kind":"waypoint","command":16},{"kind":"command","command":206}]}',
      'invoke plan.saveToFile': '{"result":true}',
      vfsRead: new Uint8Array([123, 125]),
    });
    const { said, openPlan, view } = actions();
    await act(async () => view.result.current.download());
    await vi.waitFor(() => expect(view.result.current.opened()).toBe(1));
    expect(said).toHaveBeenLastCalledWith(expect.stringContaining('plain waypoints with camera triggers'));
    expect(openPlan.name.value).toBe('Downloaded plan');
    await act(async () => view.result.current.save());
    await act(async () => view.result.current.saveAs());
    expect(written.mock.calls.map(([name]) => name)).toEqual(['Downloaded plan.plan', 'Downloaded plan.plan']);
  });
});
