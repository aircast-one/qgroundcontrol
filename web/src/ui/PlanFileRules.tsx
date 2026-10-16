import { type Json, optText } from '@/bridge/Json';

export const planStatusText = (view: Json | null): string => optText(view, 'status');

export const planTitle = (documentName: string | null): string => {
  if (documentName === null || documentName.trim() === '') return 'Untitled Plan';
  const dot = documentName.lastIndexOf('.');
  return dot > 0 ? documentName.slice(0, dot) : documentName;
};
