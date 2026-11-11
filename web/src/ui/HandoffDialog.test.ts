import { describe, expect, it } from 'vitest';
import { handoffText } from './HandoffDialog';

describe('asking before connecting to a device a link names', () => {
  it('names the address the link points at and warns to connect only from your own device', () => {
    expect(handoffText('https://100-64-0-15.falcon-gb3v.d.aircast.one')).toBe(
      'A link asks this app to connect to the Aircast device at 100-64-0-15.falcon-gb3v.d.aircast.one. Connect only if you opened it from your own device.',
    );
  });
});
