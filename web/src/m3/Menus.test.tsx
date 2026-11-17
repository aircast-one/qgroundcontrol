// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { DropdownMenu, DropdownMenuItem } from './Menus';

describe('a choice menu shows which option is chosen', () => {
  afterEach(cleanup);

  it('marks the chosen option with a check and aria-current, and leaves the others plain', async () => {
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
    expect(chosen.textContent).toContain('check');
    expect(other.hasAttribute('aria-current')).toBe(false);
    expect(other.textContent).not.toContain('check');
  });

  it('opens at least as wide as the field it drops from', async () => {
    render(
      <div>
        <DropdownMenu expanded onDismissRequest={() => undefined}>
          <DropdownMenuItem text="VTOL" onClick={() => undefined} />
        </DropdownMenu>
      </div>,
    );
    const menu = await screen.findByRole('menu');
    expect(menu.className).toContain('min-w-[max(160px,var(--anchor-width))]');
  });
});
