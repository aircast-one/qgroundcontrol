import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { insertOutcome } from './MissionKinds';
import { kindLabel, missionKinds } from './PlanBridge';

describe('MissionKindsTest', () => {
  it('an accepted insert carries no complaint', () => {
    const done = insertOutcome({ ok: true, inserted: 'survey', atSequence: 4 });
    expect(done.ok).toBe(true);
    expect(done.reason).toBe('');
  });

  it("a refusal is shown in the core's words", () => {
    const refused = insertOutcome({ ok: false, reason: 'The mission already takes off before this point.', refused: 'takeoff' });
    expect(refused.ok).toBe(false);
    expect(refused.reason).toBe('The mission already takes off before this point.');
  });

  it('a kind the catalogue does not hold says so rather than going quiet', () => {
    const unknown = insertOutcome({ ok: false, unknown: 'corkscrew', reason: 'the core has no corkscrew in its catalogue' });
    expect(unknown.ok).toBe(false);
    expect(unknown.reason).toBe('the core has no corkscrew in its catalogue');
  });

  it('a refusal with no reason still says something', () => {
    expect(insertOutcome({ ok: false }).reason).toBe('The plan did not answer.');
    expect(insertOutcome(null).reason).toBe('The plan did not answer.');
  });
});

describe('InsertedIndexTest', () => {
  it('a successful insert says where the item landed, so the next one can follow it', () => {
    expect(insertOutcome({ ok: true, inserted: 'waypoint', index: 3 }).index).toBe(3);
  });

  it('an answer without an index does not invent one', () => {
    expect(insertOutcome({ ok: true }).index).toBeNull();
  });

  it('a refusal carries no index', () => {
    expect(insertOutcome({ ok: false, reason: 'no' }).index).toBeNull();
  });
});

describe('RemoveRefusalTest', () => {
  const outcome = (json: Json) => insertOutcome(json);

  it("a refusal carries the core's sentence rather than a generic failure", () => {
    const refused = outcome({ ok: false, reason: "The first entry holds the plan's own settings and cannot be removed." });
    expect(refused.reason).toBe("The first entry holds the plan's own settings and cannot be removed.");
  });

  it('a removal that happened carries no reason to show', () => {
    const done = outcome({ ok: true, removed: 3, remaining: 6 });
    expect(done.ok).toBe(true);
    expect(done.reason).toBe('');
  });

  it('an answer with no reason still says something', () => {
    expect(outcome({ ok: false }).reason).toBe('The plan did not answer.');
    expect(insertOutcome(null).reason).toBe('The plan did not answer.');
  });

  it('the land button carries the title the core serves', () => {
    const kinds = missionKinds({ kinds: [{ id: 'land', title: 'Return', enabled: true }] });
    expect(kindLabel(kinds, 'land')).toBe('Return');
    expect(kindLabel([], 'land')).toBe('Land');
  });
});
