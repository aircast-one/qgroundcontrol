import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { LOOKING_TITLE } from './ConnectingCard';
import { NO_CAMERA_GUIDE, offlineGuide, offlineLinkAction, offlineStatus } from './OfflineStatusSheet';

describe('OfflineStatusTest', () => {
  it('a saved link says what tapping it does, so a tap never disconnects without saying so', () => {
    const link = { index: 0, text: 'bridge', description: '', action: 'connect', actionLabel: 'Connect', failed: false, connected: false };
    expect(offlineLinkAction(link)).toBe('Connect');
    expect(offlineLinkAction({ ...link, action: 'retry', actionLabel: 'Retry', failed: true })).toBe('Retry');
    expect(offlineLinkAction({ ...link, connected: true, failed: true })).toBe('Disconnect');
  });

  it("the offline page reads the core's footnote and links", () => {
    const status = offlineStatus(
      parseJson(`{"title":"Connection Failed","footnote":"Couldn't connect to Drone.","busy":false,"noLinks":false,"editAddress":true,
                   "signInUrl":"https://aircast.one/device?code=ABCD",
                   "links":[{"index":2,"text":"Drone","description":"TCP\\nrefused","action":"edit","actionLabel":"Edit","failed":true,"connected":false}]}`),
    );
    expect(status?.title).toBe('Connection Failed');
    expect(status?.editAddress).toBe(true);
    expect(status?.signInUrl).toBe('https://aircast.one/device?code=ABCD');
    expect(status?.links).toEqual([{ index: 2, text: 'Drone', description: 'TCP\nrefused', action: 'edit', actionLabel: 'Edit', failed: true, connected: false }]);
    expect(offlineStatus(parseJson('{}'))).toBeNull();
  });

  it('with no aircraft the Fly screen says what is wrong and what to do next', () => {
    const device = offlineStatus(parseJson('{"title":"No Flight Controller","footnote":"pi4 hears no flight controller. Check it is plugged in and powered on.","onDevice":true}'));
    expect(offlineGuide(device, 'Turn on the aircraft.')).toEqual({ title: 'No flight controller', text: 'pi4 hears no flight controller. Check it is plugged in and powered on.', onDevice: true, camera: false, links: [], busy: false });
    const phone = offlineStatus(parseJson('{"title":"Not Connected","footnote":"Tap a link to connect.","onDevice":false}'));
    expect(offlineGuide(phone, 'Turn on the aircraft.'), 'the chip already says Not connected, so the card does not repeat it').toEqual({ title: LOOKING_TITLE, text: 'Turn on the aircraft.', onDevice: false, camera: false, links: [], busy: false });
    expect(offlineGuide(null, 'Turn on the aircraft.')).toEqual({ title: LOOKING_TITLE, text: 'Turn on the aircraft.', onDevice: false, camera: false, links: [], busy: false });
    expect(offlineGuide(device, 'Turn on the aircraft.', true)).toEqual({ ...NO_CAMERA_GUIDE, onDevice: true, camera: true, links: [], busy: false });
  });

  it('with saved links the Fly screen offers to connect them, not only to add another', () => {
    const saved = offlineStatus(
      parseJson(`{"title":"Not Connected","footnote":"Looking for a vehicle on USB…","busy":true,"noLinks":false,
                   "links":[{"index":0,"text":"Drone","description":"UDP 14550","action":"connect","actionLabel":"Connect","failed":false,"connected":false}]}`),
    );
    const guide = offlineGuide(saved, 'Turn on the aircraft.');
    expect(guide.text).toBe('Turn on the aircraft.');
    expect(guide.links.map((link) => link.text)).toEqual(['Drone']);
  });

  it('once a saved link is tried, the Fly screen says how that went instead of the general hint', () => {
    const failed = offlineStatus(
      parseJson(`{"title":"Connection Failed","footnote":"Couldn't connect to Drone. Check the drone is powered on and on the same network, then tap Retry.","noLinks":false,
                   "links":[{"index":0,"text":"Drone","description":"timeout","action":"retry","actionLabel":"Retry","failed":true,"connected":false}]}`),
    );
    expect(offlineGuide(failed, 'Turn on the aircraft.').text).toBe("Couldn't connect to Drone. Check the drone is powered on and on the same network, then tap Retry.");
  });
});
