import { IconButton, Surface, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { HOME_ITEM } from './MissionKinds';
import type { ItemMoves } from './PlanEditing';
import { type ItemRow, itemCountText } from './PlanItems';
import { sentenceCase } from './SentenceCase';

const ITEM_MARKER_SIZE = 40;
const ACTION_MARKER_SIZE = 12;

const rowName = (row: ItemRow): string => [sentenceCase(row.name).toLowerCase(), row.number].filter((it) => it !== '').join(' ');

const ItemListHeading = ({ rows, summary, className }: { rows: readonly ItemRow[]; summary: string; className: string }) => (
  <div className={`flex flex-col ${className}`}>
    <Text typography="titleMedium">{itemCountText(rows.filter((it) => it.index !== HOME_ITEM).length)}</Text>
    {summary.trim() === '' ? null : (
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {summary}
      </Text>
    )}
  </div>
);

const ItemRowView = ({
  row,
  selected,
  onClick,
  moves,
  onMove,
  onDelete,
}: {
  row: ItemRow;
  selected: boolean;
  onClick: () => void;
  moves: ItemMoves;
  onMove: (later: boolean) => void;
  onDelete: (() => void) | null;
}) => {
  const action = row.number === '';
  const marker = action ? ACTION_MARKER_SIZE : ITEM_MARKER_SIZE;
  return (
  <div className="flex w-full items-center" style={{ background: selected ? md('secondaryContainer') : md('surfaceContainerLow'), color: selected ? md('onSecondaryContainer') : md('onSurface') }}>
    <Surface onClick={onClick} color="transparent" contentColor="inherit" className={`flex min-w-0 flex-1 items-center gap-4 pb-2 pr-3 pt-2 ${action ? 'min-h-[56px] pl-14' : 'min-h-[72px] pl-4'}`}>
      <span
        className={`flex shrink-0 items-center justify-center rounded-full ${action ? '' : 'px-2'}`}
        style={{ minWidth: marker, height: marker, ...(row.readyForSave ? { background: row.colour } : { border: `1px solid ${aircast('warning')}` }) }}
      >
        {action ? null : (
          <Text typography="labelLarge" color={row.readyForSave ? md('surface') : aircast('warning')} maxLines={1}>
            {row.number}
          </Text>
        )}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <Text typography="titleMedium" maxLines={1}>
          {sentenceCase(row.name)}
        </Text>
        {row.detail.trim() === '' ? null : (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={2}>
            {row.detail}
          </Text>
        )}
      </span>
      {selected ? null : <MaterialIcon name="keyboard_arrow_right" filled style={{ color: md('onSurfaceVariant') }} />}
    </Surface>
    {selected && moves.earlier ? (
      <IconButton onClick={() => onMove(false)} label={`Move ${rowName(row)} up`}>
        <MaterialIcon name="arrow_upward" filled />
      </IconButton>
    ) : null}
    {selected && moves.later ? (
      <IconButton onClick={() => onMove(true)} label={`Move ${rowName(row)} down`}>
        <MaterialIcon name="arrow_downward" filled />
      </IconButton>
    ) : null}
    {selected && onDelete !== null ? (
      <IconButton onClick={onDelete} label={`Delete ${rowName(row)}`}>
        <MaterialIcon name="delete" filled style={{ color: md('error') }} />
      </IconButton>
    ) : null}
  </div>
  );
};

export const PlanItemList = ({
  rows,
  summary,
  selected,
  headingClassName,
  movesOf,
  onPick,
  onMove,
  onDelete,
}: {
  rows: readonly ItemRow[];
  summary: string;
  selected: number | null;
  headingClassName: string;
  movesOf: (index: number) => ItemMoves;
  onPick: (row: ItemRow) => void;
  onMove: (index: number, later: boolean) => void;
  onDelete: (() => void) | null;
}) => (
  <>
    <ItemListHeading rows={rows} summary={summary} className={headingClassName} />
    {rows.map((row) => (
      <ItemRowView
        key={row.index}
        row={row}
        selected={selected === row.index}
        onClick={() => onPick(row)}
        moves={movesOf(row.index)}
        onMove={(later) => onMove(row.index, later)}
        onDelete={row.index === HOME_ITEM ? null : onDelete}
      />
    ))}
  </>
);
