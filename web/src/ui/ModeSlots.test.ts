import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { liveSlotText, liveSwitchesText, modeSlotsView } from './ModeSlots';

const json = (text: string): Json => parseJson(text) ?? {};

const view = (...live: number[]): Json =>
  json(`
    {"available": true, "channel": 5, "liveSlot": ${live[0] ?? 0}, "reason": "",
     "slots": [
       {"slot": 1, "mode": "Acro", "live": ${live.includes(1)}},
       {"slot": 2, "mode": "AltHold", "live": ${live.includes(2)}},
       {"slot": 3, "mode": "Auto", "live": ${live.includes(3)}},
       {"slot": 4, "mode": "Guided", "live": ${live.includes(4)}},
       {"slot": 5, "mode": "Loiter", "live": ${live.includes(5)}},
       {"slot": 6, "mode": "RTL", "live": ${live.includes(6)}}
     ]}
    `);

describe('ModeSlotsTest', () => {
  it('the live slot is named by its number and its mode, which is what the page exists to answer', () => {
    expect(liveSlotText(modeSlotsView(view(4)))).toBe('The switch on channel 5 is on slot 4, Guided.');
  });

  it('a switch sitting between the bands says so rather than naming a slot', () => {
    expect(liveSlotText(modeSlotsView(view()))).toBe('The switch on channel 5 is not on any mode slot.');
  });

  it('an unassigned mode channel says so instead of naming channel 0', () => {
    const unassigned = json(JSON.stringify(view()).replace('"reason":""', '"reason":"No mode channel is assigned."'));
    expect(liveSlotText(modeSlotsView(unassigned))).toBe('No mode channel is assigned.');
  });

  it('a vehicle that does not pick modes from a channel has nothing to show', () => {
    const unavailable = json('{"available": false, "slots": [], "liveSlot": 0}');
    expect(modeSlotsView(unavailable)).toBeNull();
    expect(liveSlotText(null)).toBeNull();
  });

  it("live switches and channel options are named, like the highlighted rows in QGC's flight mode pages", () => {
    const px4 = modeSlotsView(json('{"available":true,"channel":5,"slots":[],"activeSwitches":["Kill switch channel"],"channelOptions":[]}'));
    expect(liveSwitchesText(px4)).toBe('Switches on: Kill switch channel');
    const apm = modeSlotsView(json('{"available":true,"channel":5,"slots":[],"channelOptions":[{"channel":7,"enabled":true},{"channel":8,"enabled":false}]}'));
    expect(liveSwitchesText(apm)).toBe('Channel options on: channel 7');
    expect(liveSwitchesText(modeSlotsView(json('{"available":true,"channel":5,"slots":[]}')))).toBeNull();
  });

  it("the channel monitor is shown where the core says so, PX4's flight modes page", () => {
    expect(modeSlotsView(json('{"available":true,"channel":5,"slots":[],"channelMonitor":true}'))?.channelMonitor).toBe(true);
    expect(modeSlotsView(json('{"available":true,"channel":5,"slots":[]}'))?.channelMonitor).toBe(false);
  });

  it('the rows to paint are the ones the core names', () => {
    const named = modeSlotsView(json('{"available":true,"activeParams":["FLTMODE4","RC7_OPTION"]}'))!;
    expect(named.activeParams).toEqual(new Set(['FLTMODE4', 'RC7_OPTION']));
    expect(modeSlotsView(json('{"available":true}'))!.activeParams).toEqual(new Set());
  });
});
