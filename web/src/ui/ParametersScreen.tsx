import type { Fact } from '@/bridge/Qgc';

const PARAMETER_MANAGER = 'vehicle.parameterManager';
const DEFAULT_COMPONENT = -1;

export const parameterPath = (key: string, quantity: string | null = null): string => {
  const colon = key.indexOf(':');
  const head = colon < 0 ? '' : key.slice(0, colon);
  const component = /^[+-]?\d+$/.test(head) ? Number(head) : null;
  const name = component === null ? key : key.slice(colon + 1);
  return `${PARAMETER_MANAGER}.getParameter(${component ?? DEFAULT_COMPONENT},${name}${quantity === null ? '' : `,${quantity}`})`;
};

export const parameterFact = async (_name: string, _quantity: string | null = null): Promise<Fact | null> => null;
