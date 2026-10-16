import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { flyState } from './FlyState';
import {
  BatteryLevel,
  COMPACT_STATUS_CELLS,
  FixLevel,
  batteryLevelOf,
  batteryPercent,
  batteryReadings,
  compactStatusText,
  fixLevel,
  flightTimeSeconds,
  flightTimeText,
  indicatorParameterWait,
  overrideCell,
  packCountText,
  rcCell,
  satsText,
  shownStatusCells,
  telemetryCell,
  telemetryDetail,
} from './StatusStrip';

const json = (text: string): Json => parseJson(text) ?? {};

const toDoubleOrNaN = (text: string): number => (/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(text.trim()) ? Number(text) : Number.NaN);

describe('RcSignalTest', () => {
  const state = ({ rcSupported = true, rcSignal = '72', rcSignalText = '"72%"' }: { rcSupported?: boolean; rcSignal?: string; rcSignalText?: string } = {}) =>
    flyState(
      json(`{"kind":"object","class":"FlyState","connected":true,"contactLost":false,
         "state":"disarmed","stateText":"Disarmed","staleNotice":"","mode":"Stabilize",
         "rcSupported":${rcSupported},"rcSignal":${rcSignal},"rcSignalText":${rcSignalText}}`),
    );

  it('the sentinel the firmware sends for unknown reaches this head as no text at all', () => {
    expect(rcCell(state({ rcSignal: 'null', rcSignalText: 'null' }))).toBeNull();
  });

  it('zero is a reading the vehicle chose to send and says the link is dead', () => {
    const cell = rcCell(state({ rcSignal: '0', rcSignalText: '"No signal"' }));

    expect(cell?.text).toBe('No signal RC');
    expect(cell?.lost).toBe(true);
  });

  it("a real reading is the core's sentence, not a percentage this head formats", () => {
    const cell = rcCell(state());

    expect(cell?.text).toBe('72% RC');
    expect(cell?.lost).toBe(false);
  });

  it('a vehicle with no radio never shows the cell', () => {
    expect(rcCell(state({ rcSupported: false }))).toBeNull();
  });

  it('no vehicle is no cell', () => {
    expect(rcCell(null)).toBeNull();
  });
});

describe('BatteryTextTest', () => {
  it('the level name maps to the ladder this strip already drew', () => {
    expect(batteryLevelOf('normal')).toBe(BatteryLevel.Normal);
    expect(batteryLevelOf('caution')).toBe(BatteryLevel.Caution);
    expect(batteryLevelOf('warning')).toBe(BatteryLevel.Warning);
    expect(batteryLevelOf('critical')).toBe(BatteryLevel.Critical);
  });

  it('an unknown or absent level is not treated as an alarm', () => {
    expect(batteryLevelOf(null)).toBe(BatteryLevel.Normal);
    expect(batteryLevelOf('a level added later')).toBe(BatteryLevel.Normal);
  });

  it('each pack is a cell with its label and the lines the core chose', () => {
    const readings = batteryReadings(
      json(`{"available":true,"level":"warning","indicatorPacks":[
        {"level":"caution","indicatorLabel":"B1","indicatorLines":["70%","11.10V"]},
        {"level":"warning","indicatorLabel":"B2","indicatorLines":["40%"]}]}`),
    );

    expect(readings.map((it) => it.text)).toEqual(['B1 70% · 11.10V', 'B2 40%']);
    expect(readings.map((it) => it.level)).toEqual([BatteryLevel.Caution, BatteryLevel.Warning]);
  });

  it('combined packs read as one ring with the count beside the limiting charge', () => {
    const readings = batteryReadings(json('{"available":true,"indicatorPacks":[{"level":"normal","indicatorLabel":null,"packCount":2,"indicatorLines":["57%"]}]}'));
    expect(readings).toHaveLength(1);
    const reading = readings[0];
    expect(reading?.text).toBe('57%');
    expect(batteryPercent(reading?.text ?? '')).toBe(57);
    expect(packCountText(reading?.packs ?? 0)).toBe('×2');
    expect(packCountText(1)).toBe('');
  });

  it('a single pack carries no label', () => {
    expect(batteryReadings(json('{"available":true,"indicatorPacks":[{"level":"normal","indicatorLabel":null,"indicatorLines":["90%"]}]}')).map((it) => it.text)).toEqual(['90%']);
  });

  it('no battery is no cell rather than an empty one', () => {
    expect(batteryReadings(null)).toEqual([]);
    expect(batteryReadings(json('{"available":false}'))).toEqual([]);
    expect(batteryReadings(json('{"available":true,"indicatorPacks":[{"indicatorLines":[""]}]}'))).toEqual([]);
  });
});

describe('GpsFixTest', () => {
  it('a fix type below 2D is no fix at all', () => {
    expect(fixLevel(0)).toBe(FixLevel.None);
    expect(fixLevel(1)).toBe(FixLevel.None);
  });

  it('2D is told apart from 3D rather than both passing as a fix', () => {
    expect(fixLevel(2)).toBe(FixLevel.TwoD);
    expect(fixLevel(3)).toBe(FixLevel.Good);
    expect(fixLevel(6)).toBe(FixLevel.Good);
  });

  it('an unreadable lock shows nothing rather than claiming a fix', () => {
    expect(fixLevel(Number.NaN)).toBeNull();
  });

  it('a satellite count is not shown as reassurance when there is no fix', () => {
    expect(satsText(FixLevel.None, '11')).toBe('No fix');
  });

  it('a 2D fix says so next to the count', () => {
    expect(satsText(FixLevel.TwoD, '11')).toBe('11 sats · 2D only');
  });

  it('a good fix is just the count', () => {
    expect(satsText(FixLevel.Good, '11')).toBe('11 sats');
  });

  it('the rendered lock text is not a fix level, so wiring the string back in hides the cell', () => {
    expect(fixLevel(toDoubleOrNaN('3D Lock'))).toBeNull();
    expect(fixLevel(toDoubleOrNaN('None'))).toBeNull();
  });
});

