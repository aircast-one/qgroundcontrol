import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { CheckMark, type PreflightCheck, checkMark, checkNeedsTicking, checkStatusText, preflight, preflightSummary } from './Preflight';

const json = (text: string): Json => parseJson(text) ?? {};

const served = `
  {"airframe":"Quadrotor · ArduPilot","total":4,"blocked":["Props on"],"groups":[
    {"name":"Before you fly","checks":[
      {"name":"Props on","prompt":"Propellers fitted and tight","verdict":"failing",
       "reason":"Vehicle reports it is not ready to arm.","blocked":true},
      {"name":"Area clear","prompt":"Nobody within the rotor arc","verdict":"manual",
       "reason":"","blocked":false}]},
    {"name":"Vehicle","checks":[
      {"name":"Battery","prompt":"Pack charged and secured","verdict":"passing",
       "reason":"","blocked":false},
      {"name":"Compass","prompt":"Compass calibrated","verdict":"overridable",
       "reason":"Interference detected.","blocked":false}]}]}
`;

describe('PreflightTest', () => {
  it('the groups and their checks come through with verdicts', () => {
    const checks = preflight(json(served))!;

    expect(checks.groups.map((it) => it.name)).toEqual(['Before you fly', 'Vehicle']);
    expect(checks.groups[0]!.checks.map((it) => it.verdict)).toEqual(['failing', 'manual']);
    expect(checks.blocked).toEqual(['Props on']);
    expect(checks.total).toBe(4);
  });

  it("only a manual check is the operator's to tick", () => {
    const checks = preflight(json(served))!;
    const manual = checks.groups[0]!.checks.find((it) => it.name === 'Area clear')!;
    const passing = checks.groups[1]!.checks.find((it) => it.name === 'Battery')!;

    expect(checkNeedsTicking(manual)).toBe(true);
    expect(checkNeedsTicking(passing)).toBe(false);
  });

  it("a failing check shows the vehicle's reason rather than a generic word", () => {
    const checks = preflight(json(served))!;
    const failing = checks.groups[0]!.checks.find((it) => it.name === 'Props on')!;
    const overridable = checks.groups[1]!.checks.find((it) => it.name === 'Compass')!;

    expect(checkStatusText(failing, false)).toBe('Vehicle reports it is not ready to arm.');
    expect(checkStatusText(overridable, false)).toBe('Interference detected.');
    expect(checkStatusText(checks.groups[1]!.checks[0]!, false)).toBe('Passing');
  });

  it('the summary leads with what would stop the flight', () => {
    const checks = preflight(json(served))!;

    expect(preflightSummary(checks, new Set())).toBe('1 of 4 will stop the flight · 2 left to check.');
  });

  it('the count left to check is out of the checks the operator can tick', () => {
    const checks = { ...preflight(json(served))!, blocked: [] };

    expect(preflightSummary(checks, new Set()), 'an overridable warning is clicked past, as PreFlightCheckButton keeps it pending until then').toBe('2 of 2 left to check.');
  });

  it('ticking every manual check finishes the list the core cannot finish itself', () => {
    const checks = { ...preflight(json(served))!, blocked: [] };

    expect(preflightSummary(checks, new Set(['Area clear', 'Compass']))).toBe('All 4 checks done · 1 warning.');
  });

  it("a blocker does not hide the operator's own progress", () => {
    const checks = preflight(json(served))!;

    expect(preflightSummary(checks, new Set(['Area clear', 'Compass']))).toBe('1 of 4 will stop the flight.');
  });

  it('a check the operator cannot tick is not drawn as an empty box', () => {
    const checks = preflight(json(served))!;
    const byName = new Map<string, PreflightCheck>(checks.groups.flatMap((it) => it.checks).map((it) => [it.name, it]));

    expect(checkMark(byName.get('Area clear')!)).toBe(CheckMark.TICKABLE);
    expect(checkMark(byName.get('Battery')!)).toBe(CheckMark.PASSED);
    expect(checkMark(byName.get('Props on')!)).toBe(CheckMark.ATTENTION);
    expect(checkMark(byName.get('Compass')!)).toBe(CheckMark.TICKABLE);
  });

  it('a tick for something that is not a manual check counts for nothing', () => {
    const checks = { ...preflight(json(served))!, blocked: [] };

    expect(preflightSummary(checks, new Set(['Battery', 'Props on', 'made up']))).toBe('2 of 2 left to check.');
  });

  it('no vehicle is a sentence, not an empty list', () => {
    expect(preflight(null)).toBeNull();
    expect(preflightSummary(null, new Set())).toBe('Connect a vehicle to run its preflight checks.');
  });
});
