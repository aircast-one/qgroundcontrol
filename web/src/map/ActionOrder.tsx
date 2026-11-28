import type { CSSProperties, PointerEvent as ReactPointerEvent, ReactNode } from 'react';
import { useEffect, useState } from 'react';
import {
  type Announcements,
  type CollisionDetection,
  DndContext,
  type DragEndEvent,
  type DragOverEvent,
  DragOverlay,
  type DragStartEvent,
  type KeyboardCoordinateGetter,
  KeyboardSensor,
  type Modifier,
  PointerSensor,
  TouchSensor,
  type UniqueIdentifier,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core';
import { SortableContext, type SortingStrategy, useSortable, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useKeyedState } from '@/kotlin/Compose';
import { IconButton, Text } from '@/m3';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { swallowReleaseClick } from '@/ui/OverlayLayout';
import { type ShownAction, announcements, canDrop, dropMove, movedOrder, projectedPlaces, shownAt, stepKeys } from './ActionOrderModel';

const GRIP = 'data-drag-grip';
const TOUCH_HOLD_MS = 250;
const TOUCH_SLOP_PX = 8;
const OUTSIDE_MARGIN_PX = 32;
const SETTLED = 0.35;
const SETTLE_FADE = 'opacity 160ms ease-out';
const LIFT_FILTER = 'brightness(1.15) drop-shadow(0 8px 12px rgb(0 0 0 / 0.55))';

const fromGrip = (target: EventTarget | null): boolean => target instanceof Element && target.closest(`[${GRIP}]`) !== null;

class GripPointerSensor extends PointerSensor {
  static activators = [
    {
      eventName: 'onPointerDown' as const,
      handler: ({ nativeEvent }: ReactPointerEvent) => nativeEvent.isPrimary && nativeEvent.button === 0 && nativeEvent.pointerType !== 'touch' && fromGrip(nativeEvent.target),
    },
  ];
}

class GripKeyboardSensor extends KeyboardSensor {
  static activators = KeyboardSensor.activators.map((activator) => ({
    ...activator,
    handler: (event: { nativeEvent: KeyboardEvent }, options: Parameters<typeof activator.handler>[1], context: Parameters<typeof activator.handler>[2]) =>
      fromGrip(event.nativeEvent.target) && activator.handler(event, options, context),
  }));
}

const vertical: Modifier = ({ transform }) => ({ ...transform, x: 0 });

const KEY_STEPS: Readonly<Record<string, number>> = { ArrowUp: -1, ArrowDown: 1 };

const nextSlot: KeyboardCoordinateGetter = (event, { context: { active, over, collisionRect, droppableRects, droppableContainers } }) => {
  const step = KEY_STEPS[event.code];
  if (step === undefined || active === null || collisionRect === null) return undefined;
  event.preventDefault();
  const slots = droppableContainers
    .getEnabled()
    .flatMap((container) => {
      const rect = droppableRects.get(container.id);
      return rect === undefined ? [] : [{ id: container.id, rect }];
    })
    .toSorted((a, b) => a.rect.top - b.rect.top);
  const target = slots[slots.findIndex((it) => it.id === (over?.id ?? active.id)) + step];
  return target === undefined ? undefined : { x: collisionRect.left, y: target.rect.top + (target.rect.height - collisionRect.height) / 2 };
};

const withinList: CollisionDetection = (args) => {
  const point = args.pointerCoordinates;
  const rects = [...args.droppableRects.values()];
  if (point === null || rects.length === 0) return closestCenter(args);
  const inside =
    point.x >= Math.min(...rects.map((it) => it.left)) - OUTSIDE_MARGIN_PX &&
    point.x <= Math.max(...rects.map((it) => it.right)) + OUTSIDE_MARGIN_PX &&
    point.y >= Math.min(...rects.map((it) => it.top)) - OUTSIDE_MARGIN_PX &&
    point.y <= Math.max(...rects.map((it) => it.bottom)) + OUTSIDE_MARGIN_PX;
  return inside ? closestCenter(args) : [];
};

const refusing =
  <A,>(steps: readonly ShownAction<A>[]): SortingStrategy =>
  (args) =>
    verticalListSortingStrategy(canDrop(steps[args.activeIndex], args.overIndex) ? args : { ...args, overIndex: args.activeIndex });

export const ActionRow = <A,>({
  step,
  grip,
  lifted = false,
  refused = false,
  onEdit,
  onRemove,
}: {
  step: ShownAction<A>;
  grip: ReactNode;
  lifted?: boolean;
  refused?: boolean;
  onEdit: (() => void) | null;
  onRemove: (() => void) | null;
}) => (
  <div
    className="flex items-center bg-secondary-container text-on-secondary-container"
    style={{ borderRadius: 14, outline: refused ? `2px solid ${md('error')}` : 'none', cursor: refused ? 'not-allowed' : undefined, filter: lifted ? LIFT_FILTER : 'none' }}
  >
    {grip}
    <button type="button" className="flex min-w-0 flex-1 items-center pl-1 text-left" onClick={onEdit ?? undefined} disabled={onEdit === null}>
      <Drawable name={step.icon} size={20} />
      <span className="flex flex-col pt-1.5 pb-1.5 pl-2">
        <Text typography="labelLarge">{step.title}</Text>
        {step.value.trim() !== '' ? <Text typography="labelSmall">{step.value}</Text> : null}
      </span>
    </button>
    {refused ? <MaterialIcon name="block" size={18} style={{ color: md('error') }} /> : null}
    {onRemove !== null ? (
      <IconButton onClick={onRemove} label={`Remove ${step.title}`} contentColor={md('onSecondaryContainer')}>
        <MaterialIcon name="close" filled size={18} />
      </IconButton>
    ) : null}
  </div>
);

const GripIcon = () => (
  <span className="grid h-12 w-10 shrink-0 place-items-center" style={{ color: md('onSecondaryContainer') }}>
    <Drawable name="ic_drag_handle" size={20} />
  </span>
);

const SortableRow = <A,>({
  id,
  shown,
  enabled,
  onEdit,
  onRemove,
}: {
  id: string;
  shown: ShownAction<A>;
  enabled: boolean;
  onEdit: (() => void) | null;
  onRemove: (() => void) | null;
}) => {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id, disabled: !enabled });
  const style: CSSProperties = { transform: CSS.Translate.toString(transform), transition: [transition, SETTLE_FADE].filter((it) => it !== undefined).join(', '), opacity: isDragging ? SETTLED : 1, touchAction: 'manipulation' };
  return (
    <li ref={setNodeRef} style={style} {...listeners}>
      <ActionRow
        step={shown}
        grip={
          <button type="button" ref={setActivatorNodeRef} {...{ [GRIP]: '' }} {...attributes} aria-label={`Reorder ${shown.title}`} className="cursor-grab touch-none" style={{ touchAction: 'none' }}>
            <GripIcon />
          </button>
        }
        onEdit={onEdit}
        onRemove={onRemove}
      />
    </li>
  );
};

