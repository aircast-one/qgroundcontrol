import type { PlanStat } from '@/map/PlanStrip';

export const planStatsLine = (stats: readonly PlanStat[]): string =>
  stats
    .flatMap((stat) =>
      stat.label === 'Items' ? (stat.value === '0' ? [] : [`${stat.value} ${stat.value === '1' ? 'item' : 'items'}`]) : stat.label === 'Max alt' ? [`max ${stat.value}`] : [stat.value],
    )
    .join(' · ');

export const headerLine = (status: string, syncing: boolean, stats: readonly PlanStat[]): string =>
  syncing || !stats.some((it) => it.label === 'Items' && it.value !== '0') ? status : planStatsLine(stats);

export const PlanTab = (_props: { className?: string; onFly?: () => void }) => null;
