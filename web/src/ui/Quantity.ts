const TIGHT_UNITS: ReadonlySet<string> = new Set(['°', '%']);
const MEGABYTES_PER_GIGABYTE = 1024;
const WHOLE_NUMBER = /^(-?\d+)\.0+$/;
const PLAIN_NUMBER = /^-?\d+(\.\d+)?$/;

export const withUnit = (value: string, unit: string): string => {
  const shown = unit.trim();
  if (shown === '' || value.trim() === '') return value;
  return TIGHT_UNITS.has(shown) ? `${value}${shown}` : `${value} ${shown}`;
};

export const wholeIfWhole = (value: string): string => value.trim().replace(WHOLE_NUMBER, '$1');

export const megabytesText = (megabytes: number): string =>
  megabytes >= MEGABYTES_PER_GIGABYTE ? withUnit(wholeIfWhole((megabytes / MEGABYTES_PER_GIGABYTE).toFixed(1)), 'GB') : withUnit(wholeIfWhole(String(megabytes)), 'MB');

export const quantityText = (value: string, unit: string): string => {
  const shown = unit.trim();
  const plain = PLAIN_NUMBER.test(value.trim());
  if (plain && shown === 'MB') return megabytesText(Number(value));
  return withUnit(plain ? wholeIfWhole(value) : value.trim(), shown);
};
