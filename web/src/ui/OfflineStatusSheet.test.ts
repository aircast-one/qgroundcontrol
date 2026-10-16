import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { offlineStatus } from './OfflineStatusSheet';

describe('OfflineStatusTest', () => {
  it("the offline page reads the core's footnote and links", () => {
    const status = offlineStatus(
      parseJson(`{"title":"Connection Failed","footnote":"Couldn't connect to Drone.","busy":false,"noLinks":false,"editAddress":true,
                   "links":[{"index":2,"text":"Connect to Drone","description":"TCP\\nrefused","retry":true,"failed":true,"connected":false}]}`),
    );
    expect(status?.title).toBe('Connection Failed');
    expect(status?.editAddress).toBe(true);
    expect(status?.links).toEqual([{ index: 2, text: 'Connect to Drone', description: 'TCP\nrefused', retry: true, failed: true, connected: false }]);
    expect(offlineStatus(parseJson('{}'))).toBeNull();
  });
});
