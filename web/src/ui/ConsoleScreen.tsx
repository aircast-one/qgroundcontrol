import { type ClipboardEvent, type ReactNode, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { type Json, optArrayText, optJSONArray } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { coerceIn } from '@/kotlin/Std';
import { SmallFloatingActionButton, Text, TextButton } from '@/m3';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';

const CONSOLE_ROOT = 'mavlinkConsole';
const CONSOLE_VIEW = 'view.mavlinkConsole';

export const CONSOLE_OPEN = `${CONSOLE_ROOT}.open`;
export const CONSOLE_EMPTY_TEXT = '> ';

export type TextRange = { start: number; end: number };

export const TextRange = (start: number, end: number = start): TextRange => ({ start, end });

export type TextFieldValue = { text: string; selection: TextRange };

export const TextFieldValue = (text: string, selection: TextRange = TextRange(0)): TextFieldValue => ({ text, selection });

export const splitCompleteLines = (field: TextFieldValue): readonly [string | null, TextFieldValue] => {
  const cut = field.text.lastIndexOf('\n');
  if (cut < 0) return [null, field];
  const leftover = field.text.slice(cut + 1);
  return [field.text.slice(0, cut), TextFieldValue(leftover, TextRange(coerceIn(field.selection.end - cut - 1, 0, leftover.length)))];
};

export const shouldFollowTail = (lastVisibleIndex: number | null, count: number): boolean => lastVisibleIndex === null || lastVisibleIndex >= count - 2;

const lastVisibleIndex = (list: HTMLElement): number | null => {
  const bottom = list.getBoundingClientRect().bottom;
  const index = [...list.children].findLastIndex((child) => child.getBoundingClientRect().top < bottom);
  return index < 0 ? null : index;
};

const HISTORY_STEPS: readonly (readonly [string, string])[] = [
  ['historyUp', '↑'],
  ['historyDown', '↓'],
];

export const ConsoleScreen = ({ className }: { className?: string }) => {
  const consoleJson = useQgcPath(CONSOLE_VIEW);
  const lines = useMemo(() => consoleLines(consoleJson), [consoleJson]);
  const [field, setField] = useState<TextFieldValue>(() => TextFieldValue(''));
  const listRef = useRef<HTMLDivElement | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const warning = aircast('warning');
  const error = md('error');

  useLaunchedEffect([], () => void Qgc.invoke(CONSOLE_OPEN));

  const scrollToEnd = () => {
    const list = listRef.current;
    if (list !== null) list.scrollTop = list.scrollHeight;
  };

  const sendText = (toSend: string) => {
    void Qgc.invoke(`${CONSOLE_ROOT}.sendCommand`, toSend);
  };

  const send = () => {
    const toSend = field.text;
    setField(TextFieldValue(''));
    sendText(toSend);
    if (lines.length > 0) scrollToEnd();
  };

  const edit = (next: TextFieldValue) => {
    const [complete, leftover] = splitCompleteLines(next);
    setField(leftover);
    if (complete !== null) sendText(complete);
  };

  const paste = (event: ClipboardEvent<HTMLInputElement>) => {
    const pasted = event.clipboardData.getData('text/plain');
    if (!pasted.includes('\n')) return;
    event.preventDefault();
    const input = event.currentTarget;
    const start = input.selectionStart ?? input.value.length;
    const end = input.selectionEnd ?? start;
    edit(TextFieldValue(input.value.slice(0, start) + pasted + input.value.slice(end), TextRange(start + pasted.length)));
  };

  const recall = async (step: string) => {
    const current = field.text;
    const result = await Qgc.invokeResult(`${CONSOLE_ROOT}.${step}`, current);
    const recalled = typeof result === 'string' ? result : current;
    setField(TextFieldValue(recalled, TextRange(recalled.length)));
  };

  useEffect(() => {
    const list = listRef.current;
    if (list !== null && lines.length > 0 && shouldFollowTail(lastVisibleIndex(list), lines.length)) scrollToEnd();
  }, [lines.length]);

  useLayoutEffect(() => {
    const input = inputRef.current;
    if (input !== null && (input.selectionStart !== field.selection.start || input.selectionEnd !== field.selection.end)) input.setSelectionRange(field.selection.start, field.selection.end);
  }, [field]);

  return (
    <div className={`flex size-full flex-col ${className ?? ''}`}>
      <div ref={listRef} className="flex min-h-0 w-full flex-1 flex-col gap-1 overflow-y-auto bg-surface-container-lowest p-4">
        {lines.length === 0 ? (
          <Text typography="bodyMedium" monospace color={md('onSurface')} className="whitespace-pre-wrap">
            {CONSOLE_EMPTY_TEXT}
          </Text>
        ) : null}
        {lines.map((line, index) => (
          <Text key={index} typography="bodyMedium" monospace color={isPromptLine(line) ? md('primary') : md('onSurface')} className="whitespace-pre-wrap break-words">
            {styledText(consoleLineStyled(line, warning, error))}
          </Text>
        ))}
      </div>

      <div className="flex w-full items-center gap-2 bg-surface-container-low px-4 py-2.5">
        <input
          ref={inputRef}
          className="h-14 min-w-0 flex-1 rounded-full bg-surface-container-highest px-4 text-body-large text-on-surface outline-none placeholder:text-on-surface-variant"
          value={field.text}
          placeholder="Enter commands here..."
          aria-label="Enter commands here..."
          enterKeyHint="send"
          onChange={(event) => {
            const length = event.target.value.length;
            edit(TextFieldValue(event.target.value, TextRange(event.target.selectionStart ?? length, event.target.selectionEnd ?? length)));
          }}
          onPaste={paste}
          onKeyDown={(event) => {
            if (event.key !== 'Enter') return;
            event.preventDefault();
            send();
          }}
        />
        {HISTORY_STEPS.map(([step, arrow]) => (
          <TextButton key={step} className="size-10 px-0!" onClick={() => void recall(step)}>
            {arrow}
          </TextButton>
        ))}
        <SmallFloatingActionButton onClick={send} containerColor={md('primaryContainer')} contentColor={md('onPrimaryContainer')} label="Send">
          <Drawable name="ic_send" />
        </SmallFloatingActionButton>
      </div>
    </div>
  );
};

export const isPromptLine = (line: string): boolean => /^\w*sh> /.test(line);

export type SpanStyle = { color: string };

export type AnnotatedString = { text: string; spanStyles: readonly { start: number; end: number; item: SpanStyle }[] };

export const consoleLineStyled = (line: string, warning: string, error: string): AnnotatedString => {
  const marked = (
    [
      ['WARN', warning],
      ['ERROR', error],
    ] as const
  ).find(([prefix]) => line.startsWith(prefix));
  return { text: line, spanStyles: marked === undefined ? [] : [{ start: 0, end: marked[0].length, item: { color: marked[1] } }] };
};

const styledText = (value: AnnotatedString): ReactNode => {
  const span = value.spanStyles[0];
  return span === undefined ? (
    value.text
  ) : (
    <>
      {value.text.slice(0, span.start)}
      <span style={{ color: span.item.color }}>{value.text.slice(span.start, span.end)}</span>
      {value.text.slice(span.end)}
    </>
  );
};

export const consoleLines = (view: Json | null): readonly string[] => {
  const array = optJSONArray(view, 'lines');
  return array === null ? [] : array.map((_, index) => optArrayText(array, index));
};
