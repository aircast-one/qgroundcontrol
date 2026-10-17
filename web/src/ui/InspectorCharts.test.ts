import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { chartPoint, chartToggleEnabled, inspectorCharts, latestValue } from './InspectorCharts';

const view = parseJson(
  `{"timeScales":["5 Sec","10 Sec"],"ranges":["Auto","1"],"selectedCharted":[{"field":"roll","chart":0}],
   "charts":[{"rangeX":0,"rangeY":0,"windowMs":5000,"yMin":-1,"yMax":1,"room":true,"plots":[{"label":"ATTITUDE.roll","field":"roll","colour":0,"points":[[2000,0.5]]}]},
             {"rangeX":1,"rangeY":1,"windowMs":10000,"yMin":null,"yMax":null,"room":false,"plots":[]}]}`,
);

describe('InspectorChartsTest', () => {
  it('charts parse with their plots and which fields are charted', () => {
    const charts = inspectorCharts(view);
    if (charts === null) throw new Error('parsed');
    expect(charts.timeScales).toEqual(['5 Sec', '10 Sec']);
    expect(charts.charts[0]?.plots[0]?.points).toEqual([[2000, 0.5]]);
    expect(charts.charted).toEqual(new Map([['roll', 0]]));
    expect(charts.charts[1]?.yMin).toBeNull();
  });

  it('a field charts once, text never, and a full chart takes no more', () => {
    const charts = inspectorCharts(view);
    expect(chartToggleEnabled(charts, 'roll', 'float', 0)).toBe(true);
    expect(chartToggleEnabled(charts, 'roll', 'float', 1)).toBe(false);
    expect(chartToggleEnabled(charts, 'text', 'char[50]', 0)).toBe(false);
    expect(chartToggleEnabled(charts, 'pitch', 'float', 0)).toBe(true);
    expect(chartToggleEnabled(charts, 'pitch', 'float', 1)).toBe(false);
  });

  it('a sample lands by its age and value', () => {
    expect(chartPoint(2000, 0.5, 5000, -1.0, 1.0)).toEqual([0.6, 0.25]);
    expect(chartPoint(0, -5.0, 5000, -1.0, 1.0)).toEqual([1, 1]);
  });

  it('the legend reads the newest sample, and nothing before one arrives', () => {
    expect(
      latestValue([
        [900, 0.5],
        [40, 0.02],
        [400, -1.0],
      ]),
    ).toBe('0.02000');
    expect(latestValue([])).toBeNull();
  });
});
