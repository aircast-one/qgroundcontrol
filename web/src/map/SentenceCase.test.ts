import { describe, expect, it } from 'vitest';
import { sentenceCase } from './SentenceCase';

describe('sentence case', () => {
  it('keeps proper nouns with punctuation, leading symbols and the first word', () => {
    expect(sentenceCase('Hello, Google, World.')).toBe('Hello, Google, world.');
    expect(sentenceCase('(Optional) Camera Trigger')).toBe('(Optional) camera trigger');
  });

  it('spaces out a run-together word after a slash, and leaves acronyms alone', () => {
    expect(sentenceCase('GPS Lock/ArmingCheck')).toBe('GPS lock/arming check');
    expect(sentenceCase('Herelink AirUnit Setup')).toBe('Herelink AirUnit setup');
  });
});
