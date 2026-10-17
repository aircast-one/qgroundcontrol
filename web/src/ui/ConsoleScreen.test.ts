import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { Color } from '@/theme/Colors';
import { CONSOLE_EMPTY_TEXT, CONSOLE_OPEN, TextFieldValue, TextRange, consoleLineStyled, consoleLines, isPromptLine, shouldFollowTail, splitCompleteLines } from './ConsoleScreen';

describe('ConsoleScreenTest', () => {
  it('an empty console shows the bare prompt like the MAVLinkConsolePage text area', () => {
    expect(CONSOLE_EMPTY_TEXT).toBe('> ');
  });

  it('opening the page asks the core for a fresh controller', () => {
    expect(CONSOLE_OPEN).toBe('mavlinkConsole.open');
  });
});

describe('ConsoleFollowTailTest', () => {
  it('an empty list follows the tail', () => {
    expect(shouldFollowTail(null, 0)).toBe(true);
  });

  it('a reader at the bottom keeps following', () => {
    expect(shouldFollowTail(9, 10)).toBe(true);
  });

  it('a reader one line behind still follows so a new line does not strand them', () => {
    expect(shouldFollowTail(9, 11)).toBe(true);
  });

  it('a reader scrolled up is left where they are', () => {
    expect(shouldFollowTail(3, 40)).toBe(false);
  });

  it('scrolling up by two lines is enough to stop the yank', () => {
    expect(shouldFollowTail(7, 10)).toBe(false);
  });
});

describe('ConsoleLinesTest', () => {
  it('lines come from the served array', () => {
    expect(consoleLines(parseJson('{"lines":["a","b"]}'))).toEqual(['a', 'b']);
    expect(consoleLines(parseJson('{}'))).toEqual([]);
    expect(consoleLines(null)).toEqual([]);
  });

  it('a leading WARN or ERROR is coloured, as MAVLinkConsoleController marks it', () => {
    const warn = Color.Yellow;
    const error = Color.Red;
    const spans = (line: string) => consoleLineStyled(line, warn, error).spanStyles.map((it) => [it.start, it.end, it.item.color]);
    expect(spans('WARN  [sensors] no baro')).toEqual([[0, 4, warn]]);
    expect(spans('ERROR x')).toEqual([[0, 5, error]]);
    expect(spans('warn: an INFO WARN')).toEqual([]);
  });

  it('promptLinesAreTheEchoedCommands', () => {
    expect(isPromptLine('nsh> free')).toBe(true);
    expect(isPromptLine('total  used  free')).toBe(false);
    expect(isPromptLine('note: nsh> appears mid-line')).toBe(false);
  });
});

describe('ConsolePasteTest', () => {
  it('a pasted block sends its complete lines and keeps the unfinished tail like handleClipboard', () => {
    const [sent, left] = splitCompleteLines(TextFieldValue('ver all\nfree\nto', TextRange(15)));
    expect(sent).toBe('ver all\nfree');
    expect(left.text).toBe('to');
    expect(left.selection).toEqual(TextRange(2));
  });

  it('text after the cursor stays behind the pasted tail', () => {
    const [sent, left] = splitCompleteLines(TextFieldValue('lsfree\nps\n -l', TextRange(10)));
    expect(sent).toBe('lsfree\nps');
    expect(left.text).toBe(' -l');
    expect(left.selection).toEqual(TextRange(0));
  });

  it('typing without a newline sends nothing', () => {
    const field = TextFieldValue('help', TextRange(4));
    expect(splitCompleteLines(field)).toEqual([null, field]);
  });
});
