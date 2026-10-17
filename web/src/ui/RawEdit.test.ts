import { describe, expect, it } from 'vitest';
import { optDouble, parseJson } from '@/bridge/Json';
import { RAW_EDIT_STUCK, areaHelp, entryPoint, isLandingPattern, rawEdit, rawEditRefusal, vehicleCoordinate, vehicleHeading } from './ItemEditor';

describe('RawEditTest', () => {
  it('only simple items offer show all values', () => {
    expect(rawEdit(parseJson('{"simple":false}'))).toBeNull();
    expect(rawEdit(parseJson('{"simple":true,"rawEdit":true,"friendlyEditAllowed":false}'))).toEqual({ on: true, friendlyAllowed: false });
  });

  it('an item that cannot be shown friendly stays raw', () => {
    expect(rawEditRefusal({ on: true, friendlyAllowed: false })).toBe(RAW_EDIT_STUCK);
    expect(rawEditRefusal({ on: true, friendlyAllowed: true })).toBeNull();
    expect(rawEditRefusal({ on: false, friendlyAllowed: true })).toBeNull();
  });

  it('an unfinished pattern shape shows its help instead of its settings', () => {
    expect(areaHelp(parseJson('{"areaHelp":"Use the Polygon Tools"}'))).toBe('Use the Polygon Tools');
    expect(areaHelp(parseJson('{"areaHelp":null}'))).toBeNull();
  });

  it('a pattern offers its start corner to rotate', () => {
    expect(entryPoint(parseJson('{"entryPoint":{"label":"Start from","value":"top left","path":"p.rotateEntryPoint"}}'))).toEqual({ label: 'Start from', value: 'top left', path: 'p.rotateEntryPoint' });
    expect(entryPoint(parseJson('{"entryPoint":null}'))).toBeNull();
  });

  it('a landing pattern takes the vehicle heading and position', () => {
    expect(isLandingPattern(parseJson('{"landing":true}'))).toBe(true);
    expect(vehicleHeading(parseJson('{"value":87.5}'))).toBe(87.5);
    expect(vehicleHeading(parseJson('{"value":null}'))).toBeNull();
    expect(optDouble(vehicleCoordinate(parseJson('{"latitude":47.4,"longitude":8.5,"valid":true}')), 'latitude')).toBe(47.4);
    expect(vehicleCoordinate(parseJson('{"latitude":0,"longitude":0,"valid":false}'))).toBeNull();
  });
});