export type ActionOrderProps<A> = {
  readonly steps: readonly ShownAction<A>[];
  readonly onMove: (args: string, undo: () => void) => void;
  readonly onEdit: (step: ShownAction<A>) => (() => void) | null;
  readonly onRemove: (step: ShownAction<A>) => (() => void) | null;
};

const INSTRUCTIONS = 'To reorder an action, press Space on its grip, move it with the up and down arrows, then press Space to drop it or Escape to cancel. On a touch screen, press and hold an action, then drag it.';

export const ActionOrderList = <A,>({ waypoint, steps, onMove, onEdit, onRemove }: ActionOrderProps<A> & { readonly waypoint: number }) => {
  const orderable = steps.filter((it) => it.place !== null);
  const fixed = steps.filter((it) => it.place === null);
  const signature = JSON.stringify(orderable.map((it) => [it.kind, it.title, it.value, it.item]));
  const [pending, setPending] = useKeyedState<readonly [number, number] | null>([signature, waypoint], () => null);
  const [lifted, setLifted] = useState<number | null>(null);
  const [over, setOver] = useState<number | null>(null);
  const [touched, setTouched] = useState(false);
  const shownSteps = pending === null ? orderable : movedOrder(orderable, pending[0], pending[1]);
  const ids = stepKeys(shownSteps);
  const placeOf = (id: UniqueIdentifier | null | undefined): number | null => {
    const at = id === null || id === undefined ? -1 : ids.indexOf(String(id));
    return at < 0 ? null : at;
  };
  const projected = projectedPlaces(shownSteps, lifted, over);
  const shownAtProjected = (step: ShownAction<A>, at: number) => shownAt(step, pending === null ? (projected[at] ?? at) : at);
  const sensors = useSensors(
    useSensor(GripPointerSensor),
    useSensor(TouchSensor, { activationConstraint: { delay: TOUCH_HOLD_MS, tolerance: TOUCH_SLOP_PX } }),
    useSensor(GripKeyboardSensor, { coordinateGetter: nextSlot }),
  );
  const titleOf = (id: UniqueIdentifier): string => {
    const at = placeOf(id);
    const step = at === null ? undefined : shownSteps[at];
    return step === undefined || at === null ? 'The action' : shownAtProjected(step, at).title;
  };
  const announced: Announcements = {
    onDragStart: ({ active }) => announcements.lifted(titleOf(active.id), placeOf(active.id) ?? 0, shownSteps.length),
    onDragOver: ({ active, over: target }) => {
      const from = placeOf(active.id);
      const to = placeOf(target?.id);
      if (from === null || to === null) return `${titleOf(active.id)} is outside the list. Let go to put it back.`;
      return announcements.over(titleOf(active.id), to, shownSteps.length, shownSteps[from]?.refusal ?? null, canDrop(shownSteps[from], to));
    },
    onDragEnd: ({ active, over: target }) => {
      const from = placeOf(active.id);
      const to = placeOf(target?.id);
      const moved = from !== null && to !== null && dropMove(waypoint, shownSteps[from], to) !== null;
      return announcements.dropped(titleOf(active.id), to ?? from ?? 0, shownSteps.length, moved);
    },
    onDragCancel: ({ active }) => announcements.cancelled(titleOf(active.id)),
  };
  useEffect(() => {
    if (lifted === null) return undefined;
    const kept = new AbortController();
    window.addEventListener(
      'keydown',
      (event) => {
        if (event.key === 'Escape') event.preventDefault();
      },
      { capture: true, signal: kept.signal },
    );
    return () => void setTimeout(() => kept.abort());
  }, [lifted]);
  const settle = () => {
    if (touched) swallowReleaseClick();
    setLifted(null);
    setOver(null);
    setTouched(false);
  };
  const started = (event: DragStartEvent) => {
    setLifted(placeOf(event.active.id));
    setOver(placeOf(event.active.id));
    setTouched(typeof TouchEvent !== 'undefined' && event.activatorEvent instanceof TouchEvent);
  };
  const moved = (event: DragOverEvent) => setOver(placeOf(event.over?.id));
  const ended = (event: DragEndEvent) => {
    const from = placeOf(event.active.id);
    const to = placeOf(event.over?.id);
    const args = from === null ? null : dropMove(waypoint, shownSteps[from], to);
    settle();
    if (args === null || from === null || to === null) return;
    setPending([from, to]);
    onMove(args, () => setPending(null));
  };
  const active = lifted === null ? undefined : shownSteps[lifted];
  const refused = active !== undefined && lifted !== null && over !== null && over !== lifted && !canDrop(active, over);
  const outside = active !== undefined && over === null;
  return (
    <>
      <DndContext
        sensors={sensors}
        collisionDetection={withinList}
        modifiers={[vertical]}
        accessibility={{ announcements: announced, screenReaderInstructions: { draggable: INSTRUCTIONS } }}
        onDragStart={started}
        onDragOver={moved}
        onDragEnd={ended}
        onDragCancel={settle}
      >
        <SortableContext items={[...ids]} strategy={refusing(shownSteps)}>
          <ol className="flex flex-col gap-2" aria-label="Actions in order">
            {shownSteps.map((step, at) => (
              <SortableRow
                key={ids[at]}
                id={ids[at] ?? ''}
                shown={shownAtProjected(step, at)}
                enabled={pending === null}
                onEdit={pending === null ? onEdit(step) : null}
                onRemove={pending === null ? onRemove(step) : null}
              />
            ))}
          </ol>
        </SortableContext>
        <DragOverlay dropAnimation={null}>
          {active !== undefined && lifted !== null ? (
            <ActionRow step={shownAtProjected(active, lifted)} grip={<GripIcon />} lifted refused={refused} onEdit={null} onRemove={null} />
          ) : null}
        </DragOverlay>
      </DndContext>
      {refused && active?.refusal !== null && active?.refusal !== undefined ? (
        <Text typography="bodySmall" color={md('error')}>
          {active.refusal}
        </Text>
      ) : null}
      {outside ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Let go to put it back.
        </Text>
      ) : null}
      {fixed.length > 0 ? (
        <ul className="flex flex-col gap-2" aria-label="Actions on arrival">
          {fixed.map((step) => (
            <li key={step.kind}>
              <ActionRow step={step} grip={<span className="w-3 shrink-0" />} onEdit={onEdit(step)} onRemove={onRemove(step)} />
            </li>
          ))}
        </ul>
      ) : null}
    </>
  );
};
