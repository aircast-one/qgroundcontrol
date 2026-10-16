import { type KeyboardEvent, type ReactNode, useMemo, useState } from 'react';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { type Json, arrayJSONObject, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { IconButton, Text, TextButton } from '@/m3';
import type { SnackbarDuration } from '@/m3/Feedback';
import { AircastSheet } from '@/map/AircastSheet';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { useOsdBackdrop, useOsdTint } from './Osd';
import { EDIT_PARAMETER_TITLE, LinkedText, ParameterEditDialog } from './ParameterLinks';
import { EmptyState } from './SetupRow';

export const MESSAGES = 'view.messages';

export type MessageSeverity = { readonly name: 'Error' | 'Warning' | 'Normal'; readonly ordinal: number };

export const MessageSeverity = {
  Error: { name: 'Error', ordinal: 0 },
  Warning: { name: 'Warning', ordinal: 1 },
  Normal: { name: 'Normal', ordinal: 2 },
} as const satisfies Record<string, MessageSeverity>;

export const MessageSeverityEntries: readonly MessageSeverity[] = [MessageSeverity.Error, MessageSeverity.Warning, MessageSeverity.Normal];

export type VehicleMessage = {
  index: number;
  time: string;
  severity: string;
  level: MessageSeverity;
  text: string;
  toldElsewhere: boolean;
};

export const VehicleMessage = (fields: { index: number; time: string; severity: string; level: MessageSeverity; text: string; toldElsewhere?: boolean }): VehicleMessage => ({
  toldElsewhere: false,
  ...fields,
});

const PREARM_PREFIX = 'PreArm:';

export const chipBlocker = (blocker: string): string => {
  const trimmed = blocker.trim();
  const rest = (trimmed.startsWith(PREARM_PREFIX) ? trimmed.slice(PREARM_PREFIX.length) : trimmed).trim();
  const sentence = rest.includes('. ') ? rest.slice(0, rest.indexOf('. ')) : rest;
  return sentence.replace(/\.+$/, '');
};

export const bannerMessages = (unread: readonly VehicleMessage[]): readonly VehicleMessage[] =>
  unread.filter((it) => (it.level === MessageSeverity.Error || it.level === MessageSeverity.Warning) && !it.toldElsewhere);

export const bannerText = (messages: readonly VehicleMessage[]): string | null => {
  if (messages.length === 0) return null;
  const worst = messages.findLast((it) => it.level === MessageSeverity.Error) ?? messages.findLast((it) => it.level === MessageSeverity.Warning) ?? null;
  const count = messageCountText(messages.length);
  return worst !== null && worst.text.trim() !== '' ? worst.text : count;
};

export const severitySummary = (messages: readonly VehicleMessage[]): string => {
  const counted = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`;
  const errors = messages.filter((it) => it.level === MessageSeverity.Error).length;
  const warnings = messages.filter((it) => it.level === MessageSeverity.Warning).length;
  const rest = messages.length - errors - warnings;
  const summary = [...(errors > 0 ? [counted(errors, 'error')] : []), ...(warnings > 0 ? [counted(warnings, 'warning')] : []), ...(rest > 0 ? [counted(rest, 'message')] : [])].join(' · ');
  return summary.trim() === '' ? messageCountText(0) : summary;
};

export const messageCountText = (count: number): string => `${count} message${count === 1 ? '' : 's'} from the vehicle`;

export const unreadMessages = (messages: readonly VehicleMessage[], unread: number): readonly VehicleMessage[] => messages.slice(Math.max(0, messages.length - Math.max(0, unread)));

export const unreadCount = (view: Json | null): number => (view === null ? 0 : optInt(view, 'unread', 0));

export const levelOf = (name: string): MessageSeverity => {
  switch (name) {
    case 'error':
      return MessageSeverity.Error;
    case 'warning':
      return MessageSeverity.Warning;
    default:
      return MessageSeverity.Normal;
  }
};

export const OLDEST_FIRST = 'oldestFirst';

export const vehicleMessages = (view: Json | null): readonly VehicleMessage[] => {
  const items = optJSONArray(view, 'items');
  if (view === null || items === null) return [];
  const read = items.flatMap((_, at) => {
    const item = arrayJSONObject(items, at);
    if (item === null) return [];
    const text = optText(item, 'text');
    return text.trim() === ''
      ? []
      : [
          VehicleMessage({
            index: optInt(item, 'index', at),
            time: messageTime(optText(item, 'time')),
            severity: optText(item, 'severity'),
            level: levelOf(optText(item, 'level')),
            text,
            toldElsewhere: optBoolean(item, 'toldElsewhere'),
          }),
        ];
  });
  return optText(view, 'order') === OLDEST_FIRST ? read : read.toReversed();
};

export const WARNINGS = 'view.warnings';

export const armingBlocker = (view: Json | null): string | null => {
  if (view === null || isNull(view, 'armingBlocker')) return null;
  const blocker = optText(view, 'armingBlocker');
  return blocker.trim() === '' ? null : blocker;
};

export type ArmingCheck = { message: string; description: string; severity: string };

export const ArmingCheck = (message: string, description: string, severity: string): ArmingCheck => ({ message, description, severity });

export const armingChecks = (view: Json | null): readonly ArmingCheck[] | null => {
  const listed = view === null || isNull(view, 'armingChecks') ? null : optJSONArray(view, 'armingChecks');
  if (listed === null) return null;
  return jsonObjects(listed)
    .map((problem) => ArmingCheck(optText(problem, 'message'), optText(problem, 'description'), optText(problem, 'severity')))
    .filter((it) => it.message.trim() !== '');
};

const CLOCK_WITH_MILLIS = /^(\d{1,2}:\d{2}:\d{2})\.\d+$/;

export const messageTime = (served: string): string => CLOCK_WITH_MILLIS.exec(served.trim())?.[1] ?? served;

const ALERT_CORNER = 12;

const activate = (event: KeyboardEvent, run: () => void) => {
  if (event.key !== 'Enter' && event.key !== ' ') return;
  event.preventDefault();
  run();
};

export const VehicleMessageBanner = ({ className }: { className?: string }) => {
  const messagesJson = useQgcPath(MESSAGES);
  const messages = useMemo(() => vehicleMessages(messagesJson), [messagesJson]);
  const [showing, setShowing] = useState(false);
  const shown = bannerMessages(unreadMessages(messages, unreadCount(messagesJson)));
  const urgent = shown.some((it) => it.level === MessageSeverity.Error);
  const errorBackdrop = useOsdBackdrop(md('errorContainer'));
  const warningBackdrop = useOsdBackdrop(aircast('warningContainer'));
  const errorContent = useOsdTint(md('onErrorContainer'), md('error'));
  const warningContent = useOsdTint(md('onSurface'), aircast('warning'));

  const sheet = showing ? <VehicleMessagesSheet onDismiss={() => setShowing(false)} /> : null;
  if (shown.length === 0) return sheet;

  const open = () => {
    setShowing(true);
    void VehicleCommands.resetAllMessages();
  };
  const content = urgent ? errorContent : warningContent;
  return (
    <>
      {sheet}
      <div
        role="button"
        tabIndex={0}
        onClick={open}
        onKeyDown={(event) => activate(event, open)}
        className={`flex min-h-10 cursor-pointer items-center gap-2.5 px-3 py-1.5 ${className ?? ''}`}
        style={{ background: urgent ? errorBackdrop : warningBackdrop, color: content, borderRadius: urgent ? ALERT_CORNER : 9999 }}
      >
        <Drawable name={urgent ? 'ic_error' : 'ic_warning'} size={24} className="shrink-0" style={{ color: urgent ? content : aircast('warning') }} />
        <Text typography="labelLarge" maxLines={2} className="min-w-0">
          {bannerText(shown) ?? messageCountText(shown.length)}
        </Text>
        <span className="flex shrink-0" onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}>
          <IconButton onClick={() => void VehicleCommands.resetAllMessages()} label="Dismiss messages" className="!size-8" contentColor={content}>
            <Drawable name="ic_close" size={24} />
          </IconButton>
        </span>
      </div>
    </>
  );
};

export const VehicleMessagesSheet = ({ onDismiss }: { onDismiss: () => void }) => {
  const warnings = useQgcPath(WARNINGS);
  const messagesJson = useQgcPath(MESSAGES);
  const messages = useMemo(() => vehicleMessages(messagesJson), [messagesJson]);
  const checks = useMemo(() => armingChecks(warnings) ?? [], [warnings]);
  return <VehicleMessageLog messages={messages} checks={checks} blocker={armingBlocker(warnings)} onDismiss={onDismiss} />;
};

const VehicleMessageLog = ({ messages, checks, blocker, onDismiss }: { messages: readonly VehicleMessage[]; checks: readonly ArmingCheck[]; blocker: string | null; onDismiss: () => void }) => {
  const blocking = checks.length === 0 ? (blocker === null ? [] : [ArmingCheck(blocker, '', 'error')]) : checks;
  const lines = useMemo(() => messages.toReversed(), [messages]);
  const [editing, setEditing] = useState<string | null>(null);

  return (
    <AircastSheet onDismissRequest={onDismiss}>
      {editing !== null ? <ParameterEditDialog name={editing} title={EDIT_PARAMETER_TITLE} onDismiss={() => setEditing(null)} /> : null}
      <div className="flex flex-col px-6 pb-6">
        <Text typography="headlineSmall">{blocking.length === 0 ? 'Messages' : 'Why it will not arm'}</Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {severitySummary(lines)}
        </Text>
        {lines.length === 0 && blocking.length === 0 ? (
          <Text className="block py-4">The vehicle has not said anything yet.</Text>
        ) : (
          <div className="max-h-[420px] overflow-auto pt-3">
            {blocking.map((check, at) => (
              <MessageLine key={`check-${at}`} level={check.severity === 'error' ? MessageSeverity.Error : MessageSeverity.Warning} time="">
                <Text typography="bodyMedium">{check.message}</Text>
                {check.description.trim() !== '' ? <LinkedText html={check.description} style="bodySmall" color={md('onSurfaceVariant')} onParameter={setEditing} /> : null}
              </MessageLine>
            ))}
            {lines.map((message, at) => (
              <MessageLine key={`line-${at}`} level={message.level} time={message.time}>
                <Text typography="bodyMedium">{message.text}</Text>
              </MessageLine>
            ))}
          </div>
        )}
        <div className="flex justify-end pt-2">
          <TextButton
            onClick={() => {
              void VehicleCommands.clearMessages();
              onDismiss();
            }}
          >
            Clear
          </TextButton>
          <TextButton onClick={onDismiss}>Close</TextButton>
        </div>
      </div>
    </AircastSheet>
  );
};

export const VehicleMessagesPage = ({ className }: { className?: string }) => {
  const messagesJson = useQgcPath(MESSAGES);
  const lines = useMemo(() => vehicleMessages(messagesJson).toReversed(), [messagesJson]);
  useLaunchedEffect([], () => void VehicleCommands.resetAllMessages());

  if (lines.length === 0) return <EmptyState icon="ic_description" title="No messages" text="The vehicle has not said anything yet." className={className} />;
  return (
    <div className={`size-full overflow-auto px-4 ${className ?? ''}`}>
      {lines.map((message, at) => (
        <MessageLine key={at} level={message.level} time={message.time}>
          <Text typography="bodyMedium">{message.text}</Text>
        </MessageLine>
      ))}
      <div className="flex justify-end">
        <TextButton onClick={() => void VehicleCommands.clearMessages()}>Clear</TextButton>
      </div>
    </div>
  );
};

const levelIcon = (level: MessageSeverity) => {
  switch (level.name) {
    case 'Error':
      return { icon: 'ic_error', tint: md('error') } as const;
    case 'Warning':
      return { icon: 'ic_warning', tint: aircast('warning') } as const;
    case 'Normal':
      return { icon: 'ic_check_circle', tint: aircast('success') } as const;
  }
};

export const MessageLine = ({ level, time, children }: { level: MessageSeverity; time: string; children: ReactNode }) => {
  const { icon, tint } = levelIcon(level);
  return (
    <div className="flex w-full gap-4 py-2.5">
      <Drawable name={icon} size={24} className="shrink-0" style={{ color: tint }} />
      <div className="flex min-w-0 flex-1 flex-col">
        {time.trim() !== '' ? (
          <Text typography="labelSmall" color={md('onSurfaceVariant')}>
            {time}
          </Text>
        ) : null}
        {children}
      </div>
    </div>
  );
};

export type SnackbarData = {
  visuals: { message: string; duration: SnackbarDuration; actionLabel?: string | null; withDismissAction?: boolean };
  dismiss: () => void;
  performAction?: () => void;
};

export const AppSnackbar = ({ data }: { data: SnackbarData }) => {
  if (data.visuals.duration !== 'Indefinite') {
    const action = data.visuals.actionLabel ?? null;
    return (
      <div role="status" aria-live="polite" className="mx-auto flex min-h-12 w-[min(560px,calc(100vw-32px))] items-center gap-2 rounded-shape-xs bg-inverse-surface py-1 pl-4 pr-2 text-body-medium text-inverse-on-surface shadow-lg">
        <span className="flex-1 py-2">{data.visuals.message}</span>
        {action !== null ? (
          <TextButton onClick={data.performAction} contentColor="var(--md-inverse-primary,var(--md-primary-container))">
            {action}
          </TextButton>
        ) : null}
        {data.visuals.withDismissAction === true ? (
          <IconButton onClick={data.dismiss} label="Dismiss" contentColor={md('inverseOnSurface')}>
            <Drawable name="ic_close" size={24} />
          </IconButton>
        ) : null}
      </div>
    );
  }
  return (
    <div
      role="alert"
      className="m-3 flex min-h-12 items-center gap-2 py-1 pl-4 pr-1 shadow-lg"
      style={{ borderRadius: ALERT_CORNER, background: md('errorContainer'), color: md('onErrorContainer') }}
    >
      <div className="flex flex-1 items-center gap-3">
        <Drawable name="ic_error" size={24} className="shrink-0" />
        <Text typography="labelLarge">{data.visuals.message}</Text>
      </div>
      <IconButton onClick={data.dismiss} label="Dismiss" contentColor={md('onErrorContainer')}>
        <Drawable name="ic_close" size={24} />
      </IconButton>
    </div>
  );
};
