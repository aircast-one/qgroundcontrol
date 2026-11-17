import { type Json, jsonStrings, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { AlertDialog, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { md } from '@/theme/Colors';

export const CREATE_FROM_TEMPLATE = 'plan.createFromTemplate';
const BLANK_LABEL = 'Blank mission';
export const ZOOM_IN_FOR_TEMPLATE = 'Zoom the map in to where the mission should go. A template fills the area you can see.';

export type PlanTemplatesState = { enabled: boolean; names: readonly string[]; blank: string };

export const planTemplates = (view: Json | null): PlanTemplatesState | null => {
  const templates = optJSONObject(view, 'templates');
  return templates === null
    ? null
    : {
        enabled: optBoolean(templates, 'enabled'),
        names: jsonStrings(optJSONArray(templates, 'names')),
        blank: optText(templates, 'blank'),
      };
};

export const templateChoices = (state: PlanTemplatesState | null): readonly (readonly [string | null, string])[] => [
  [null, BLANK_LABEL],
  ...(state?.names ?? []).filter((it) => it !== state?.blank).map((it) => [it, sentenceCase(it)] as const),
];

export const newPlanNote = (replacing: boolean, zoomedOut: boolean): string =>
  [replacing ? 'Replaces the plan you are editing.' : null, zoomedOut ? ZOOM_IN_FOR_TEMPLATE : 'Templates fill the area you can see on the map.'].filter((it): it is string => it !== null).join(' ');

export const NewPlanDialog = ({
  state,
  replacing,
  zoomedOut,
  onDismiss,
  onPick,
}: {
  state: PlanTemplatesState | null;
  replacing: boolean;
  zoomedOut: boolean;
  onDismiss: () => void;
  onPick: (template: string | null) => void;
}) => (
  <AlertDialog
    onDismissRequest={onDismiss}
    title="New plan"
    text={
      <div className="flex flex-col">
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {newPlanNote(replacing, zoomedOut)}
        </Text>
        {templateChoices(state).map(([name, label]) => (
          <TextButton key={name ?? ''} className="self-start" enabled={name === null || (state?.enabled === true && !zoomedOut)} onClick={() => onPick(name)}>
            {label}
          </TextButton>
        ))}
      </div>
    }
    dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
  />
);
