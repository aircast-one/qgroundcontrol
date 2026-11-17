import { describe, expect, it } from 'vitest';
import { type AppUpdateState, DISCONNECT_FIRST, channelOf, installBlocker, percentOf, updateBannerKind, updateStatus } from './AppUpdates';

const state = (fields: Partial<AppUpdateState>): AppUpdateState => ({
  current: '5.6.6-beta.1',
  phase: 'idle',
  latest: '',
  downloaded: 0,
  total: 0,
  error: '',
  autoCheck: true,
  skipped: '',
  later: false,
  updatedFrom: '',
  ...fields,
});

describe('app updates', () => {
  it('a prerelease is on the beta channel and a release on stable', () => {
    expect(channelOf('5.6.6-beta.1')).toBe('beta');
    expect(channelOf('5.6.6')).toBe('stable');
  });

  it('says where the update is', () => {
    expect(updateStatus(state({ phase: 'downloading', latest: '5.6.6-beta.2', downloaded: 42, total: 100 }))).toBe('Downloading Aircast 5.6.6-beta.2… 42%');
    expect(updateStatus(state({ phase: 'installing', latest: '5.6.6-beta.2' }))).toBe('Installing Aircast 5.6.6-beta.2…');
    expect(updateStatus(state({ phase: 'failed' }))).toBe('The update failed.');
    expect(percentOf(state({ downloaded: 10, total: 0 }))).toBe(0);
    expect(percentOf(state({ downloaded: 120, total: 100 }))).toBe(100);
  });

  it('offers an update on the Fly view until it is skipped or put off, shows the install, and says once that it installed', () => {
    expect(updateBannerKind(state({ phase: 'available', latest: '5.6.6-beta.2' }))).toBe('offer');
    expect(updateBannerKind(state({ phase: 'available', latest: '5.6.6-beta.2', skipped: '5.6.6-beta.2' }))).toBeNull();
    expect(updateBannerKind(state({ phase: 'available', latest: '5.6.6-beta.3', skipped: '5.6.6-beta.2' }))).toBe('offer');
    expect(updateBannerKind(state({ phase: 'available', latest: '5.6.6-beta.2', later: true }))).toBeNull();
    expect(updateBannerKind(state({ phase: 'downloading', latest: '5.6.6-beta.2' }))).toBe('installing');
    expect(updateBannerKind(state({ phase: 'failed', latest: '5.6.6-beta.2' }))).toBe('failed');
    expect(updateBannerKind(state({ phase: 'failed', error: 'Updater does not have any endpoints set.' }))).toBeNull();
    expect(updateBannerKind(state({ phase: 'upToDate', updatedFrom: '5.6.5-beta.60' }))).toBe('updated');
    expect(updateBannerKind(state({ phase: 'upToDate' }))).toBeNull();
  });

  it('holds the install while closing would cost something, and names what to do first', () => {
    expect(installBlocker({ prompts: [] })).toBeNull();
    expect(installBlocker(null)).toBeNull();
    expect(installBlocker({ prompts: [{ id: 'activeConnections', message: 'There are still active connections to vehicles.', losesWork: false }] })).toBe(DISCONNECT_FIRST);
    expect(installBlocker({ prompts: [{ id: 'unsavedMission', message: 'unsaved', losesWork: true }] })).toBe('Save or send the mission first');
    expect(installBlocker({ prompts: [{ id: 'somethingNew', message: 'Something new is open.' }] })).toBe('Something new is open.');
  });
});
