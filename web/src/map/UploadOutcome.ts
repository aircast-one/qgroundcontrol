export type UploadOutcome = { readonly name: 'Sent' | 'NotStarted'; readonly ordinal: number };

export const UploadOutcome = {
  Sent: { name: 'Sent', ordinal: 0 },
  NotStarted: { name: 'NotStarted', ordinal: 1 },
} as const satisfies Record<string, UploadOutcome>;

export const UploadOutcomeEntries: readonly UploadOutcome[] = [UploadOutcome.Sent, UploadOutcome.NotStarted];

export const uploadOutcome = (invoked: boolean): UploadOutcome => (invoked ? UploadOutcome.Sent : UploadOutcome.NotStarted);

export const uploadMessage = (outcome: UploadOutcome): string | null => (outcome === UploadOutcome.Sent ? null : 'Upload did not start');
