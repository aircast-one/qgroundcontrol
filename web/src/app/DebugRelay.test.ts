import { describe, expect, it } from 'vitest';
import { relayPort } from '@/app/DebugRelay';

describe('relayPort', () => {
  it('takes the debug port from the launch query', () => {
    expect(relayPort('?debug=51234&name=Front')).toBe(51234);
  });

  it('ignores a page opened without one, or with a port no socket can have', () => {
    expect(relayPort('')).toBeNull();
    expect(relayPort('?debug=')).toBeNull();
    expect(relayPort('?debug=0')).toBeNull();
    expect(relayPort('?debug=70000')).toBeNull();
    expect(relayPort('?debug=8800x')).toBeNull();
  });
});
