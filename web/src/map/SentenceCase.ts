const PROPER_NOUNS = new Set(['Aircast', 'Android', 'ArduPilot', 'Pixhawk', 'Herelink', 'Trimble', 'Septentrio', 'Bing', 'Esri', 'Google', 'Mapbox', 'Tianditu', 'VWorld', 'OpenAIP', 'QGroundControl', 'Wi-Fi', 'Starlink']);

const isUpper = (char: string) => char !== char.toLowerCase() && char === char.toUpperCase();
const isLower = (char: string) => char !== char.toUpperCase() && char === char.toLowerCase();
const isLetterOrDigit = (char: string) => /[\p{L}\p{N}]/u.test(char);

const keepsCase = (word: string): boolean => PROPER_NOUNS.has(word) || [...word.slice(1)].some(isUpper) || ![...word].some(isLower);

const PROPER_PHRASES = ['PX4 Pro', '3DR Solo', 'Parrot Discovery', 'Yuneec Mantis G', 'Herelink AirUnit', 'Herelink Hotspot'];

const JOINERS = /([-/])/;
const TRAILING_PUNCTUATION = /[.,:;!?)]+$/;
const RUN_TOGETHER = /^(?:[A-Z][a-z]+){2,}$/;

const spacedOut = (piece: string): string => piece.replace(/(?<=[a-z])(?=[A-Z])/g, ' ').toLowerCase();

const lowerFirst = (text: string) => (text ? text.charAt(0).toLowerCase() + text.slice(1) : text);

const casedPiece =
  (firstWord: boolean) =>
  (piece: string, part: number, pieces: readonly string[]): string => {
    if (part % 2 === 1) return piece;
    if (pieces[part - 1] === '/' && RUN_TOGETHER.test(piece) && !PROPER_NOUNS.has(piece)) return spacedOut(piece);
    const lead = [...piece].findIndex(isLetterOrDigit);
    const split = lead === -1 ? piece.length : lead;
    const body = piece.slice(split);
    return (firstWord && part === 0) || keepsCase(body) ? piece : piece.slice(0, split) + lowerFirst(body);
  };

const casedWord = (word: string, at: number): string =>
  PROPER_NOUNS.has(word.replace(TRAILING_PUNCTUATION, '')) ? word : word.split(JOINERS).map(casedPiece(at === 0)).join('');

export const sentenceCase = (label: string): string => {
  const phrase = PROPER_PHRASES.find((it) => label.startsWith(it));
  return phrase ? phrase + lowerFirst(sentenceCase(label.slice(phrase.length))) : label.split(' ').map(casedWord).join(' ');
};
