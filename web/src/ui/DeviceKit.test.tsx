// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DeviceSelect } from './DeviceKit';

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
