import { describe, expect, it } from 'vitest';
import { factNumber, sliderSteps, tuningTabs } from './Px4TuningScreen';

describe('Px4TuningScreenTest', () => {
  it('tabs, axes and sliders read from the tuning view', () => {
    const tabs = tuningTabs(
      JSON.parse(
        '{"available":true,"tabs":[{"name":"Rate Controller","title":"Rate","unit":"deg/s","extras":[],"axes":[{"name":"Roll","params":[' +
          '{"title":"Overall Multiplier (MC_ROLLRATE_K)","description":"d","param":"MC_ROLLRATE_K","min":0.3,"max":3,"step":0.05,' +
          '"fact":{"class":"Control","path":"vehicle.parameterManager.getParameter(-1,MC_ROLLRATE_K)","name":"MC_ROLLRATE_K","value":1,"valueString":"1.00"}}]}]}]}',
      ),
    );
    expect(tabs).toHaveLength(1);
    expect(tabs[0]?.name).toBe('Rate controller');
    expect(tabs[0]?.axes).toHaveLength(1);
    expect(tabs[0]?.axes[0]?.params).toHaveLength(1);
    const param = tabs[0]?.axes[0]?.params[0];
    expect(param?.fact.name).toBe('MC_ROLLRATE_K');
    expect(param?.title).toBe('Overall multiplier (MC_ROLLRATE_K)');
    expect(param === undefined ? null : factNumber(param.fact)).toBe(1);
    expect(param === undefined ? null : sliderSteps(param.min, param.max, param.step)).toBe(53);
  });
});
