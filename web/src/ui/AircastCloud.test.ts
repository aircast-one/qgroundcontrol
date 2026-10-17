import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { DEFAULT_ACCOUNT_SERVER, accountLine, accountState, cloudApiBaseValid, cloudDeviceValid } from './AircastCloud';
import { LinkDraft, LinkType } from './LinksScreen';

describe('AircastCloudTest', () => {
  it('fields validate like aircast cloud settings qml', () => {
    expect(cloudApiBaseValid('https://api.aircast.one')).toBe(true);
    expect(cloudApiBaseValid(' http://10.0.0.2:8080 ')).toBe(true);
    expect(cloudApiBaseValid('api.aircast.one')).toBe(false);
    expect(cloudApiBaseValid('https:///x')).toBe(false);
    expect(cloudDeviceValid('  ')).toBe(false);
  });

  it('a new cloud link starts on the Aircast account server, so sign-in works without a device setup', () => {
    expect(LinkDraft({ type: LinkType.AircastCloud }).apiBase).toBe(DEFAULT_ACCOUNT_SERVER);
    expect(cloudApiBaseValid(DEFAULT_ACCOUNT_SERVER)).toBe(true);
    expect(LinkDraft({ type: LinkType.AircastCloud, apiBase: 'https://api.dev.aircast.one' }).apiBase).toBe('https://api.dev.aircast.one');
  });

  it('the account line reads signed in, the status, or not signed in', () => {
    const signedIn = accountState(parseJson('{"kind":"object","signedIn":true,"signingIn":false,"status":"","userCode":"","verificationUrl":""}'))!;
    expect(accountLine(signedIn)).toBe('Signed in');
    expect(accountLine({ ...signedIn, signedIn: false })).toBe('Not signed in');
    expect(accountLine({ ...signedIn, signedIn: false, status: 'Approve code ABC in the browser to sign in.' })).toBe('Approve code ABC in the browser to sign in.');
    expect(accountState(parseJson('{"kind":"null"}'))).toBeNull();
  });
});
