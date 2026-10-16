import { describe, expect, it } from 'vitest';
import { UploadOutcome, downloadMessage, uploadMessage, uploadOutcome } from './UploadOutcome';

describe('UploadOutcomeTest', () => {
  it('an invoked upload is reported as sent, never as accepted', () => {
    expect(uploadOutcome(true)).toBe(UploadOutcome.Sent);
    expect(uploadMessage(UploadOutcome.Sent)).toBe('Upload sent to vehicle');
  });

  it('a download is reported as requested, never as received', () => {
    expect(downloadMessage(UploadOutcome.Sent)).toBe('Download requested from vehicle');
    expect(downloadMessage(UploadOutcome.NotStarted)).toBe('Download did not start');
  });

  it('a call that never ran says so', () => {
    expect(uploadOutcome(false)).toBe(UploadOutcome.NotStarted);
    expect(uploadMessage(UploadOutcome.NotStarted)).toBe('Upload did not start');
  });
});
