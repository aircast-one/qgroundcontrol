import { describe, expect, it } from 'vitest';
import { apmFollow, headingOfTap, oneDecimal, text } from './ApmFollowScreen';

describe('ApmFollowScreenTest', () => {
  it('a_vehicle_without_follow_parameters_has_no_page_and_offsets_read_with_one_decimal', () => {
    expect(apmFollow(JSON.parse('{"available":false}'))).toBeNull();
    const follow = apmFollow(
      JSON.parse(`{"available":true,"enabled":true,"waiting":false,"supported":true,"showSettings":true,"rover":false,
        "positionOptions":["Maintain Current Offsets","Specify Offsets"],"positionIndex":1,
        "pointOptions":["a","b","c"],"pointIndex":-1,"angle":45.0,"distance":5.0,"height":5.0}`),
    );
    expect(follow).not.toBeNull();
    if (follow === null) return;
    expect(follow.showSettings).toBe(true);
    expect(follow.pointIndex).toBe(-1);
    expect(follow.positionOptions[follow.positionIndex]).toBe('Specify Offsets');
    expect(oneDecimal(follow.angle)).toBe('45.0');
  });

  it('the lead list carries each system id so a pick names the system rather than a row', () => {
    const follow = apmFollow(JSON.parse('{"available":true,"leads":[{"id":255,"name":"Ground station"},{"id":2,"name":"Vehicle 2"}],"leadIndex":1}'));
    expect(follow?.leads).toEqual([
      { id: 255, name: 'Ground station' },
      { id: 2, name: 'Vehicle 2' },
    ]);
    expect(follow === null ? null : follow.leads[follow.leadIndex]?.id).toBe(2);
    expect(apmFollow(JSON.parse('{"available":true}'))?.leadIndex).toBe(-1);
  });

  it('offsets show and type in the app distance units', () => {
    const follow = apmFollow(
      JSON.parse(`{"available":true,"distance":3.048,"height":6.096,"horizontalUnit":"ft","horizontalMetresPerUnit":0.3048,
        "verticalUnit":"ft","verticalMetresPerUnit":0.3048}`),
    );
    expect(follow).not.toBeNull();
    if (follow === null) return;
    expect(text(follow.horizontal, follow.distance)).toBe('10.0 ft');
    expect(text(follow.vertical, follow.height)).toBe('20.0 ft');
    const metric = apmFollow(JSON.parse('{"available":true,"distance":5.0}'));
    expect(metric === null ? null : text(metric.horizontal, 5.0)).toBe('5.0 m');
  });

  it('a tap sets the heading of the vehicle around the ground station', () => {
    expect(headingOfTap(0, 10)).toBeCloseTo(0.0, 9);
    expect(headingOfTap(10, 0)).toBeCloseTo(90.0, 9);
    expect(headingOfTap(0, -10)).toBeCloseTo(180.0, 9);
    expect(headingOfTap(-10, 0)).toBeCloseTo(270.0, 9);
  });
});
