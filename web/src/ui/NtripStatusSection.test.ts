import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { dataRate, dataSize, ntripBrowser, ntripStatus } from './NtripStatusSection';

describe('NtripStatusSectionTest', () => {
  it('counts_and_sizes_read_like_qgc', () => {
    expect(ntripStatus(parseJson('{}'))).toBeNull();
    const status = ntripStatus(parseJson('{"status":"connected","statusMessage":"Connected","messageTypes":[[1005,3],[0,1]],"bytesReceived":2048}'))!;
    expect(status.messageTypes).toEqual([
      [1005, 3],
      [0, 1],
    ]);
    expect(dataSize(status.bytesReceived)).toBe('2.0 KB');
    expect(dataSize(512)).toBe('512 B');
    expect(dataSize(1572864)).toBe('1.5 MB');
    expect(dataRate(900.0)).toBe('900 B/s');
    expect(dataRate(2048.0)).toBe('2.0 KB/s');
  });

  it('the_browser_lists_mountpoints_and_marks_the_selected_one', () => {
    const browser = ntripBrowser(parseJson('{"status":"success","error":"","canBrowse":true,"mountpoints":[{"mountpoint":"NEAR","detail":"RTCM 3.3 · GPS","selected":true}]}'));
    expect(browser.mountpoints).toEqual([{ mountpoint: 'NEAR', detail: 'RTCM 3.3 · GPS', selected: true }]);
    expect(ntripBrowser(null).status).toBe('');
  });
});
