import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { betaDiagnostics } from './BetaDiagnosticsDialog';

describe('BetaDiagnosticsDialogTest', () => {
  it('the offer shows while the core says a beta build has not answered', () => {
    const prompt = betaDiagnostics(
      parseJson('{"show":true,"title":"Help us debug this beta","text":"Share logs?","accept":"Share logs","dismiss":"Not now","endpoint":"otel.aircast.one:443"}'),
    )!;
    expect(prompt.title).toBe('Help us debug this beta');
    expect(prompt.accept).toBe('Share logs');
    expect(prompt.dismiss).toBe('Not now');
    expect(prompt.endpoint).toBe('otel.aircast.one:443');
    expect(betaDiagnostics(parseJson('{"show":false}'))).toBeNull();
    expect(betaDiagnostics(null)).toBeNull();
  });
});
