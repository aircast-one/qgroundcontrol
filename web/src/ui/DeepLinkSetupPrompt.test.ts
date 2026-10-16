import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { deepLinkSetup } from './DeepLinkSetupPrompt';

describe('DeepLinkSetupPromptTest', () => {
  it('a device link is shown only while the core holds one', () => {
    expect(deepLinkSetup(parseJson('{"show":false,"host":null}'))).toBeNull();
    expect(deepLinkSetup(null)).toBeNull();
    const shown = deepLinkSetup(parseJson('{"show":true,"host":"10.0.0.5","title":"Set up from this device?","text":"from 10.0.0.5","accept":"Set up","decline":"Ignore"}'));
    expect(shown).toEqual({ host: '10.0.0.5', title: 'Set up from this device?', text: 'from 10.0.0.5', accept: 'Set up', decline: 'Ignore' });
  });
});
