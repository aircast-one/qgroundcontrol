import { describe, expect, it } from 'vitest';
import { cloudBackoffMs, relayTokenRefusal } from './cloud';

describe('the browser cloud relay', () => {
  it('backs off like the native relay, doubling from 2 s to a 60 s ceiling', () => {
    expect([0, 1, 4, 5, 9].map(cloudBackoffMs)).toEqual([2000, 4000, 32_000, 60_000, 60_000]);
  });

  it('names why the account server refused a relay token', () => {
    expect(relayTokenRefusal(401)).toBe('Sign in to your Aircast account again.');
    expect(relayTokenRefusal(403)).toBe('This account cannot reach that device.');
    expect(relayTokenRefusal(500)).toBe('The Aircast account server answered 500.');
  });
});
