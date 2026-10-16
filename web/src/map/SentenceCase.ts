const PROPER_NOUNS = new Set(['Android', 'ArduPilot', 'Pixhawk', 'Herelink', 'Trimble', 'Septentrio', 'Bing', 'Esri', 'Google', 'Mapbox', 'Tianditu', 'VWorld', 'OpenAIP', 'QGroundControl']);

const isUpper = (char: string) => char !== char.toLowerCase() && char === char.toUpperCase();
const isLower = (char: string) => char !== char.toUpperCase() && char === char.toLowerCase();
const isLetterOrDigit = (char: string) => /[\p{L}\p{N}]/u.test(char);

const keepsCase = (word: string): boolean => PROPER_NOUNS.has(word) || [...word.slice(1)].some(isUpper) || ![...word].some(isLower);

const PROPER_PHRASES = ['PX4 Pro', '3DR Solo', 'Parrot Discovery', 'Yuneec Mantis G', 'Herelink AirUnit', 'Herelink Hotspot'];

const lowerFirst = (text: string) => (text ? text.charAt(0).toLowerCase() + text.slice(1) : text);

export const sentenceCase = (label: string): string => {
  const phrase = PROPER_PHRASES.find((it) => label.startsWith(it));
  if (phrase) return phrase + lowerFirst(sentenceCase(label.slice(phrase.length)));
  return label
    .split(' ')
    .map((word, at) =>
      word
        .split('-')
        .map((piece, part) => {
          const lead = [...piece].findIndex(isLetterOrDigit);
          const split = lead === -1 ? piece.length : lead;
          const body = piece.slice(split);
          return (at === 0 && part === 0) || keepsCase(body) ? piece : piece.slice(0, split) + lowerFirst(body);
        })
        .join('-'),
    )
    .join(' ');
};
