import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { canAddKey, isHexKey, keyButtons, signingKeys } from './SigningKeysSection';

describe('SigningKeysSectionTest', () => {
  it('a_key_needs_a_name_and_a_long_enough_passphrase_or_64_hex', () => {
    expect(canAddKey('', false, 'longenough', 8)).toBe(false);
    expect(canAddKey('field', false, 'short', 8)).toBe(false);
    expect(canAddKey('field', false, 'longenough', 8)).toBe(true);
    expect(canAddKey('field', true, 'ab'.repeat(32), 8)).toBe(true);
    expect(canAddKey('field', true, 'ab'.repeat(31), 8)).toBe(false);
    expect(isHexKey('zz'.repeat(32))).toBe(false);
  });

  it('the_view_lists_keys_only_when_the_core_owns_them', () => {
    expect(signingKeys(parseJson('{"available":false}'))).toBeNull();
    const keys = signingKeys(parseJson('{"available":true,"vehicle":true,"activeKey":"None","minPassphraseLength":8,"keys":[{"name":"field","inUse":false}]}'))!;
    expect(keys.keys).toEqual([{ name: 'field', inUse: false, activeOnVehicle: false }]);
  });

  it('only_the_active_key_offers_disable_and_others_say_another_key_is_active', () => {
    const keys = signingKeys(
      parseJson('{"available":true,"vehicle":true,"armed":false,"state":"on","linkName":"USB","activeKey":"field","keys":[{"name":"field","inUse":true,"activeOnVehicle":true},{"name":"bench","inUse":false,"activeOnVehicle":false}]}'),
    )!;
    expect(keyButtons(keys, keys.keys[0]!)).toEqual({ enable: false, disable: true, otherActive: false, pending: false });
    expect(keyButtons(keys, keys.keys[1]!)).toEqual({ enable: false, disable: false, otherActive: true, pending: false });
    const off = { ...keys, activeKey: 'None', state: 'off' };
    expect(keyButtons(off, off.keys[1]!).enable).toBe(true);
  });
});
