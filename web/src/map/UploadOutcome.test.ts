import { describe, expect, it } from 'vitest';
import { UploadOutcome, uploadMessage, uploadOutcome } from './UploadOutcome';

describe('UploadOutcomeTest', () => {
  it('an upload that started says nothing while it runs, leaving the progress to the upload button', () => {
    expect(uploadOutcome(true)).toBe(UploadOutcome.Sent);
    expect(uploadMessage(UploadOutcome.Sent)).toBeNull();
  });

  it('a call that never ran says so', () => {
    expect(uploadOutcome(false)).toBe(UploadOutcome.NotStarted);
    expect(uploadMessage(UploadOutcome.NotStarted)).toBe('Upload did not start');
  });
});
