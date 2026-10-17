import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { supportHostCannotBeAsked, supportHostPath, supportHostVerdict } from './RemoteSupportScreen';

const served = (valid: boolean, error: string) => parseJson(`{"kind":"object","class":"SupportHost","valid":${valid},"error":"${error}"}`);

describe('RemoteSupportScreenTest', () => {
  it("the head reports the core's sentence, not one of its own", () => {
    const refused = supportHostVerdict(served(false, 'The part after the colon has to be a port number between 1 and 65535.'));
    expect(refused?.valid).toBe(false);
    expect(refused?.error).toBe('The part after the colon has to be a port number between 1 and 65535.');
  });

  it('an accepted host carries no complaint', () => {
    const ok = supportHostVerdict(served(true, ''));
    expect(ok?.valid).toBe(true);
    expect(ok?.error).toBe('');
  });

  it('a view of another shape is not a verdict', () => {
    expect(supportHostVerdict(null)).toBeNull();
    expect(supportHostVerdict(parseJson('{"kind":"object","class":"Links"}'))).toBeNull();
  });

  it('the query names the host it is asking about', () => {
    expect(supportHostPath('10.0.0.4:14550')).toBe('view.supportHost(10.0.0.4:14550)');
  });

  it('the core answers the comma now, so the head does not', () => {
    expect(supportHostCannotBeAsked('10.0.0.4,14550')).toBeNull();
    expect(supportHostCannotBeAsked('two words:14550')).toBeNull();
  });

  it('surrounding space stays refused here, because the core trims and QGC does not', () => {
    expect(supportHostCannotBeAsked('  leading:14550')).toBe('Remove the space before or after the address.');
    expect(supportHostCannotBeAsked('host:14550 ')).toBe('Remove the space before or after the address.');
    expect(supportHostCannotBeAsked('host:14550')).toBeNull();
    expect(supportHostCannotBeAsked('')).toBeNull();
  });
});
