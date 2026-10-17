import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { applyAltitudePrompt } from './PlanTab';

describe('ApplyAltitudePromptTest', () => {
  it('the prompt shows only while the core asks', () => {
    expect(applyAltitudePrompt(parseJson('{"applyAltitudePrompt":{"title":"Apply new altitude","text":"t"}}'))).toEqual({ title: 'Apply new altitude', text: 't' });
    expect(applyAltitudePrompt(parseJson('{"applyAltitudePrompt":null}'))).toBeNull();
  });
});
