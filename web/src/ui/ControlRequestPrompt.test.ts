import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { controlPrompt, secondsLeft } from './ControlRequestPrompt';

const json = (text: string): Json => parseJson(text) ?? {};

describe('ControlRequestPromptTest', () => {
  it("another station's request and the takeover countdown are read from the core", () => {
    const asked = controlPrompt(json('{"inControl":true,"incomingRequest":{"systemId":9,"timeoutMs":20000,"remainingMs":12500},"takeoverRevertMs":null}'));
    expect(asked.incoming).toEqual({ systemId: 9, timeoutMs: 20000, remainingMs: 12500 });
    expect(asked.revertMs).toBeNull();
    expect(secondsLeft(12500)).toBe(13);
    expect(controlPrompt(json('{"inControl":true,"incomingRequest":null,"takeoverRevertMs":4000}')).revertMs).toBe(4000);
    expect(controlPrompt(json('{"inControl":false,"incomingRequest":null,"takeoverRevertMs":4000}')).revertMs, 'the countdown closes once the other station has taken control').toBeNull();
  });
});
