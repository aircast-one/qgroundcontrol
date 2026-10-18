import { describe, expect, it } from 'vitest';
import { CloudStage, parseDeviceSettings } from '@/ui/DeviceSettings';
import { DeviceLevel } from '@/ui/DeviceLevel';

const served = {
  health: { level: 2, text: 'Mobile data down', href: '/network#cellular' },
  notice: { level: 1, text: 'v0.9.1 is available — install it under Software.' },
  stats: [{ label: 'Services', value: 'All running', hint: '3 supervised units', level: 0 }],
  services: [{ label: 'Video server', state: 'running', level: 0 }],
  cloud: { status: { level: 3, text: 'waiting for sign-in' }, stage: 'signing-in', userCode: 'ABCD-1234', signInUrl: 'https://app.aircast.one/device', issuer: 'https://api.aircast.one' },
  software: {
    status: { level: 1, text: 'Update failed' },
    summary: 'v0.9.0',
    notices: [{ level: 2, title: 'The update to v0.9.1 did not finish.', text: 'You can try again.', detailLabel: 'What went wrong', detail: 'E: dpkg' }],
    candidate: { title: 'v0.9.1 is available', notesUrl: 'https://aircast.one/docs/releases#v0.9.1', install: 'Install v0.9.1', confirm: 'Install v0.9.1?', success: 'Installing v0.9.1' },
    notes: ['Updates come from the Stable channel.'],
    checked: 'Checked just now.',
    canCheck: true,
  },
  hostname: 'falcon',
};

describe('DeviceSettingsTest', () => {
  it('reads the device page the device serves', () => {
    const view = parseDeviceSettings(served);
    expect(view?.health).toEqual({ level: DeviceLevel.Blocked, text: 'Mobile data down', href: '/network#cellular' });
    expect(view?.notice?.level).toBe(DeviceLevel.Warn);
    expect(view?.stats[0]).toEqual({ label: 'Services', value: 'All running', hint: '3 supervised units', level: DeviceLevel.Ready });
    expect(view?.services).toEqual([{ label: 'Video server', state: 'running', level: DeviceLevel.Ready }]);
    expect(view?.cloud).toMatchObject({ stage: CloudStage.SigningIn, userCode: 'ABCD-1234', deviceId: '', error: '' });
    expect(view?.software.candidate?.install).toBe('Install v0.9.1');
    expect(view?.software.notices[0]?.detail).toBe('E: dpkg');
    expect(view?.software.notes).toEqual(['Updates come from the Stable channel.']);
    expect(view?.hostname).toBe('falcon');
  });

  it('an answer without a health line is not a device page', () => {
    expect(parseDeviceSettings({ stats: [] })).toBeNull();
    expect(parseDeviceSettings(null)).toBeNull();
  });

  it('missing pieces read as nothing to show, never as a crash', () => {
    const view = parseDeviceSettings({ health: { level: 0, text: 'All systems operational' } });
    expect(view?.health.href).toBe('');
    expect(view?.notice).toBeNull();
    expect(view?.cloud.stage).toBe(CloudStage.Connect);
    expect(view?.software.candidate).toBeNull();
    expect(view?.software.canCheck).toBe(false);
  });
});
