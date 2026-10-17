import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { createDocument, openDocument } from '@/kotlin/Platform';
import { AlertDialog, Checkbox, DropdownMenu, DropdownMenuItem, Text, TextButton } from '@/m3';
import { aircast, md } from '@/theme/Colors';
import { useKeyedState } from '@/kotlin/Compose';

export const PARAMETER_TOOLS_PATH = 'view.parameterTools';
export const PARAMETER_REFRESH_PATH = 'parameterTools.refresh';
export const PARAMETER_SAVE_PATH = 'parameterFile.save';
export const PARAMETER_REVIEW_PATH = 'parameterFile.review';
export const PARAMETER_APPLY_PATH = 'parameterFile.apply';
const PARAMETER_FILE_NAME = 'vehicle.params';
const PARAMETER_FILE_TYPES = ['*/*'];

export type ParameterDiffRow = {
  json: Json;
  name: string;
  fileValue: string;
  vehicleValue: string;
  units: string;
  cannotSend: boolean;
  componentId: number;
  noVehicleValue: boolean;
};

export const ParameterDiffRow = (fields: Omit<ParameterDiffRow, 'componentId' | 'noVehicleValue'> & { componentId?: number; noVehicleValue?: boolean }): ParameterDiffRow => ({
  componentId: 1,
  noVehicleValue: false,
  ...fields,
});

export const diffRowKey = (row: ParameterDiffRow): string => `${row.componentId}:${row.name}`;

export type ParameterReview = {
  rows: readonly ParameterDiffRow[];
  otherVehicle: boolean;
  multipleComponents: boolean;
  parsed: number;
  unchanged: number;
  readOnly: number;
};

export const ParameterReview = (fields: { rows: readonly ParameterDiffRow[]; otherVehicle: boolean; multipleComponents: boolean; parsed?: number; unchanged?: number; readOnly?: number }): ParameterReview => ({
  parsed: fields.rows.length,
  unchanged: 0,
  readOnly: 0,
  ...fields,
});

export const parameterReview = (result: Json | null): ParameterReview | null =>
  result === null
    ? null
    : ParameterReview({
        rows: jsonObjects(optJSONArray(result, 'rows')).map((it) =>
          ParameterDiffRow({
            json: it,
            name: optText(it, 'name'),
            fileValue: optText(it, 'fileValue'),
            vehicleValue: optText(it, 'vehicleValue'),
            units: optText(it, 'units'),
            cannotSend: optBoolean(it, 'cannotSend'),
            componentId: optInt(it, 'componentId', 1),
            noVehicleValue: optBoolean(it, 'noVehicleValue'),
          }),
        ),
        otherVehicle: optBoolean(result, 'otherVehicle'),
        multipleComponents: optBoolean(result, 'multipleComponents'),
        parsed: optInt(result, 'parsed'),
        unchanged: optInt(result, 'unchanged'),
        readOnly: optInt(result, 'readOnly'),
      });

export const sendableCount = (review: ParameterReview): number => review.rows.filter((it) => !it.cannotSend).length;

export const reviewSummary = (review: ParameterReview): string => {
  const sendable = sendableCount(review);
  const newToVehicle = review.rows.filter((it) => it.noVehicleValue && !it.cannotSend).length;
  const missing = review.rows.filter((it) => it.cannotSend).length;
  const clauses = [
    sendable === 0 ? null : newToVehicle > 0 ? `${sendable} will be changed (including ${newToVehicle} not currently on the Vehicle)` : `${sendable} will be changed`,
    review.unchanged === 1 ? '1 already matches the Vehicle' : review.unchanged > 1 ? `${review.unchanged} already match the Vehicle` : null,
    review.readOnly === 1 ? '1 read-only parameter will not be sent' : review.readOnly > 1 ? `${review.readOnly} read-only parameters will not be sent` : null,
    missing > 0 ? `${missing} not found on the Vehicle and cannot be sent` : null,
  ].filter((it): it is string => it !== null);
  const loaded = review.parsed === 1 ? 'Loaded 1 parameter from file' : `Loaded ${review.parsed} parameters from file`;
  return clauses.length === 0 ? `${loaded}.` : `${loaded}: ${clauses.join(', ')}.`;
};

