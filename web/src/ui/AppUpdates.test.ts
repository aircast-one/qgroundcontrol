import { describe, expect, it } from 'vitest';
import { appUpdateReading, sized, updateBannerKind, updateStatus } from './AppUpdates';

const view = (fields: Record<string, unknown>) =>
  appUpdateReading({ current: '5.6.6-beta.1', channel: 'beta', phase: 'idle', latest: '', sizeBytes: 0, progress: 0, error: '', autoCheck: true, skipped: false, later: false, updatedFrom: '', canInstall: true, ...fields });

describe('app updates', () => {
  it('reads nothing until the core knows its own version', () => {
    expect(appUpdateReading(null)).toBeNull();
    expect(appUpdateReading({ current: '' })).toBeNull();
    expect(view({ phase: 'nonsense' })?.phase).toBe('idle');
  });

  it('says where the update is, in the words the native apps use', () => {
    const reading = view({ phase: 'downloading', latest: '5.6.6-beta.2', progress: 0.42 });
    expect(reading && updateStatus(reading)).toBe('Downloading Aircast 5.6.6-beta.2… 42%');
    const failed = view({ phase: 'failed', error: '' });
    expect(failed && updateStatus(failed)).toBe('The update check failed.');
    expect(sized('Update', 104_400_000)).toBe('Update (104 MB)');
    expect(sized('Update', 0)).toBe('Update');
  });

  it('offers an update on the Fly view until it is skipped or put off, and says once that it installed', () => {
    expect(updateBannerKind(view({ phase: 'available' }))).toBe('offer');
    expect(updateBannerKind(view({ phase: 'available', skipped: true }))).toBeNull();
    expect(updateBannerKind(view({ phase: 'ready', later: true }))).toBeNull();
    expect(updateBannerKind(view({ phase: 'ready' }))).toBe('ready');
    expect(updateBannerKind(view({ phase: 'upToDate', updatedFrom: '5.6.5-beta.22' }))).toBe('updated');
    expect(updateBannerKind(view({ phase: 'upToDate' }))).toBeNull();
    expect(updateBannerKind(null)).toBeNull();
  });
});
