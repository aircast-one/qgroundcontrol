import { type GetResourceResponse, type RequestParameters, addProtocol } from 'maplibre-gl';
import { optText } from '@/bridge/Json';
import { call } from '@/core/client';
import { MapBridge } from './MapBridge';
import { OSM_TILE_URL } from './VehicleMap';

export const QGC_TILE_HOST = 'qgc.tiles';

const TILE_PATH = /^\/([^/]+)\/(\d+)\/(\d+)\/(\d+)$/;
const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47];
const MAP_PROVIDER = 'settings.flightMapSettings.mapProvider.rawValue';
const MAP_TYPE = 'settings.flightMapSettings.mapType.rawValue';
const QGC_TILE_PROTOCOL = 'qgc';
const QGC_TILE_ORIGIN = `https://${QGC_TILE_HOST}`;

export const MapTileHost: { fetch: ((mapType: string, x: number, y: number, zoom: number) => Promise<Uint8Array | null>) | null } = {
  fetch: async (mapType, x, y, zoom) => (await call<Uint8Array | undefined>('mapTile', mapType, x, y, zoom)) ?? null,
};

export type TileAddress = { readonly mapType: string; readonly z: number; readonly x: number; readonly y: number };

const formDecode = (text: string): string => decodeURIComponent(text.replace(/\+/g, ' '));

const formEncode = (text: string): string =>
  encodeURIComponent(text)
    .replace(/[!'()~]/g, (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`)
    .replace(/%20/g, '+');

export const tileAddress = (path: string): TileAddress | null => {
  const found = TILE_PATH.exec(path);
  if (found === null) return null;
  const [, type = '', z = '', x = '', y = ''] = found;
  try {
    return { mapType: formDecode(type), z: Number.parseInt(z, 10), x: Number.parseInt(x, 10), y: Number.parseInt(y, 10) };
  } catch {
    return null;
  }
};

export const qgcTileUrl = (mapType: string): string => `https://${QGC_TILE_HOST}/${formEncode(mapType)}/{z}/{x}/{y}`;

export const osmTileUrl = (address: TileAddress): string =>
  OSM_TILE_URL.replace('{z}', `${address.z}`).replace('{x}', `${address.x}`).replace('{y}', `${address.y}`);

export const mapTypeName = (provider: string, type: string): string => [provider, type].filter((it) => it.trim() !== '').join(' ');

const setting = async (path: string): Promise<string> => optText(await MapBridge.read(path), 'value');

export const currentMapType = async (): Promise<string> => mapTypeName(await setting(MAP_PROVIDER), await setting(MAP_TYPE));

export const tileMimeType = (tile: Uint8Array): string =>
  tile.length >= PNG_MAGIC.length && PNG_MAGIC.every((byte, index) => tile[index] === byte) ? 'image/png' : 'image/jpeg';

export const coreTile = async (encodedPath: string): Promise<Uint8Array | null> => {
  const address = tileAddress(encodedPath);
  if (address === null) return null;
  try {
    return (await MapTileHost.fetch?.(address.mapType, address.x, address.y, address.z)) ?? null;
  } catch {
    return null;
  }
};

export const qgcRasterStyle = (mapType: string): string => `
{
  "version": 8,
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "sources": {
    "qgc": {
      "type": "raster",
      "tiles": ["${qgcTileUrl(mapType)}"],
      "tileSize": 256,
      "maxzoom": 20,
      "attribution": "${mapType}"
    }
  },
  "layers": [ { "id": "qgc", "type": "raster", "source": "qgc" } ]
}
`;

const served = async (tile: Uint8Array): Promise<GetResourceResponse<ImageBitmap>> => ({
  data: await createImageBitmap(new Blob([tile as Uint8Array<ArrayBuffer>], { type: tileMimeType(tile) })),
});

const fetched = async (url: string): Promise<GetResourceResponse<ArrayBuffer>> => {
  const response = await fetch(url);
  if (!response.ok) throw new Error('No tile');
  return { data: await response.arrayBuffer() };
};

export const QgcTileInterceptor = {
  transformRequest: (url: string): RequestParameters | undefined =>
    url.startsWith(`${QGC_TILE_ORIGIN}/`) ? { url: `${QGC_TILE_PROTOCOL}${url.slice('https'.length)}` } : undefined,

  async intercept(request: RequestParameters): Promise<GetResourceResponse<ImageBitmap | ArrayBuffer>> {
    const url = new URL(`https${request.url.slice(QGC_TILE_PROTOCOL.length)}`);
    if (url.host !== QGC_TILE_HOST) return fetched(url.toString());
    const address = tileAddress(url.pathname);
    if (address === null) throw new Error('Not a tile');
    const tile = await coreTile(url.pathname);
    if (tile !== null) return served(tile);
    if (MapTileHost.fetch !== null) throw new Error('No tile');
    return fetched(osmTileUrl(address)).catch(() => {
      throw new Error('No tile');
    });
  },
};

export const installQgcTileSource = (): void => {
  addProtocol(QGC_TILE_PROTOCOL, (request) => QgcTileInterceptor.intercept(request));
};
