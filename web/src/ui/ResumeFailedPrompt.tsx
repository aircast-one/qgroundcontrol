import { useEffect, useMemo } from 'react';
import { type Json, has, isNull, optInt } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, TextButton } from '@/m3';
import { GUIDED_ACTIONS } from './GuidedActions';

const RESUME_MISSION_PATH = 'planFly.missionController.resumeMission';

export const resumeFailedIndex = (view: Json | null): number | null =>
  view !== null && has(view, 'resumeFailedIndex') && !isNull(view, 'resumeFailedIndex') ? optInt(view, 'resumeFailedIndex') : null;

export const resumeCleared = (view: Json | null): boolean => view !== null && resumeFailedIndex(view) === null;

export const ResumeFailedPrompt = ({ dismissed, onDismissed }: { dismissed: number | null; onDismissed: (dismissed: number | null) => void }) => {
  const actions = useQgcPath(GUIDED_ACTIONS);
  const failed = useMemo(() => resumeFailedIndex(actions), [actions]);
  const cleared = resumeCleared(actions);
  useEffect(() => {
    if (cleared) onDismissed(null);
  }, [cleared]);
  const index = failed !== null && failed !== dismissed ? failed : null;
  if (index === null) return null;
  return (
    <AlertDialog
      onDismissRequest={() => onDismissed(index)}
      title="Resume FAILED"
      text="Upload of resume mission failed. Confirm to retry upload"
      confirmButton={
        <TextButton
          onClick={() => {
            onDismissed(index);
            void Qgc.invoke(RESUME_MISSION_PATH, index);
          }}
        >
          Confirm
        </TextButton>
      }
      dismissButton={<TextButton onClick={() => onDismissed(index)}>Cancel</TextButton>}
    />
  );
};
