import { describe, expect, it } from 'vitest';
import { WifiInfo, phoneSsid, securedFrom } from './PhoneNetwork';

describe('FlasherScreenTest', () => {
  it('the_phones_wifi_name_is_cleaned_of_quotes_and_placeholders', () => {
    expect(phoneSsid('"field-net"')).toBe('field-net');
    expect(phoneSsid('<unknown ssid>')).toBeNull();
    expect(phoneSsid('""')).toBeNull();
    expect(phoneSsid(null)).toBeNull();
  });

  it('only_an_open_network_counts_as_unsecured', () => {
    expect(securedFrom(WifiInfo.SECURITY_TYPE_OPEN)).toBe(false);
    expect(securedFrom(WifiInfo.SECURITY_TYPE_PSK)).toBe(true);
    expect(securedFrom(WifiInfo.SECURITY_TYPE_UNKNOWN)).toBeNull();
  });
});
