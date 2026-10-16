import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { MODE_REPLY_MS, ModeOutcome, flightModesView, modeAck, modeHeading, modeOutcome, pinsAfter, startsSection } from './FlightModes';
import { flightModeIcon } from './VehicleUi';

const json = (text: string): Json => parseJson(text) ?? {};

describe('FlightModesTest', () => {
  const served = `
        {"available":true,"canSet":true,"current":"Stabilize",
         "currentSummary":"You fly it; it keeps itself level.",
         "modes":[
           {"name":"Stabilize","summary":"You fly it; it keeps itself level.","current":true,
            "advanced":false,"needsConfirm":false},
           {"name":"Loiter","summary":"Holds position.","current":false,
            "advanced":false,"needsConfirm":false},
           {"name":"Acro","summary":"Rate mode, no self-levelling.","current":false,
            "advanced":true,"needsConfirm":false}]}
    `;

  it('every mode the vehicle offers is listed in its order', () => {
    expect(flightModesView(json(served))!.all.map((it) => it.name)).toEqual(['Stabilize', 'Loiter', 'Acro']);
  });

  it('the current mode is marked so the list can tick it', () => {
    const modes = flightModesView(json(served))!;

    expect(modes.current).toBe('Stabilize');
    expect(modes.all.find((it) => it.name === 'Stabilize')!.current).toBe(true);
  });

  it('the confirm flag is read from the mode, not guessed from its name', () => {
    const flying = flightModesView(
      json(`{"available":true,"canSet":true,"current":"Loiter",
                "modes":[
                  {"name":"RTL","summary":"Flies home.","current":false,
                   "advanced":false,"needsConfirm":true},
                  {"name":"Loiter","summary":"Holds position.","current":true,
                   "advanced":false,"needsConfirm":false}]}`),
    )!;

    expect(flying.all.find((it) => it.name === 'RTL')!.needsConfirm).toBe(true);
    expect(flying.all.find((it) => it.name === 'Loiter')!.needsConfirm).toBe(false);
  });

  it("the popover lists the core's quick modes with their cautions", () => {
    const modes = flightModesView(
      json(`{"available":true,"canSet":true,"current":"Stabilize",
                "pinnedSetting":"settings.flightModeSettings.apmPinnedFlightModesFixedWing",
                "quick":[
                  {"name":"Cruise","quick":true,"caution":""},
                  {"name":"Loiter","quick":true,"caution":"Needs GPS"}],
                "modes":[
                  {"name":"Stabilize","current":true},
                  {"name":"Cruise","quick":true},
                  {"name":"Loiter","quick":true,"caution":"Needs GPS"}]}`),
    )!;

    expect(modes.quick.map((it) => it.name)).toEqual(['Cruise', 'Loiter']);
    expect(modes.quick.at(-1)!.caution).toBe('Needs GPS');
    expect(modes.all.map((it) => it.quick)).toEqual([false, true, true]);
    expect(modes.pinnedSetting).toBe('settings.flightModeSettings.apmPinnedFlightModesFixedWing');
  });

  it("the core's reason for refusing every mode change is carried to the menu", () => {
    const refused = flightModesView(json('{"available":true,"canSet":false,"cannotSetNotice":"This vehicle does not accept a flight mode change from here."}'))!;
    expect(refused.cannotSetNotice).toBe('This vehicle does not accept a flight mode change from here.');
    expect(flightModesView(json('{"available":true,"canSet":true}'))!.cannotSetNotice).toBe('');
  });

  it('a vehicle reporting no modes offers no picker', () => {
    expect(flightModesView(null)).toBeNull();
    expect(flightModesView(json('{"available":false}'))).toBeNull();
  });
});

describe('UnknownModeTest', () => {
  it("the core's unknown-mode line is carried to the menu", () => {
    const view = json('{"available":true,"current":"Custom 7","unknownModeNotice":"The vehicle is in Custom 7, which this version of the app doesn\'t know. Choose a mode below to change it."}');
    expect(flightModesView(view)!.unknownModeNotice.startsWith('The vehicle is in Custom 7')).toBe(true);
    expect(flightModesView(json('{"available":true}'))!.unknownModeNotice).toBe('');
  });
});

describe('ModeHeadingTest', () => {
  const modes = (current: string, summary: string) =>
    flightModesView(
      json(`{"kind":"object","class":"FlightModes","available":true,"canSet":true,"current":${current},
               "currentSummary":${summary}}`),
    );

  it('the heading names the mode and what it does', () => {
    expect(modeHeading(modes('"Stabilize"', '"You fly it by hand, it only levels itself"'))).toBe('Stabilize — You fly it by hand, it only levels itself');
  });

  it('a mode the core has no sentence for gets no heading rather than a dangling dash', () => {
    expect(modeHeading(modes('"Stabilize"', '""'))).toBeNull();
    expect(modeHeading(modes('""', '"Something"'))).toBeNull();
    expect(modeHeading(null)).toBeNull();
  });

  it('pinning starts from the quick list on show and adds or drops one mode', () => {
    expect(pinsAfter(['Cruise', 'FBW A'], 'Acro', true)).toBe('Cruise,FBW A,Acro');
    expect(pinsAfter(['Cruise', 'FBW A'], 'Cruise', false)).toBe('FBW A');
    expect(pinsAfter(['Cruise'], 'Cruise', false)).toBe('');
    expect(pinsAfter(['Cruise'], 'Cruise', true)).toBe('Cruise');
  });

  it('each mode in the menu carries an icon like the Penpot mode picker', () => {
    expect(['Stabilize', 'Altitude Hold', 'Position Hold', 'Auto', 'Smart RTL', 'Circle'].map(flightModeIcon)).toEqual(['ic_gamepad', 'ic_height', 'ic_my_location', 'ic_route', 'ic_home', 'ic_flight']);
  });

  it('a divider starts the return modes like the QGC mode sections', () => {
    const modesList = flightModesView(
      json(`{"available":true,"modes":[
                {"name":"Loiter","section":"normal"},{"name":"Guided"},
                {"name":"RTL","section":"return"},{"name":"Land","section":"return"}]}`),
    )!.all;

    expect(modesList.map((_, index) => startsSection(modesList, index))).toEqual([false, false, true, false]);
  });
});

describe('ModeOutcomeTest', () => {
  const earlier = { serial: 4, accepted: true };

  it('a mode change settles, or stops waiting on a refusal or silence, which the core words', () => {
    expect(modeOutcome(earlier, earlier, true, 100)).toBe(ModeOutcome.Settled);
    expect(modeOutcome(earlier, earlier, false, 100)).toBe(ModeOutcome.Pending);
    expect(modeOutcome(earlier, { serial: 5, accepted: false }, false, 100)).toBe(ModeOutcome.Unsettled);
    expect(modeOutcome({ serial: 5, accepted: false }, { serial: 5, accepted: false }, false, 100), 'an old refusal is not this one').toBe(ModeOutcome.Pending);
    expect(modeOutcome(null, null, false, MODE_REPLY_MS)).toBe(ModeOutcome.Unsettled);
  });

  it("the core's ack reads into the head", () => {
    expect(modeAck(json('{"modeAck":{"serial":2,"accepted":false,"wording":"refused for now"}}'))).toEqual({ serial: 2, accepted: false });
    expect(modeAck(json('{"modeAck":null}'))).toBeNull();
  });
});