describe('SatelliteCellTest', () => {
  it('a sentence does not get the noun appended to it', () => {
    expect(satsText(FixLevel.None, '11')).toBe('No fix');
    expect(satsText(FixLevel.None, '')).toBe('No fix');
  });

  it('a count carries the noun once', () => {
    expect(satsText(FixLevel.Good, '11')).toBe('11 sats');
    expect(satsText(FixLevel.TwoD, '11')).toBe('11 sats · 2D only');
  });

  it("a fix with no count reported reads as GPSIndicator's dashes rather than a bare noun", () => {
    expect(satsText(FixLevel.Good, '')).toBe('--');
    expect(satsText(FixLevel.TwoD, '')).toBe('2D only');
  });
});

describe('IndicatorParameterWaitTest', () => {
  it('battery failsafes wait for parameters like ToolIndicatorPage expandedComponentWaitForParameters', () => {
    expect(indicatorParameterWait(json('{"connected":true,"parametersReady":true,"parametersReason":"incomplete"}'))).toBeNull();
    expect(indicatorParameterWait(json('{"connected":true,"parametersReady":false,"parametersReason":"loading"}'))).toBe('Waiting for parameters…');
    expect(indicatorParameterWait(json('{"connected":true,"parametersReady":false,"parametersReason":"unanswered"}'))).toBe('Waiting for parameters…');
    expect(indicatorParameterWait(json('{"connected":true,"parametersReady":false,"parametersReason":"skipped"}'))).toBe('Parameters not available');
  });
});

describe('CompactStatusTest', () => {
  it('a compact cell keeps only the reading, and an icon alone when there is none', () => {
    expect(compactStatusText('B1 53%')).toBe('53%');
    expect(compactStatusText('12 sats')).toBe('12');
    expect(compactStatusText('12 sats · 2D only')).toBe('12');
    expect(compactStatusText('-70 dBm')).toBe('-70');
    expect(compactStatusText('No signal RC')).toBe('');
  });

  it('a narrow status pill keeps only the battery so nothing is cut in half', () => {
    expect(shownStatusCells(true)).toBe(1);
    expect(shownStatusCells(false)).toBe(COMPACT_STATUS_CELLS);
  });
});

describe('RadioLinkTest', () => {
  const state = (extra: string) =>
    flyState(
      json(`{"kind":"object","class":"FlyState","connected":true,"armed":false,"contactLost":false,
          "state":"ready","stateText":"Ready","staleNotice":"","mode":"Stabilize",
          "rcSupported":true,"rcSignalText":"80%","rcSignal":80,${extra}}`),
    );

  it('a radio that has never spoken is not a link at minus zero', () => {
    expect(state('"rcOverride":false,"telemetry":null')?.telemetry).toBeNull();
    expect(telemetryCell(state('"rcOverride":false,"telemetry":null'))).toBeNull();
    expect(telemetryDetail(null)).toEqual([]);
  });

  it("the strip shows this station's own signal", () => {
    const reading = state(`"rcOverride":false,"telemetry":{"localRssiDbm":-71,"remoteRssiDbm":-68,
           "localNoise":42,"remoteNoise":39,"receiveErrors":7,"errorsFixed":2,"txBuffer":95}`);
    expect(telemetryCell(reading)).toBe('-71 dBm');
    expect(telemetryDetail(reading?.telemetry ?? null).map((it) => it.label)).toEqual(['Local RSSI:', 'Remote RSSI:', 'RX Errors:', 'Errors Fixed:', 'TX Buffer:', 'Local Noise:', 'Remote Noise:']);
  });

  it('a radio reporting only its own end still gives a reading and a shorter detail', () => {
    const reading = state(`"rcOverride":false,"telemetry":{"localRssiDbm":-71,"remoteRssiDbm":null,
           "localNoise":null,"remoteNoise":null,"receiveErrors":null}`);
    expect(telemetryCell(reading)).toBe('-71 dBm');
    expect(telemetryDetail(reading?.telemetry ?? null).map((it) => it.label)).toEqual(['Local RSSI:']);
  });

  it('the override cell appears only while the transmitter is actually overridden', () => {
    expect(overrideCell(state('"rcOverride":true,"telemetry":null'))?.text).toBe('RC override');
    expect(overrideCell(state('"rcOverride":false,"telemetry":null'))).toBeNull();
    expect(overrideCell(state('"rcOverride":null,"telemetry":null')), 'null is no vehicle, and claiming manual control is not overridden would be a safety claim from nothing').toBeNull();
  });
});

describe('DjiChromeTest', () => {
  it('the battery ring reads the percentage out of the cell text', () => {
    expect(batteryPercent('B1 53%')).toBe(53);
    expect(batteryPercent('No battery')).toBeNull();
  });
});

describe('FlightTimeTest', () => {
  it("flight time reads as DJI's minutes and seconds", () => {
    expect(flightTimeText(null)).toBe('00\'00"');
    expect(flightTimeText(0)).toBe('00\'00"');
    expect(flightTimeText(75.4)).toBe('01\'15"');
    expect(flightTimeText(4325)).toBe('72\'05"');
  });
});

describe('FlightTimeReadTest', () => {
  it('the seconds are the value of the fact the core answers with', () => {
    expect(flightTimeSeconds(json('{"kind":"fact","value":10.325}'))).toBe(10.325);
    expect(flightTimeSeconds(json('{"kind":"fact","value":null}'))).toBeNull();
  });
});
