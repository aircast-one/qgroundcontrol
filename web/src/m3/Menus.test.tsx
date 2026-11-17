// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { DropdownMenu, DropdownMenuItem } from './Menus';

const marks = (item: HTMLElement): number => item.querySelectorAll('[aria-hidden="true"]').length;

describe('a choice menu shows which option is chosen', () => {
  afterEach(cleanup);

  it('marks the chosen option for screen readers with aria-current and for the eye with a mark, and leaves the others plain', async () => {
    render(
      <div>
        <DropdownMenu expanded onDismissRequest={() => undefined}>
          <DropdownMenuItem text="Multi-rotor" selected onClick={() => undefined} />
          <DropdownMenuItem text="Fixed wing" selected={false} onClick={() => undefined} />
        </DropdownMenu>
      </div>,
    );
    const chosen = await screen.findByRole('menuitem', { name: 'Multi-rotor' });
    const other = screen.getByRole('menuitem', { name: 'Fixed wing' });
    expect(chosen.getAttribute('aria-current')).toBe('true');
    expect(other.hasAttribute('aria-current')).toBe(false);
    expect(marks(chosen)).toBe(1);
    expect(marks(other)).toBe(0);
  });
});
