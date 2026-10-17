import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { promptCopy, vehicleChangePrompt } from './PlanTab';

describe('VehicleChangePromptTest', () => {
  it("the plan view asks only when the core raises it, and reads the aircraft's stored route", () => {
    expect(vehicleChangePrompt(parseJson('{"vehicleChangePrompt":null}'))).toBeNull();
    expect(vehicleChangePrompt(parseJson('{"vehicleChangePrompt":{"connected":true,"dirty":true,"aircraftItems":12}}'))).toEqual({ connected: true, dirty: true, aircraftItems: 12 });
    expect(vehicleChangePrompt(parseJson('{"vehicleChangePrompt":{"connected":true,"dirty":true,"aircraftItems":null}}'))?.aircraftItems).toBeNull();
  });

  it("a route the operator drew is kept by the main button, and the aircraft's route is named", () => {
    const drawn = promptCopy({ connected: true, dirty: true, aircraftItems: 12 });
    expect(drawn).toEqual({
      title: 'Aircraft connected',
      text: 'Keep the route you drew, or replace it with the route stored on the aircraft (12 items)?',
      primary: 'Keep my route',
      secondary: "Load the aircraft's route",
      primaryKeeps: true,
    });
    expect(promptCopy({ connected: true, dirty: false, aircraftItems: null }).primaryKeeps).toBe(false);
    expect(promptCopy({ connected: false, dirty: true, aircraftItems: null }).secondary).toBe('Discard it');
  });
});
