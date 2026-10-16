import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { vtolState } from './VtolStateCell';

describe('VtolStateCellTest', () => {
  it('a vtol shows its flight state and offers the opposite transition', () => {
    const forward = vtolState(parseJson('{"vtol":true,"vtolInFwdFlight":true,"flying":true}'))!;
    expect(forward.label).toBe('FW(vtol)');
    expect(forward.transition).toBe('vtolTransitionToMultiRotor');
    expect(vtolState(parseJson('{"vtol":true,"vtolInFwdFlight":false}'))!.label).toBe('MR(vtol)');
    expect(vtolState(parseJson('{"vtol":false}'))).toBeNull();
  });
});
