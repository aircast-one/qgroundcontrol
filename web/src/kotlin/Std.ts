export const coerceIn = (value: number, minimum: number, maximum: number): number => Math.min(maximum, Math.max(minimum, value));

export const coerceAtLeast = (value: number, minimum: number): number => Math.max(minimum, value);

export const coerceAtMost = (value: number, maximum: number): number => Math.min(maximum, value);

export const roundToInt = (value: number): number => {
  if (Number.isNaN(value)) throw new Error('Cannot round NaN value.');
  return Math.round(value);
};

export const isFinite = (value: number | null | undefined): value is number => typeof value === 'number' && Number.isFinite(value);

export const delay = (milliseconds: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, milliseconds));

const pad = (text: string, width: number, zero: boolean, left: boolean) => {
  if (text.length >= width) return text;
  if (left) return text.padEnd(width, ' ');
  if (!zero) return text.padStart(width, ' ');
  const sign = text.startsWith('-') ? '-' : '';
  return sign + text.slice(sign.length).padStart(width - sign.length, '0');
};

const grouped = (text: string) => {
  const [whole = '', fraction] = text.split('.');
  const sign = whole.startsWith('-') ? '-' : '';
  const digits = whole.slice(sign.length).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return sign + digits + (fraction === undefined ? '' : `.${fraction}`);
};

export const format = (pattern: string, ...args: unknown[]): string => {
  const state = { next: 0 };
  return pattern.replace(/%(\d+\$)?([-0,+ ]*)(\d+)?(?:\.(\d+))?([dfsxXeEc%])/g, (_match, position: string | undefined, flags: string, width: string | undefined, precision: string | undefined, kind: string) => {
    if (kind === '%') return '%';
    const index = position ? Number(position.slice(0, -1)) - 1 : state.next++;
    const value = args[index];
    const number = typeof value === 'number' ? value : Number(value);
    const body = (() => {
      switch (kind) {
        case 'd': return Math.trunc(number).toString();
        case 'f': return number.toFixed(precision === undefined ? 6 : Number(precision));
        case 'e': return number.toExponential(precision === undefined ? 6 : Number(precision));
        case 'E': return number.toExponential(precision === undefined ? 6 : Number(precision)).toUpperCase();
        case 'x': return Math.trunc(number).toString(16);
        case 'X': return Math.trunc(number).toString(16).toUpperCase();
        case 'c': return String(value);
        default: return precision === undefined ? String(value) : String(value).slice(0, Number(precision));
      }
    })();
    const signed = flags.includes('+') && number >= 0 && 'dfeE'.includes(kind) ? `+${body}` : body;
    const withGroups = flags.includes(',') && 'df'.includes(kind) ? grouped(signed) : signed;
    return pad(withGroups, width ? Number(width) : 0, flags.includes('0'), flags.includes('-'));
  });
};

export const replaceFirstCharUpper = (text: string): string => (text ? text.charAt(0).toUpperCase() + text.slice(1) : text);

export const replaceFirstCharLower = (text: string): string => (text ? text.charAt(0).toLowerCase() + text.slice(1) : text);

export const same = (a: unknown, b: unknown): boolean => {
  if (Object.is(a, b)) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (a instanceof Map && b instanceof Map) return a.size === b.size && [...a].every(([key, value]) => b.has(key) && same(value, b.get(key)));
  if (a instanceof Set && b instanceof Set) return a.size === b.size && [...a].every((value) => b.has(value));
  const keysA = Object.keys(a);
  const keysB = Object.keys(b);
  return keysA.length === keysB.length && keysA.every((key) => same((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]));
};