export const SEND_HINT = "Click 'Ok' to update the parameters below on the Vehicle.";
export const CANNOT_SEND_HINT =
  "Parameters marked 'not on Vehicle' cannot be sent since the file does not include type information. They may only exist after another parameter is changed and the Vehicle is rebooted, after which you can load this file again.";

export const checkedAll = (review: ParameterReview, chosen: ReadonlySet<string>, on: boolean): Set<string> => {
  const sendable = new Set(review.rows.filter((it) => !it.cannotSend).map(diffRowKey));
  return on ? new Set([...chosen, ...sendable]) : new Set([...chosen].filter((it) => !sendable.has(it)));
};

export const reviewWarnings = (review: ParameterReview): string[] =>
  [
    review.otherVehicle ? 'The parameters in the file are from a different vehicle.' : null,
    review.multipleComponents ? 'The file contains parameters for more than one component.' : null,
  ].filter((it): it is string => it !== null);

export type ParameterTool = { path: string; label: string; confirm: boolean; confirmTitle: string; confirmMessage: string };

export const parameterTools = (view: Json | null): ParameterTool[] =>
  jsonObjects(optJSONArray(view, 'tools')).map((it) => ({
    path: optText(it, 'path'),
    label: optText(it, 'label'),
    confirm: optBoolean(it, 'confirm'),
    confirmTitle: optText(it, 'confirmTitle'),
    confirmMessage: optText(it, 'confirmMessage'),
  }));

export const NEW_TO_VEHICLE_HINT = "Parameters marked 'new to Vehicle' have not been reported by the Vehicle. They may only become visible after they are sent and the Vehicle is rebooted.";

const notBlank = (text: string): boolean => text.trim() !== '';

export const diffLine = (row: ParameterDiffRow): string => {
  if (row.cannotSend) return ['Vehicle N/A — not on Vehicle', `File ${row.fileValue}`].join(' · ');
  if (row.noVehicleValue) return ['Vehicle N/A — new to Vehicle', `File ${row.fileValue}`, row.units].filter(notBlank).join(' · ');
  return [`Vehicle ${row.vehicleValue}`, `File ${row.fileValue}`, row.units].filter(notBlank).join(' · ');
};

export const confirmLabel = (tool: ParameterTool): string => (tool.confirmTitle === 'Reset All' ? 'Reset' : 'Ok');

