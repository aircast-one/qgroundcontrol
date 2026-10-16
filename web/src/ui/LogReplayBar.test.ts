import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { logReplay, replayProgress, speedLabel } from './LogReplayBar';

const json = (text: string): Json => parseJson(text) ?? {};

describe('LogReplayBarTest', () => {
  it('the_bar_reads_the_core_replay_state', () => {
    expect(logReplay(json('{"available":false}'))).toBeNull();
    const replay = logReplay(
      json('{"available":true,"shown":true,"loaded":true,"playing":false,"percent":12.5,"playheadTime":"01m:15s","totalTime":"10m:00s","speedIndex":4,"speeds":["0.1","0.25","0.5","1x","2x","5x","10x"],"canLoad":false,"loadRefusal":"x","error":""}'),
    )!;
    expect(replay.speeds[replay.speedIndex]).toBe('2x');
    expect(replay.percent).toBe(12.5);
    expect(replay.playing).toBe(false);
  });

  it('the_bar_reads_its_position_and_speeds_like_the_design', () => {
    const replay = logReplay(json('{"available":true,"shown":true,"playheadTime":"01m:15s","totalTime":"10m:00s","speeds":["0.25","2x"]}'))!;
    expect(replayProgress(replay)).toBe('01m:15s of 10m:00s');
    expect(replayProgress({ ...replay, playheadTime: '', totalTime: '' })).toBe('');
    expect(replay.speeds.map(speedLabel)).toEqual(['0.25×', '2×']);
  });
});
