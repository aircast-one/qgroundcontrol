import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { LOOKING_TITLE } from './ConnectingCard';
import { offlineGuide, offlineStatus } from './OfflineStatusSheet';

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

  it('with no aircraft the Fly screen says what is wrong and what to do next', () => {
    const device = offlineStatus(parseJson('{"title":"No Flight Controller","footnote":"pi4 hears no flight controller. Check it is plugged in and powered on.","onDevice":true}'));
    expect(offlineGuide(device, 'Turn on the aircraft.')).toEqual({ title: 'No flight controller', text: 'pi4 hears no flight controller. Check it is plugged in and powered on.', onDevice: true });
    const phone = offlineStatus(parseJson('{"title":"Not Connected","footnote":"Tap a link to connect.","onDevice":false}'));
    expect(offlineGuide(phone, 'Turn on the aircraft.')).toEqual({ title: 'Not connected', text: 'Turn on the aircraft.', onDevice: false });
    expect(offlineGuide(null, 'Turn on the aircraft.')).toEqual({ title: LOOKING_TITLE, text: 'Turn on the aircraft.', onDevice: false });
  });
});
