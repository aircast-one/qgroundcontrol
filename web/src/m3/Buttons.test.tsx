// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { IconButton, TextButton } from './Buttons';

describe('IconButton', () => {
  afterEach(cleanup);

  it('shows its label as a tooltip when focused from the keyboard', async () => {
    render(<IconButton label="Close settings" onClick={() => undefined}>x</IconButton>);
    const button = screen.getByRole('button', { name: 'Close settings' });
    await act(async () => {
      fireEvent.keyDown(document.body, { key: 'Tab' });
      button.focus();
    });
    expect((await screen.findByRole('tooltip')).textContent).toBe('Close settings');
  });

  it('a title stands in for the label, once, as the tooltip and the name', async () => {
    render(<IconButton title="Dismiss">x</IconButton>);
    const button = screen.getByRole('button', { name: 'Dismiss' });
    expect(button.getAttribute('title')).toBeNull();
    await act(async () => {
      fireEvent.keyDown(document.body, { key: 'Tab' });
      button.focus();
    });
    expect((await screen.findByRole('tooltip')).textContent).toBe('Dismiss');
  });

  it('text buttons carry no tooltip', async () => {
    render(<TextButton onClick={() => undefined}>Cancel</TextButton>);
    await act(async () => screen.getByRole('button', { name: 'Cancel' }).focus());
    expect(screen.queryByRole('tooltip')).toBeNull();
  });
});
