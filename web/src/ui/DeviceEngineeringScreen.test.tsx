// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EngineeringScreen } from '@/ui/DeviceEngineeringScreen';
import { deviceCall } from '@/ui/DeviceView';

const device = vi.hoisted(() => ({ view: {} as unknown }));

vi.mock('@/ui/DeviceView', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/ui/DeviceView')>()),
  deviceCall: vi.fn(),
  deviceUrl: (path: string) => path,
  useDeviceView: <T,>(_: string, parse: (json: unknown) => T | null) => parse(device.view),
}));

const call = vi.mocked(deviceCall);

const field = (name: string, label: string, value = '', secret = false) => ({ name, label, value, placeholder: '', secret });

const served = (clientId: string, issuer = 'https://auth.example.com') => ({
  backend: { status: { level: 3, text: 'engine' }, text: 'Engine.', switch: 'mediamtx', label: 'Switch to mediamtx', confirm: 'Restart?', success: 'Switching.' },
  publish: {
    status: { level: 3, text: 'not tested' },
    awaitingCloud: false,
    mode: 'oauth',
    modes: [{ value: 'oauth', title: 'Custom OAuth server', desc: 'OAuth.', fields: ['issuer', 'clientId'], clears: ['token'], submit: 'Save', unsaved: 'Unsaved changes.' }],
    fields: [field('issuer', 'Issuer URL', issuer), field('clientId', 'Client ID', clientId), field('token', 'Access token', 'old-token', true)],
    environment: issuer,
    verdict: { level: 3, text: 'Not tested yet.' },
  },
  diagnostics: { status: { level: 3, text: 'off' }, hint: '', endpoint: '', token: '', tokenLabel: 'Bearer token', streaming: false, canStream: false, state: '', confirm: '' },
});

const screenFor = (view: unknown) => {
  device.view = view;
  return <EngineeringScreen href="/settings/engineering" query={new URLSearchParams()} onOpen={() => undefined} onClose={() => undefined} />;
};

const signIn = () => within(screen.getByRole('region', { name: 'Publishing sign-in' }));

const textbox = (name: string) => signIn().getByRole<HTMLInputElement>('textbox', { name });

describe('EngineeringScreen sign-in', () => {
  beforeEach(() => {
    call.mockReset();
    Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }) });
  });
  afterEach(cleanup);

  it('keeps what is typed right after a save when the device reports the save', async () => {
    call.mockResolvedValue(null);
    const { rerender } = render(screenFor(served('drone')));
    fireEvent.click(signIn().getByRole('button', { name: /Advanced · sign-in credentials/ }));

    fireEvent.change(textbox('Client ID'), { target: { value: 'pilot' } });
    fireEvent.click(signIn().getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(call).toHaveBeenCalledWith('PUT', '/api/stream/sfu/oauth', { issuer: 'https://auth.example.com', clientId: 'pilot', token: '' }));
    rerender(screenFor(served('drone')));
    expect(textbox('Client ID').value).toBe('pilot');

    fireEvent.change(textbox('Client ID'), { target: { value: 'pilot-2' } });
    rerender(screenFor(served('pilot', 'https://auth2.example.com')));

    expect(textbox('Client ID').value).toBe('pilot-2');
    expect(textbox('Issuer URL').value).toBe('https://auth2.example.com');
  });
});
