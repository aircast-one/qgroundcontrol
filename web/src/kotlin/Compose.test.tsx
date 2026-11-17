// @vitest-environment jsdom
import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { historyDepth, useBackHandler, useEscapeHandler } from './Compose';

const Screen = ({ onPage }: { onPage: (page: string) => void }) => {
  const [page, setPage] = useState('home');
  const [selected, setSelected] = useState(true);
  useBackHandler(page !== 'home', () => {
    setPage('home');
    onPage('home');
  });
  useEscapeHandler(selected, () => setSelected(false));
  return (
    <div>
      <button type="button" onClick={() => setPage('detail')}>
        open
      </button>
      <span>{`${page}|${selected ? 'selected' : 'none'}`}</span>
    </div>
  );
};

const popped = () => new Promise<void>((resolve) => window.addEventListener('popstate', () => resolve(), { once: true }));

describe('Escape and Back', () => {
  afterEach(cleanup);

  it('Escape only closes or deselects, and never leaves a page', async () => {
    const view = render(<Screen onPage={() => undefined} />);
    fireEvent.click(view.getByText('open'));
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(view.getByText('detail|none')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(view.getByText('detail|none')).toBeTruthy();
  });

  it('opening a page pushes a history entry and Back returns to the page before it, inside the app', async () => {
    const onPage = vi.fn();
    const view = render(<Screen onPage={onPage} />);
    fireEvent.click(view.getByText('open'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(1));
    const back = popped();
    history.back();
    await back;
    await waitFor(() => expect(view.getByText('home|selected')).toBeTruthy());
    expect(onPage).toHaveBeenCalledExactlyOnceWith('home');
    expect(historyDepth(history.state)).toBe(0);
  });

  it('reads the depth it pushed and nothing else', () => {
    expect(historyDepth({ aircastDepth: 2 })).toBe(2);
    expect(historyDepth(null)).toBe(0);
    expect(historyDepth({ aircastDepth: 'x' })).toBe(0);
  });
});
