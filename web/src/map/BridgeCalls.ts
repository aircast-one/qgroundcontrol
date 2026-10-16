import { call } from '@/core/client';
import { optBoolean, parseJson } from '@/bridge/Json';

export const coordinateJson = (latitude: number, longitude: number, altitudeMetres = 0): string =>
  `{"latitude":${latitude},"longitude":${longitude},"altitude":${altitudeMetres}}`;

export const coordinateJsonAt = (at: { latitude: number; longitude: number }): string => coordinateJson(at.latitude, at.longitude);

export const settingJson = (value: string): string => `{"value":${value}}`;

export const setOk = async (path: string, value: string): Promise<boolean> => optBoolean(parseJson(await call<string>('set', path, value)), 'ok');

export const PLAN_UNDO = 'plan.undo';

export const invokeOk = async (path: string, args = '[]'): Promise<boolean> => optBoolean(parseJson(await call<string>('invoke', path, args)), 'ok');
