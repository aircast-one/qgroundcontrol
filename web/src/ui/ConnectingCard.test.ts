import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { BROWSER_LOOKING_HINT, BUILT_IN_LOOKING_HINT, LOOKING_HINT, SAVED_LOOKING_HINT, TURN_ON_HINT, builtInRadioProblem, connectingTitle, loadingWords, lookingHint } from './ConnectingCard';

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

  it('the hint names only the links that this build can add', () => {
    expect(lookingHint(null, null, true)).toContain('for Wi-Fi or a network, add a link');
    expect(lookingHint(null, null, false)).toBe(BROWSER_LOOKING_HINT);
    expect(BROWSER_LOOKING_HINT).toContain('network (WebSocket)');
    expect(BROWSER_LOOKING_HINT).not.toContain('by itself');
  });

  it('with links already saved the hint stops sending the pilot to add another', () => {
    expect(lookingHint(null, null, true, true)).toBe(SAVED_LOOKING_HINT);
    expect(lookingHint(null, null, false, true)).toBe(TURN_ON_HINT);
    expect(lookingHint('radiomaster-ax12', null, true, true)).toBe(BUILT_IN_LOOKING_HINT);
  });
});
