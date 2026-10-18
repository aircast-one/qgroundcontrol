import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { imagesTakenText, missionComplete } from './MissionCompleteDialog';

const json = (text: string): Json => parseJson(text) ?? {};

describe('MissionCompleteDialogTest', () => {
  it('an open notice carries what the dialog offers', () => {
    const notice = missionComplete(json('{"open":true,"id":3,"imagesTaken":12,"resumeFromWaypoint":4,"batteryWarning":true}'))!;
    expect(notice.id).toBe(3);
    expect(notice.resumeFromWaypoint).toBe(4);
    expect(notice.batteryWarning).toBe(true);
    expect(imagesTakenText(notice.imagesTaken)).toBe('12 Images Taken');
  });

  it('a closed notice or no photos shows nothing', () => {
    expect(missionComplete(json('{"open":false,"id":3}'))).toBeNull();
    expect(missionComplete(json('{"open":true,"id":1,"resumeFromWaypoint":null}'))!.resumeFromWaypoint).toBeNull();
    expect(imagesTakenText(0)).toBeNull();
  });

  it('an interrupted mission says so and names where it resumes', () => {
    const notice = missionComplete(json('{"open":true,"id":2,"imagesTaken":0,"resumeFromWaypoint":5,"batteryWarning":true,"interrupted":true,"title":"Mission interrupted","detail":"2 drops are still to come.","resumeLabel":"Resume from waypoint 4"}'))!;
    expect([notice.interrupted, notice.title, notice.resumeLabel, notice.resumeFromWaypoint]).toEqual([true, 'Mission interrupted', 'Resume from waypoint 4', 5]);
    expect(missionComplete(json('{"open":true,"id":3,"imagesTaken":0,"resumeFromWaypoint":null,"batteryWarning":false}'))!.title).toBe('Flight plan complete');
  });
});
