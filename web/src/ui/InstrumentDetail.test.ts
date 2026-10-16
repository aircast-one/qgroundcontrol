import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { osdModeText, osdStatusNote } from './ActiveVehicle';
import {
  BatteryHeadline,
  DetailRow,
  SEVERITY_SECONDARY,
  batteryDetail,
  batteryHeadline,
  batteryReturnOffered,
  gpsDetail,
  gpsStatus,
  linkDetail,
  notYetComputed,
  signalLostTitle,
  silenceText,
  totalCurrent,
  totalDraw,
  usableDop,
} from './InstrumentDetail';
import { BarTone, BatteryLevel, FixLevel, barTone, batteryBarDescription, fixLevel, rcDetail, statusPillShown } from './StatusStrip';
import { LossFailsafe, VehicleLink, VehicleLinks } from './VehicleLinks';

const json = (text: string): Json => parseJson(text) ?? {};

describe('InstrumentDetailTest', () => {
  const row = (label: string, value: string, severity: number = SEVERITY_SECONDARY) => `{"label":"${label}","value":"${value}","severity":${severity}}`;

  const battery = (packs: readonly string[], headline: string = 'null') =>
    json(`{"kind":"object","class":"Battery","available":true,"packs":[${packs.join(',')}],"headline":${headline}}`);

  const pack = (...rows: string[]) => `{"rows":[${rows.join(',')}]}`;

  it('a battery that is not there has no detail', () => {
    expect(batteryDetail(null)).toEqual([]);
    expect(batteryDetail(json('{"kind":"object","available":false,"packs":[]}'))).toEqual([]);
    expect(batteryHeadline(null)).toBeNull();
  });

  it("the core's popup rows are shown in its order with their colouring", () => {
    const rows = batteryDetail(battery([pack(row('Time left', '4:10', 1), row('Charge', '30%', 1), row('Voltage', '11.10 V'))]));
    expect(rows.map((it) => it.label)).toEqual(['Time left', 'Charge', 'Voltage']);
    expect(rows.map((it) => it.severity)).toEqual([1, 1, SEVERITY_SECONDARY]);
  });

  it('two packs read as one line each, with the limiting one marked', () => {
    const view = battery([pack(row('Charge', '90%'), row('Voltage', '12.60 V')), pack(row('Charge', '79%', 1), row('Voltage', '12.30 V'))], '{"text":"79%","detail":"","severity":0,"index":1}');
    expect(batteryDetail(view)).toEqual([
      DetailRow({ label: 'Battery 1', value: '90% · 12.60 V', severity: 0 }),
      DetailRow({ label: 'Battery 2 · lowest', value: '79% · 12.30 V', severity: 1 }),
    ]);
  });

  it('the headline is the limiting pack the core picked, with its level and failsafe margin', () => {
    const view = battery([pack()], '{"text":"Critical","detail":"1:30 left","severity":2,"level":"critical","margin":"Returns home at 7%","index":0}');
    expect(batteryHeadline(view)).toEqual(BatteryHeadline({ text: 'Critical', detail: '1:30 left', severity: 2, level: BatteryLevel.Critical, margin: 'Returns home at 7%', index: 0 }));
  });

  it('the bar turns amber at the turn-back point and red in the reserve', () => {
    const view = battery([pack()], '{"text":"41%","detail":"","severity":0,"percent":41.0,"timeLeft":"6:09","reserve":7.0,"returnAt":26.0,"returnNow":false}');
    const calm = batteryHeadline(view)!;
    expect([calm.percent, calm.timeLeft, calm.returnAt]).toEqual([41.0, '6:09', 26.0]);
    expect(barTone(calm)).toBe(BarTone.Fine);
    expect(barTone({ ...calm, percent: 25.0, returnNow: true })).toBe(BarTone.ReturnNow);
    expect(barTone({ ...calm, percent: 6.0, returnNow: true })).toBe(BarTone.Reserve);
    expect(barTone({ ...calm, percent: 12.0, level: BatteryLevel.Warning }), 'a pack at the red battery threshold tints the bar above the reserve too').toBe(BarTone.Low);
    expect(batteryBarDescription(calm)).toBe('Battery 41%, return home needed at 26%');
  });

  it('silence reads in seconds, then minutes', () => {
    expect(silenceText(12)).toBe('No data from the aircraft for 12 s');
    expect(silenceText(150)).toBe('No data from the aircraft for 2 min');
  });

  it('the vehicle chip says the signal is lost and for how long', () => {
    expect(signalLostTitle(12)).toBe('Signal lost · 12 s');
    expect(osdModeText(signalLostTitle(12))).toBe('Signal lost');
    expect(osdStatusNote(signalLostTitle(12))).toBe('12 s');
    expect(signalLostTitle(null)).toBe('Signal lost');
  });

  it('the chip counts down to the lost-link failsafe, then names it', () => {
    const home = LossFailsafe('Return home', 10.0);
    expect(signalLostTitle(7, home)).toBe('Signal lost · 7 s · Return home in 3 s');
    expect(signalLostTitle(12, home)).toBe('Signal lost · 12 s · Return home');
    expect(signalLostTitle(7, LossFailsafe('No failsafe', null))).toBe('Signal lost · 7 s · No failsafe');
    expect(signalLostTitle(2, home, true), 'the portrait chip keeps the countdown and shortens the rest').toBe('Lost 2 s · Return home in 8 s');
  });

  it('every placeholder QGC prints for an uncomputed fact is treated as one', () => {
    expect(notYetComputed('--.--')).toBe(true);
    expect(notYetComputed('--:--:--')).toBe(true);
    expect(notYetComputed('--')).toBe(true);
    expect(notYetComputed(' ')).toBe(true);
    expect(notYetComputed('0')).toBe(false);
    expect(notYetComputed('11.10')).toBe(false);
  });

  it('a dilution the receiver never computed is not a precision claim', () => {
    expect(usableDop('--.--')).toBe(false);
    expect(usableDop('')).toBe(false);
    expect(usableDop('0')).toBe(false);
    expect(usableDop('99999')).toBe(false);
    expect(usableDop('1.4')).toBe(true);
  });

  it('gps detail is the lock plus the rows the core served', () => {
    const view = json(
      '{"kind":"object","available":true,"satellites":11,"lock":3,"lockText":"3D Lock","rows":[{"label":"Satellites","value":"11"},{"label":"HDOP","value":"1.4"},{"label":"","value":"x"}]}',
    );
    const gps = gpsStatus(view);
    expect(gps?.satellites).toBe(11);
    expect(fixLevel(gps!.lock)).toBe(FixLevel.Good);
    expect(gpsDetail(gps).map((it) => it.label)).toEqual(['GPS Lock', 'Satellites', 'HDOP']);
    expect(gpsDetail(gps)[0]!.value).toBe('3D Lock');
  });

  it('no vehicle and no lock read as nothing rather than a fix', () => {
    expect(gpsStatus(json('{"kind":"object","available":false,"rows":[]}'))).toBeNull();
    const unlocked = gpsStatus(json('{"kind":"object","available":true,"satellites":null,"lock":null,"rows":[]}'));
    expect(fixLevel(unlocked!.lock)).toBeNull();
    expect(unlocked!.satellites).toBeNull();
    expect(gpsDetail(unlocked)).toEqual([]);
  });

  it("the lock row shows the core's lock text, so RTK states read as GPSIndicatorPage spells them", () => {
    const rtk = gpsStatus(json('{"kind":"object","available":true,"satellites":20,"lock":6,"lockText":"RTK Fixed","rows":[]}'));
    expect(gpsDetail(rtk)).toEqual([DetailRow({ label: 'GPS Lock', value: 'RTK Fixed' })]);
    expect(gpsDetail(null).map((it) => it.label)).toEqual([]);
  });

  it('the carrying link is named as the one in use', () => {
    const links = VehicleLinks({ available: true, links: [VehicleLink(false), VehicleLink(true)] });
    const rows = linkDetail(links, ['Telemetry', 'WiFi'], 'Telemetry');
    expect(rows.map((it) => it.label)).toEqual(['Telemetry', 'WiFi']);
    expect(rows.map((it) => it.value)).toEqual(['carrying', 'no contact']);
  });

  it('a link that is neither primary nor lost is standing by, not blank', () => {
    const links = VehicleLinks({ available: true, links: [VehicleLink(false), VehicleLink(false)] });
    expect(linkDetail(links, ['Telemetry', 'WiFi'], 'Telemetry').map((it) => it.value)).toEqual(['carrying', 'standing by']);
  });

  it('the RC sheet reads the same words as the RC cell', () => {
    expect(rcDetail(null)).toEqual([]);
  });

  it('the status pill hides only when nothing would draw in it', () => {
    expect(statusPillShown(false, false, false)).toBe(false);
    expect(statusPillShown(false, false, true)).toBe(true);
    expect(statusPillShown(true, false, false)).toBe(true);
  });
});

describe('BatteryReturnTest', () => {
  const battery = (state: number, power: string, current: string) =>
    json(`{"available":true,"packs":[{"chargeState":${state},"facts":[{"name":"instantPower","value":${power}},{"name":"current","value":${current}}]}]}`);

  it('a critical or worse pack offers Return', () => {
    expect(batteryReturnOffered(battery(3, '120', '10'))).toBe(true);
    expect(batteryReturnOffered(battery(6, '120', '10'))).toBe(true);
    expect(batteryReturnOffered(battery(2, '120', '10'))).toBe(false);
    expect(batteryReturnOffered(null)).toBe(false);
  });

  it('total draw sums watts, else amps', () => {
    expect(totalDraw(battery(1, '119.6', '10'))).toBe('120W');
    expect(totalDraw(battery(1, 'null', '10'))).toBe('10.0A');
    expect(totalDraw(battery(1, 'null', 'null'))).toBeNull();
  });

  it('the strip shows amps even when watts are known', () => {
    expect(totalCurrent(battery(1, '119.6', '10'))).toBe('10.0A');
    expect(totalCurrent(battery(1, '119.6', 'null'))).toBeNull();
    expect(totalCurrent(null)).toBeNull();
  });
});
