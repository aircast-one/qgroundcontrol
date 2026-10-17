import { describe, expect, it } from 'vitest';
import { choosesPlanVehicle } from './PlanVehicleRows';

describe('PlanVehicleRowsTest', () => {
  it('the planned vehicle is chosen only with no vehicle and an empty plan', () => {
    expect(choosesPlanVehicle(false, { hasMissionItems: false })).toBe(true);
    expect(choosesPlanVehicle(true, { hasMissionItems: false })).toBe(false);
    expect(choosesPlanVehicle(false, { hasMissionItems: true })).toBe(false);
    expect(choosesPlanVehicle(false, { containsItems: true, hasMissionItems: false }), 'a fence alone leaves the vehicle choosable, as MissionSettingsEditor counts mission items only').toBe(true);
  });
});
