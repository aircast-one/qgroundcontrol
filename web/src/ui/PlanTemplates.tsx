import { type Json, jsonStrings, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { AlertDialog, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { md } from '@/theme/Colors';

export const CREATE_FROM_TEMPLATE = 'plan.createFromTemplate';
const BLANK_LABEL = 'Blank mission';

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

export const NewPlanDialog = ({ state, replacing, onDismiss, onPick }: { state: PlanTemplatesState | null; replacing: boolean; onDismiss: () => void; onPick: (template: string | null) => void }) => (
  <AlertDialog
    onDismissRequest={onDismiss}
    title="New plan"
    text={
      <div className="flex flex-col">
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {replacing ? 'Replaces the plan you are editing. Templates start at the map centre.' : 'Templates start at the map centre.'}
        </Text>
        {templateChoices(state).map(([name, label]) => (
          <TextButton key={name ?? ''} className="self-start" enabled={name === null || state?.enabled === true} onClick={() => onPick(name)}>
            {label}
          </TextButton>
        ))}
      </div>
    }
    dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
  />
);
