import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { accountLine, accountState, cloudApiBaseValid, cloudDeviceValid } from './AircastCloud';

describe('AircastCloudTest', () => {
  it('fields validate like aircast cloud settings qml', () => {
    expect(cloudApiBaseValid('https://api.aircast.one')).toBe(true);
    expect(cloudApiBaseValid(' http://10.0.0.2:8080 ')).toBe(true);
    expect(cloudApiBaseValid('api.aircast.one')).toBe(false);
    expect(cloudApiBaseValid('https:///x')).toBe(false);
    expect(cloudDeviceValid('  ')).toBe(false);
  });

  it('the account line reads signed in, the status, or not signed in', () => {
    const signedIn = accountState(parseJson('{"kind":"object","signedIn":true,"signingIn":false,"status":"","userCode":"","verificationUrl":""}'))!;
    expect(accountLine(signedIn)).toBe('Signed in');
    expect(accountLine({ ...signedIn, signedIn: false })).toBe('Not signed in');
    expect(accountLine({ ...signedIn, signedIn: false, status: 'Approve code ABC in the browser to sign in.' })).toBe('Approve code ABC in the browser to sign in.');
    expect(accountState(parseJson('{"kind":"null"}'))).toBeNull();
  });
});
