import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { BUILT_IN_LOOKING_HINT, LOOKING_HINT, builtInRadioProblem, connectingTitle, loadingWords, lookingHint } from './ConnectingCard';

describe('ConnectingCardTest', () => {
  it('namesTheVehicleWhenKnown', () => {
    expect(connectingTitle('guta-test')).toBe('Connecting to guta-test');
  });

  it('fallsBackWithoutAName', () => {
    expect(connectingTitle(null)).toBe('Connecting');
    expect(connectingTitle('')).toBe('Connecting');
  });

  it('aSilentLinkSaysLoadingStoppedInsteadOfPromisingProgress', () => {
    const lost = loadingWords('guta-test', true);
    expect(lost.title).toBe('Signal lost');
    expect(lost.detail.startsWith('Loading stopped.')).toBe(true);
    expect(lost.dismiss).toBe('Hide');
    expect(loadingWords('guta-test', false).title).toBe('Connecting to guta-test');
  });

  it('aBuiltInRadioThatWillNotOpenSaysWhyInsteadOfTheHint', () => {
    const busy = parseJson('{"links":[{"name":"UDP Link (AutoConnect)","lastError":""},{"name":"Built-in radio","lastError":"Another app is using the built-in radio. Close it to connect."}]}');
    const open = parseJson('{"links":[{"name":"Built-in radio","lastError":""}]}');
    expect(lookingHint('radiomaster-ax12', builtInRadioProblem(busy))).toBe('Another app is using the built-in radio. Close it to connect.');
    expect(lookingHint('radiomaster-ax12', builtInRadioProblem(open))).toBe(BUILT_IN_LOOKING_HINT);
    expect(lookingHint(null, null)).toBe(LOOKING_HINT);
  });
});
