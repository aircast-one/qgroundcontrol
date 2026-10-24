// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SETTLE_MS } from '@/ui/DeviceDraft';
import { deviceCall } from '@/ui/DeviceView';
import { DeviceForm, DeviceSelect, DeviceSwitchRow } from './DeviceKit';

vi.mock('@/ui/DeviceView', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/ui/DeviceView')>()), deviceCall: vi.fn() }));

const call = vi.mocked(deviceCall);

const USAGE = [
  { value: 'no', label: 'Unmetered' },
  { value: 'yes', label: 'Metered' },
];

describe('DeviceSelect', () => {
  afterEach(cleanup);

  it('picking an option hands it over and closes the menu', async () => {
    const picked = vi.fn();
    render(<DeviceSelect label="Data usage" value="no" options={USAGE} onPick={picked} />);

    fireEvent.click(screen.getByRole('button', { name: 'Data usage' }));
    fireEvent.click(await screen.findByRole('menuitemcheckbox', { name: 'Metered' }));

    expect(picked).toHaveBeenCalledWith('yes');
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
  });
});

const RECORD = '/api/streams/cam2/record';

const recordSwitch = (checked: boolean) => (
  <DeviceSwitchRow title="Record to disk" checked={checked} on={{ method: 'PUT', path: RECORD }} off={{ method: 'DELETE', path: RECORD }} success="Recording on." offSuccess="Recording off." />
);

const recordToggle = () => screen.getByRole('switch', { name: 'Record to disk' });

const shownChecked = () => recordToggle().getAttribute('aria-checked');

const answered = (writes: number) =>
  waitFor(() => {
    expect(call).toHaveBeenCalledTimes(writes);
    expect(recordToggle().hasAttribute('data-disabled')).toBe(false);
  });

describe('DeviceSwitchRow', () => {
  beforeEach(() => call.mockReset());
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('shows what the device accepted while the next view model still carries the old value', async () => {
    call.mockResolvedValue(null);
    const { rerender } = render(recordSwitch(false));

    fireEvent.click(recordToggle());
    await answered(1);
    rerender(recordSwitch(false));

    expect(shownChecked()).toBe('true');
    rerender(recordSwitch(true));
    expect(shownChecked()).toBe('true');
  });

  it('a quick second tap acts on the accepted value, and the first tap arriving late does not flick it back', async () => {
    call.mockResolvedValue(null);
    const { rerender } = render(recordSwitch(false));

    fireEvent.click(recordToggle());
    await answered(1);
    fireEvent.click(recordToggle());
    await answered(2);

    expect(call).toHaveBeenNthCalledWith(1, 'PUT', RECORD, undefined);
    expect(call).toHaveBeenNthCalledWith(2, 'DELETE', RECORD, undefined);
    expect(shownChecked()).toBe('false');
    rerender(recordSwitch(true));
    expect(shownChecked()).toBe('false');
    rerender(recordSwitch(false));
    expect(shownChecked()).toBe('false');
  });

  it('a refused write puts the switch back and says why', async () => {
    call.mockResolvedValue({ message: 'cam2 is not streaming.', field: '' });
    render(recordSwitch(false));

    fireEvent.click(recordToggle());

    expect((await screen.findByRole('alert')).textContent).toBe('cam2 is not streaming.');
    expect(shownChecked()).toBe('false');
  });

  it('gives way to the device when the device never reports the accepted value', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    call.mockResolvedValue(null);
    const { rerender } = render(recordSwitch(false));

    await act(async () => fireEvent.click(recordToggle()));
    rerender(recordSwitch(false));
    expect(shownChecked()).toBe('true');
    act(() => vi.advanceTimersByTime(SETTLE_MS));

    expect(shownChecked()).toBe('false');
  });
});

const apnForm = (apn: string, metered = 'no') => (
  <DeviceForm
    method="PUT"
    path="/api/network/cellular"
    submit="Save"
    fields={[
      { name: 'apn', label: 'APN', value: apn },
      { name: 'metered', label: 'Data usage', value: metered, options: USAGE },
    ]}
  />
);

const apnField = () => screen.getByRole<HTMLInputElement>('textbox', { name: 'APN' });

const type = (value: string) => fireEvent.change(apnField(), { target: { value } });

describe('DeviceForm', () => {
  beforeEach(() => call.mockReset());
  afterEach(cleanup);

  it('keeps what is typed after a save while the device catches up, and takes untouched fields from the device', async () => {
    call.mockResolvedValue(null);
    const { rerender } = render(apnForm('internet'));

    type('telenor');
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(call).toHaveBeenCalledWith('PUT', '/api/network/cellular', { apn: 'telenor', metered: 'no' }));
    rerender(apnForm('internet'));
    expect(apnField().value).toBe('telenor');

    type('telenor-iot');
    rerender(apnForm('telenor', 'yes'));

    expect(apnField().value).toBe('telenor-iot');
    expect(screen.getByRole('button', { name: 'Data usage' }).textContent).toContain('Metered');
  });

  it('a field is the device’s again once the device reports what was saved', async () => {
    call.mockResolvedValue(null);
    const { rerender } = render(apnForm('internet'));

    type('telenor');
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(call).toHaveBeenCalledTimes(1));
    rerender(apnForm('telenor'));
    rerender(apnForm('changed-elsewhere'));

    expect(apnField().value).toBe('changed-elsewhere');
  });

  it('a create form starts empty again once the device accepts it', async () => {
    call.mockResolvedValue(null);
    render(<DeviceForm method="POST" path="/api/share" submit="Create link" fields={[{ name: 'label', label: 'Who is it for', value: '' }]} />);

    fireEvent.change(screen.getByRole('textbox', { name: 'Who is it for' }), { target: { value: 'command post' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create link' }));

    await waitFor(() => expect(screen.getByRole<HTMLInputElement>('textbox', { name: 'Who is it for' }).value).toBe(''));
  });
});
