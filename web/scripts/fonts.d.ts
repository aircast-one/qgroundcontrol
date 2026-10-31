declare module 'subset-font' {
  export default function subsetFont(font: Buffer, text: string, options?: { targetFormat?: 'sfnt' | 'woff' | 'woff2'; noLayoutClosure?: boolean }): Promise<Buffer>;
}

declare module 'fontverter' {
  export function convert(font: Buffer, to: 'sfnt' | 'woff' | 'woff2'): Promise<Buffer>;
}
