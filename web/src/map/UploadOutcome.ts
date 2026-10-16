export type UploadOutcome = { readonly name: 'Sent' | 'NotStarted'; readonly ordinal: number };

export const UploadOutcome = {
  Sent: { name: 'Sent', ordinal: 0 },
  NotStarted: { name: 'NotStarted', ordinal: 1 },
} as const satisfies Record<string, UploadOutcome>;

export const UploadOutcomeEntries: readonly UploadOutcome[] = [UploadOutcome.Sent, UploadOutcome.NotStarted];

export const uploadOutcome = (invoked: boolean): UploadOutcome => (invoked ? UploadOutcome.Sent : UploadOutcome.NotStarted);

export const uploadMessage = (outcome: UploadOutcome): string => (outcome === UploadOutcome.Sent ? 'Upload sent to vehicle' : 'Upload did not start');

export const downloadMessage = (outcome: UploadOutcome): string => (outcome === UploadOutcome.Sent ? 'Download requested from vehicle' : 'Download did not start');
