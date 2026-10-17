import { useEffect, useState } from 'react';
import { type Json, isJson, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optLong, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { SetupCommands } from '@/bridge/BridgeCommands';
import { useBackHandler, useLaunchedEffect, useTicker } from '@/kotlin/Compose';
import { Checkbox, DropdownMenu, DropdownMenuItem, FilterChip, HorizontalDivider, ListItem, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { INSPECTOR_CHARTS_VIEW, type InspectorCharts, InspectorChartPanel, chartToggleEnabled, inspectorCharts, toggleChartField } from '@/ui/InspectorCharts';
import { SearchPill, SetupRow } from '@/ui/SetupRow';

export const inspectorSystemText = (view: Json | null): string | null => {
  const system = view !== null && optBoolean(view, 'available') ? optInt(view, 'systemId', 0) : 0;
  return system > 0 ? `System ${system}` : null;
};
export const INSPECTOR_VIEW = 'view.inspector';

export const inspectorRateText = (view: Json | null): string | null => {
  const listed = view !== null && optBoolean(view, 'available') ? optJSONArray(view, 'messages') : null;
  if (listed === null) return null;
  const total = listed
    .map((entry) => (isJson(entry) ? optDouble(entry, 'rateHz', 0) : 0))
    .filter((rate) => Number.isFinite(rate))
    .reduce((sum, rate) => sum + rate, 0);
  return total > 0 ? `${Math.round(total)} messages/s` : null;
};

export const inspectorEmptyText = (view: Json | null): string | null => {
  if (view === null) return null;
  const text = optText(view, 'emptyText');
  return text.trim() === '' ? null : text;
};

export type InspectorMessage = {
  index: number;
  id: number;
  name: string;
  rateText: string;
  count: number;
  path: string;
  compId: number;
  title: string;
  targetRateTitle: string;
  fieldSelected: boolean;
};

export const InspectorMessage = (fields: Omit<InspectorMessage, 'targetRateTitle' | 'fieldSelected'> & { targetRateTitle?: string; fieldSelected?: boolean }): InspectorMessage => ({
  targetRateTitle: '',
  fieldSelected: false,
  ...fields,
});

export type InspectorRate = { rate: number; title: string };

export type InspectorChoice = { id: number; title: string };

export const inspectorChoices = (view: Json | null, key: string): readonly InspectorChoice[] =>
  jsonObjects(optJSONArray(view, key)).map((it) => ({ id: optInt(it, 'id'), title: optText(it, 'title') }));

export const inspectorDetails = (message: InspectorMessage): readonly (readonly [string, string])[] => [
  ['Message', `${message.name} (${message.id})`],
  ['Component', String(message.compId)],
  ['Count', String(message.count)],
  ['Actual Rate', message.rateText],
];

export const inspectorShown = (messages: readonly InspectorMessage[], filter: string, component: number | null): readonly InspectorMessage[] =>
  messages.filter((it) => it.name.toLowerCase().includes(filter.toLowerCase()) && (component === null || it.compId === component));

export const inspectorRateChoices = (view: Json | null): readonly InspectorRate[] =>
  jsonObjects(optJSONArray(view, 'rateChoices'))
    .map((it) => ({ rate: optInt(it, 'rate'), title: optText(it, 'title') }))
    .filter((it) => it.title.trim() !== '');

export type InspectorField = { name: string; type: string; value: string };

export const inspectorMessages = (view: Json | null): readonly InspectorMessage[] => {
  const items = optJSONArray(view, 'messages');
  if (items === null) return [];
  return items
    .flatMap((message, index) =>
      !isJson(message) ? [] : [InspectorMessage({
        index: optInt(message, 'index', index),
        id: optInt(message, 'id'),
        name: optText(message, 'name'),
        rateText: optText(message, 'rateText'),
        count: optLong(message, 'count'),
        path: optText(message, 'path'),
        compId: optInt(message, 'compId'),
        title: optText(message, 'title').trim() === '' ? optText(message, 'name') : optText(message, 'title'),
        targetRateTitle: optText(message, 'targetRateTitle'),
        fieldSelected: optBoolean(message, 'fieldSelected'),
      })],
    )
    .toSorted((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
};

export const parseInspectorFields = (view: Json | null, messagePath: string): readonly InspectorField[] => {
  const messages = optJSONArray(view, 'messages');
  if (messages === null) return [];
  const shown = jsonObjects(messages).some((it) => optBoolean(it, 'selected') && optText(it, 'path') === messagePath);
  if (!shown) return [];
  return jsonObjects(optJSONArray(view, 'fields')).map((field) => ({ name: optText(field, 'name'), type: optText(field, 'type'), value: optText(field, 'value') }));
};

const CHART_POLL_MS = 200;

export const selectedPathFor = (messagePath: string): string => {
  const cut = messagePath.indexOf('.messages.');
  return (cut < 0 ? messagePath : messagePath.slice(0, cut)) + '.selected';
};

export const messageIndexIn = (messagePath: string): number => {
  const tail = messagePath.slice(messagePath.lastIndexOf('.') + 1);
  return /^[+-]?\d+$/.test(tail) && Math.abs(Number(tail)) <= 2147483647 ? Number(tail) : -1;
};

const selectMessage = (messagePath: string): void => {
  void Qgc.set(selectedPathFor(messagePath), messageIndexIn(messagePath));
};

const requestRate = (messagePath: string, rate: number): void => {
  void (async () => {
    await Qgc.set(selectedPathFor(messagePath), messageIndexIn(messagePath));
    await SetupCommands.setInspectorMessageInterval(rate);
  })();
};

const RatePicker = ({ messagePath, current, choices }: { messagePath: string; current: string; choices: readonly InspectorRate[] }) => {
  const [open, setOpen] = useState(false);
  if (choices.length === 0) return null;
  return (
    <div className="relative">
      <TextButton onClick={() => setOpen(true)}>{current.trim() === '' ? 'Set Rate:' : `Rate: ${current}`}</TextButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {choices.map((choice, index) => (
          <DropdownMenuItem
            key={index}
            text={choice.title}
            onClick={() => {
              setOpen(false);
              requestRate(messagePath, choice.rate);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

export const openMessageIn = (messages: readonly InspectorMessage[], path: string | null): InspectorMessage | null =>
  path === null ? null : (messages.find((it) => it.path === path) ?? null);

const InspectorNotice = ({ text, className }: { text: string; className?: string }) => (
  <Text typography="bodyLarge" textAlign="center" className={`w-full p-6 ${className ?? ''}`}>
    {text}
  </Text>
);

const FieldRows = ({ rows, className }: { rows: readonly InspectorField[]; className?: string }) => {
  const [charts, setCharts] = useState<InspectorCharts | null>(null);
  useTicker(CHART_POLL_MS, [], async () => setCharts(inspectorCharts(await Qgc.get(INSPECTOR_CHARTS_VIEW))));

  return (
    <div className={`flex size-full flex-col overflow-y-auto ${className ?? ''}`}>
      {charts !== null ? (
        <>
          <InspectorChartPanel key="chart0" index={0} charts={charts} />
          <InspectorChartPanel key="chart1" index={1} charts={charts} />
        </>
      ) : null}
      <div key="plots" className="flex w-full justify-end pr-4">
        {CHART_LABELS.map((label) => (
          <Text key={label} typography="labelSmall" textAlign="center" maxLines={1} className="w-12">
            {label}
          </Text>
        ))}
      </div>
      {rows.map((field, index) => (
        <div key={`${index}:${field.name}`}>
          <ListItem
            headlineContent={<Text monospace>{field.name}</Text>}
            supportingContent={field.type}
            trailingContent={
              <div className="flex items-center">
                <Text monospace>{field.value}</Text>
                {[0, 1].map((chart) => (
                  <span key={chart} className="flex w-12 justify-center">
                    <Checkbox
                      checked={charts?.charted.get(field.name) === chart}
                      enabled={chartToggleEnabled(charts, field.name, field.type, chart)}
                      onCheckedChange={(wanted) => toggleChartField(chart, field.name, wanted)}
                      label={`${field.name} on ${CHART_LABELS[chart]}`}
                    />
                  </span>
                ))}
              </div>
            }
          />
          <HorizontalDivider />
        </div>
      ))}
    </div>
  );
};

const FieldList = ({ messagePath, className }: { messagePath: string; className?: string }) => {
  useLaunchedEffect([messagePath], () => selectMessage(messagePath));

  const inspectorJson = useQgcPath(INSPECTOR_VIEW);
  const rows = parseInspectorFields(inspectorJson, messagePath);

  if (rows.length === 0) return <InspectorNotice text="Waiting for this message to arrive again." className={className} />;
  return <FieldRows rows={rows} className={className} />;
};

export const InspectorScreen = ({ className }: { className?: string }) => {
  const inspectorJson = useQgcPath(INSPECTOR_VIEW);
  const [openPath, setOpenPath] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const [component, setComponent] = useState<number | null>(null);
  const systems = inspectorChoices(inspectorJson, 'systems');
  const components = inspectorChoices(inspectorJson, 'components');
  const activeSystem = inspectorJson !== null && !isNull(inspectorJson, 'systemId') ? optInt(inspectorJson, 'systemId') : null;
  useEffect(() => setComponent(null), [activeSystem]);
  const messages = inspectorMessages(inspectorJson);
  const open = openMessageIn(messages, openPath);

  useBackHandler(open !== null, () => setOpenPath(null));

  const emptyText = inspectorEmptyText(inspectorJson);
  if (emptyText !== null) return <InspectorNotice text={emptyText} className={className} />;

  if (open !== null) {
    return (
      <div className={`flex size-full flex-col ${className ?? ''}`}>
        <button type="button" className="flex w-full items-center p-4 text-left" onClick={() => setOpenPath(null)}>
          <Text typography="titleSmall" monospace className="flex-1">
            {open.name}
          </Text>
        </button>
        <div className="flex w-full items-center px-2">
          <RatePicker messagePath={open.path} current={open.targetRateTitle} choices={inspectorRateChoices(inspectorJson)} />
        </div>
        <div className="flex flex-col px-4 py-1">
          {inspectorDetails(open).map(([label, value]) => (
            <div key={label} className="flex w-full">
              <Text typography="bodySmall" className="flex-1">
                {label}
              </Text>
              <Text typography="bodySmall" monospace>
                {value}
              </Text>
            </div>
          ))}
        </div>
        <HorizontalDivider />
        <FieldList messagePath={open.path} className="min-h-0 flex-1" />
      </div>
    );
  }

  const shown = inspectorShown(messages, filter, component);

  return (
    <div className={`flex size-full flex-col ${className ?? ''}`}>
      {(() => {
        const line = inspectorSystemText(inspectorJson);
        return line !== null ? (
          <Text typography="labelSmall" color={md('onSurfaceVariant')} className="px-5 py-1">
            {line}
          </Text>
        ) : null;
      })()}
      {systems.length > 1 || components.length > 1 ? (
        <div className="flex gap-2 overflow-x-auto px-4">
          {systems.length > 1
            ? systems.map((system) => (
                <FilterChip key={`s${system.id}`} selected={system.id === activeSystem} onClick={() => void SetupCommands.setInspectorSystem(system.id)} label={system.title} className="shrink-0" />
              ))
            : null}
          {components.length > 1
            ? [{ id: -1, title: 'Comp All' }, ...components].map((choice) => (
                <FilterChip key={`c${choice.id}`} selected={(component ?? -1) === choice.id} onClick={() => setComponent(choice.id >= 0 ? choice.id : null)} label={choice.title} className="shrink-0" />
              ))
            : null}
        </div>
      ) : null}
      <SearchPill value={filter} onValueChange={setFilter} placeholder="Filter messages" />

      {shown.length === 0 ? (
        <InspectorNotice text={`No message matches "${filter}".`} />
      ) : (
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
          {shown.map((message) => (
            <SetupRow
              key={message.path}
              title={(message.name.trim() === '' ? message.title : message.name) + (message.fieldSelected ? ' *' : '')}
              subtitle={`comp ${message.compId}`}
              status={message.rateText}
              onClick={() => setOpenPath(message.path)}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const CHART_LABELS: readonly string[] = ['Plot 1', 'Plot 2'];
