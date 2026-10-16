import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { resumeCleared, resumeFailedIndex } from './ResumeFailedPrompt';

const json = (text: string): Json => parseJson(text) ?? {};

describe('ResumeFailedPromptTest', () => {
  it('a failed resume upload names the index to retry', () => {
    expect(resumeFailedIndex(json('{"resumeFailedIndex":4}'))).toBe(4);
    expect(resumeFailedIndex(json('{"resumeFailedIndex":null}'))).toBeNull();
    expect(resumeFailedIndex(null)).toBeNull();
  });

  it('a dismissal is forgotten only once the core says nothing failed, not while the view is still loading', () => {
    expect(resumeCleared(null)).toBe(false);
    expect(resumeCleared(json('{"resumeFailedIndex":4}'))).toBe(false);
    expect(resumeCleared(json('{"resumeFailedIndex":null}'))).toBe(true);
  });
});
