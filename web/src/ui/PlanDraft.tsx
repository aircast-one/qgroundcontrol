import { del, get, set } from 'idb-keyval';
import { optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';

export const PLAN_DRAFT_KEY = 'plan-draft';
const PLAN_DRAFT_VIEW = 'view.planDraft';
const RESTORE_DRAFT = 'core.plan.restoreDraft';

const restoring: { finished: () => void; done: Promise<void> } = { finished: () => undefined, done: Promise.resolve() };
restoring.done = new Promise<void>((resolve) => {
  restoring.finished = resolve;
});

export const restorePlanDraft = async (): Promise<boolean> => {
  try {
    const text = await get<unknown>(PLAN_DRAFT_KEY).catch(() => undefined);
    return typeof text === 'string' && text !== '' && (await Qgc.invokeResult(RESTORE_DRAFT, text)) === true;
  } finally {
    restoring.finished();
  }
};

export const keepPlanDraft = async (text: string): Promise<void> => {
  await restoring.done;
  await (text === '' ? del(PLAN_DRAFT_KEY) : set(PLAN_DRAFT_KEY, text)).catch(() => undefined);
};

export const PlanDraftKeeper = () => {
  const draft = useQgcPath(PLAN_DRAFT_VIEW);
  useLaunchedEffect([draft], async () => {
    if (draft !== null) await keepPlanDraft(optText(draft, 'text'));
  });
  return null;
};
