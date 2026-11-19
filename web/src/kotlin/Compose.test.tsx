// @vitest-environment jsdom
import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { forgetSaveable, historyDepth, useBackHandler, useEscapeHandler, useSaveable } from './Compose';

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

const Levels = () => {
  const [level, setLevel] = useState(0);
  useBackHandler(level >= 1, () => setLevel(0));
  useBackHandler(level >= 2, () => setLevel(1));
  return (
    <div>
      <button type="button" onClick={() => setLevel(level + 1)}>
        deeper
      </button>
      <button type="button" onClick={() => setLevel(0)}>
        close
      </button>
      <span>{`level ${level}`}</span>
    </div>
  );
};

const popped = () => new Promise<void>((resolve) => window.addEventListener('popstate', () => resolve(), { once: true }));

describe('Escape and Back', () => {
  afterEach(async () => {
    cleanup();
    await waitFor(() => expect(historyDepth(history.state)).toBe(0));
  });

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

  it('closing a page from the app pops the history entry it pushed', async () => {
    const view = render(<Levels />);
    fireEvent.click(view.getByText('deeper'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(1));
    fireEvent.click(view.getByText('close'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(0));
    expect(view.getByText('level 0')).toBeTruthy();
  });

  it('closing two levels at once pops both entries, and Back walks out one level at a time', async () => {
    const view = render(<Levels />);
    fireEvent.click(view.getByText('deeper'));
    fireEvent.click(view.getByText('deeper'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(2));
    fireEvent.click(view.getByText('close'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(0));
    expect(view.getByText('level 0')).toBeTruthy();
    fireEvent.click(view.getByText('deeper'));
    fireEvent.click(view.getByText('deeper'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(2));
    history.back();
    await waitFor(() => expect(view.getByText('level 1')).toBeTruthy());
    expect(historyDepth(history.state)).toBe(1);
    history.back();
    await waitFor(() => expect(view.getByText('level 0')).toBeTruthy());
    expect(historyDepth(history.state)).toBe(0);
  });

  it('Forward after Back does not reopen the page, and leaves no entry behind', async () => {
    const view = render(<Levels />);
    fireEvent.click(view.getByText('deeper'));
    await waitFor(() => expect(historyDepth(history.state)).toBe(1));
    history.back();
    await waitFor(() => expect(view.getByText('level 0')).toBeTruthy());
    await waitFor(() => expect(historyDepth(history.state)).toBe(0));
    const forward = popped();
    history.forward();
    await forward;
    await waitFor(() => expect(historyDepth(history.state)).toBe(0));
    expect(view.getByText('level 0')).toBeTruthy();
  });

  it('reads the depth it pushed and nothing else', () => {
    expect(historyDepth({ aircastDepth: 2 })).toBe(2);
    expect(historyDepth(null)).toBe(0);
    expect(historyDepth({ aircastDepth: 'x' })).toBe(0);
  });
});

const Draft = () => {
  const [typed, setTyped] = useSaveable('test.draft', () => '');
  return <input aria-label="draft" value={typed} onChange={(event) => setTyped(event.target.value)} />;
};

describe('useSaveable', () => {
  afterEach(() => {
    cleanup();
    forgetSaveable('test.');
  });

  it('a half-typed value survives its screen being drawn again, as when the layout switches between phone and desktop', () => {
    const first = render(<Draft />);
    fireEvent.change(first.getByLabelText('draft'), { target: { value: 'ws://192.168.1.50:5760' } });
    first.unmount();
    const again = render(<Draft />);
    expect((again.getByLabelText('draft') as HTMLInputElement).value).toBe('ws://192.168.1.50:5760');
  });

  it('a forgotten value starts again from blank', () => {
    const first = render(<Draft />);
    fireEvent.change(first.getByLabelText('draft'), { target: { value: 'ws://192.168.1.50:5760' } });
    first.unmount();
    forgetSaveable('test.');
    const again = render(<Draft />);
    expect((again.getByLabelText('draft') as HTMLInputElement).value).toBe('');
  });
});
