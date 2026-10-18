import { describe, expect, it } from 'vitest';
import { modeNamed, parseEngineering, shownFields, signInBody, signInKey } from '@/ui/DeviceEngineering';

const field = (name: string, value = '', secret = false) => ({ name, label: name, value, placeholder: '', secret });

const served = {
  backend: { status: { level: 3, text: 'mediamtx' }, text: 'Video is carried by mediamtx.', switch: 'engine', label: 'Switch to the engine', confirm: 'Restart?', success: 'Switching.' },
  publish: {
    status: { level: 3, text: 'awaiting cloud connect' },
    awaitingCloud: true,
    mode: 'aircast',
    modes: [
      { value: 'aircast', title: 'Aircast cloud', desc: 'Managed.', fields: [], clears: ['token'], submit: 'Test sign-in', unsaved: 'Unsaved change — press Test sign-in to apply.' },
      { value: 'oauth', title: 'Custom OAuth server', desc: 'OAuth.', fields: ['issuer', 'clientId', 'clientSecret', 'tokenUrl', 'scope'], clears: ['token'], submit: 'Save', unsaved: 'Unsaved.' },
      { value: 'token', title: 'Access token (WHIP)', desc: 'Token.', fields: ['token'], clears: [], submit: 'Save', unsaved: 'Unsaved.' },
      { value: 'open', title: 'None — open server', desc: 'Open.', fields: [], clears: ['issuer', 'clientId', 'clientSecret', 'tokenUrl', 'scope', 'token'], submit: 'Save', unsaved: 'Unsaved.', confirm: 'Clear all stored sign-in settings, including the cloud issuer?' },
    ],
    fields: [field('issuer', 'https://api.dev.aircast.one'), field('clientId', 'drone'), field('clientSecret', 's3cr3t', true), field('tokenUrl'), field('scope'), field('token', 'old-token', true)],
    environment: 'https://api.dev.aircast.one',
    verdict: { level: 3, text: 'Not tested yet — press Test sign-in.' },
  },
  diagnostics: { status: { level: 3, text: 'off' }, hint: 'Stream logs.', endpoint: 'otel.aircast.one:443', token: '', tokenLabel: 'Bearer token', streaming: false, canStream: false, state: 'Save the endpoint first.', confirm: 'Stream?' },
};

const view = parseEngineering(served);
const publish = view!.publish;

describe('DeviceEngineeringTest', () => {
  it('reads the engineering page the device serves', () => {
    expect(view?.backend.switchTo).toBe('engine');
    expect(publish.mode).toBe('aircast');
    expect(publish.awaitingCloud).toBe(true);
    expect(publish.fields.find((it) => it.name === 'clientSecret')?.secret).toBe(true);
    expect(view?.diagnostics.canStream).toBe(false);
    expect(parseEngineering({ publish: {} })).toBeNull();
  });

  it('each sign-in method shows only its own fields', () => {
    expect(shownFields(publish, 'aircast')).toEqual([]);
    expect(shownFields(publish, 'oauth').map((it) => it.name)).toEqual(['issuer', 'clientId', 'clientSecret', 'tokenUrl', 'scope']);
    expect(shownFields(publish, 'token').map((it) => it.name)).toEqual(['token']);
    expect(modeNamed(publish, 'aircast')?.submit).toBe('Test sign-in');
    expect(modeNamed(publish, 'nowhere')).toBeNull();
  });

  it('saving sends every credential and empties exactly what the method clears', () => {
    expect(signInBody(publish, 'aircast', {})).toEqual({ issuer: 'https://api.dev.aircast.one', clientId: 'drone', clientSecret: 's3cr3t', tokenUrl: '', scope: '', token: '' });
    expect(signInBody(publish, 'token', { token: 'whip-token-abc' })).toEqual({ issuer: 'https://api.dev.aircast.one', clientId: 'drone', clientSecret: 's3cr3t', tokenUrl: '', scope: '', token: 'whip-token-abc' });
    expect(signInBody(publish, 'open', { issuer: 'typed' })).toEqual({ issuer: '', clientId: '', clientSecret: '', tokenUrl: '', scope: '', token: '' });
    expect(signInBody(publish, 'oauth', { clientSecret: 'new' })).toMatchObject({ clientSecret: 'new', token: '' });
  });

  it('the editor starts over only when what is saved changes', () => {
    expect(signInKey(publish)).toBe(signInKey(parseEngineering(served)!.publish));
    expect(signInKey(publish)).not.toBe(signInKey({ ...publish, mode: 'token' }));
    expect(signInKey(publish)).not.toBe(signInKey({ ...publish, fields: publish.fields.map((it) => (it.name === 'token' ? { ...it, value: 'new' } : it)) }));
  });
});