export const ParameterToolsMenu = ({ onRefreshed }: { onRefreshed: () => void }) => {
  const view = useQgcPath(PARAMETER_TOOLS_PATH);
  const tools = parameterTools(view);
  const [open, setOpen] = useState(false);
  const [asking, setAsking] = useState<ParameterTool | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [review, setReview] = useState<ParameterReview | null>(null);
  const [chosen, setChosen] = useState<ReadonlySet<string>>(new Set());

  const save = async () => {
    const saved = await Qgc.invokeResult(PARAMETER_SAVE_PATH);
    if (typeof saved !== 'string') {
      setRefusal('The parameters could not be read from the vehicle.');
      return;
    }
    try {
      createDocument(PARAMETER_FILE_NAME, 'text/plain', saved);
      setRefusal(null);
    } catch {
      setRefusal('The file could not be written.');
    }
  };

  const load = async () => {
    const source = await openDocument(PARAMETER_FILE_TYPES);
    if (source === null) return;
    const text = await source.text().catch(() => null);
    if (text === null) {
      setRefusal('Unable to open file.');
      return;
    }
    const reviewed = await Qgc.call(PARAMETER_REVIEW_PATH, text);
    const parsed = parameterReview(optJSONObject(reviewed, 'result'));
    if (parsed === null) {
      const reason = optText(reviewed, 'reason');
      setRefusal(notBlank(reason) ? reason : 'The file could not be reviewed.');
      return;
    }
    setReview(parsed);
    setChosen(new Set(parsed.rows.filter((it) => !it.cannotSend).map(diffRowKey)));
  };

  const run = (tool: ParameterTool) => {
    if (tool.path === PARAMETER_SAVE_PATH) return void save();
    if (tool.path === PARAMETER_REVIEW_PATH) return void load();
    void (async () => {
      const refused = await Qgc.refusalOf(tool.path);
      setRefusal(refused);
      if (refused === null) onRefreshed();
    })();
  };

  const apply = (shown: ParameterReview) => {
    const rows = shown.rows.filter((it) => chosen.has(diffRowKey(it))).map((it) => it.json);
    setReview(null);
    void (async () => {
      setRefusal(await Qgc.refusalOf(PARAMETER_APPLY_PATH, rows));
      onRefreshed();
    })();
  };

  if (tools.length === 0) return null;
  return (
    <>
      <div className="relative">
        <TextButton onClick={() => setOpen(true)}>Tools</TextButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {tools.map((tool) => (
            <DropdownMenuItem
              key={tool.path}
              text={tool.label}
              onClick={() => {
                setOpen(false);
                if (tool.confirm) setAsking(tool);
                else run(tool);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
      {asking !== null ? (
        <AlertDialog
          onDismissRequest={() => setAsking(null)}
          title={asking.confirmTitle}
          text={asking.confirmMessage}
          confirmButton={
            <TextButton
              onClick={() => {
                setAsking(null);
                run(asking);
              }}
            >
              {confirmLabel(asking)}
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setAsking(null)}>Cancel</TextButton>}
        />
      ) : null}
      {review !== null ? <ReviewDialog shown={review} chosen={chosen} onChosen={setChosen} onApply={() => apply(review)} onDismiss={() => setReview(null)} /> : null}
      {refusal !== null ? (
        <AlertDialog onDismissRequest={() => setRefusal(null)} title="Parameters" text={refusal} confirmButton={<TextButton onClick={() => setRefusal(null)}>Close</TextButton>} />
      ) : null}
    </>
  );
};

const ReviewDialog = ({
  shown,
  chosen,
  onChosen,
  onApply,
  onDismiss,
}: {
  shown: ParameterReview;
  chosen: ReadonlySet<string>;
  onChosen: (chosen: ReadonlySet<string>) => void;
  onApply: () => void;
  onDismiss: () => void;
}) => {
  const [all, setAll] = useKeyedState([shown], () => true);
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Load Parameters"
      text={
        <div className="flex flex-col">
          {reviewWarnings(shown).map((it) => (
            <Text key={it} typography="bodyMedium" color={aircast('warning')}>
              {it}
            </Text>
          ))}
          <Text typography="bodyMedium">{reviewSummary(shown)}</Text>
          {sendableCount(shown) > 0 ? <Text typography="bodyMedium">{SEND_HINT}</Text> : null}
          {shown.rows.length > 0 ? (
            <div className="flex items-center gap-3 py-2">
              <Checkbox
                checked={all}
                label="Name"
                onCheckedChange={(on) => {
                  setAll(on);
                  onChosen(checkedAll(shown, chosen, on));
                }}
              />
              <Text typography="labelLarge">Name</Text>
            </div>
          ) : null}
          <div className="max-h-[360px] overflow-auto">
            {shown.rows.map((row) => {
              const rowName = shown.multipleComponents ? `${row.componentId}: ${row.name}` : row.name;
              return (
                <div key={diffRowKey(row)} className="flex items-center gap-3 py-1">
                  <Checkbox
                    checked={chosen.has(diffRowKey(row))}
                    enabled={!row.cannotSend}
                    label={rowName}
                    onCheckedChange={(on) => onChosen(on ? new Set([...chosen, diffRowKey(row)]) : new Set([...chosen].filter((it) => it !== diffRowKey(row))))}
                  />
                  <div className="flex min-w-0 flex-1 flex-col">
                    <Text typography="bodyMedium">{rowName}</Text>
                    <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                      {diffLine(row)}
                    </Text>
                  </div>
                </div>
              );
            })}
          </div>
          {shown.rows.some((it) => it.noVehicleValue && !it.cannotSend) ? <Text typography="bodySmall">{NEW_TO_VEHICLE_HINT}</Text> : null}
          {shown.rows.some((it) => it.cannotSend) ? <Text typography="bodySmall">{CANNOT_SEND_HINT}</Text> : null}
        </div>
      }
      confirmButton={
        sendableCount(shown) > 0 ? (
          <TextButton enabled={chosen.size > 0} onClick={onApply}>
            Ok
          </TextButton>
        ) : undefined
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};
