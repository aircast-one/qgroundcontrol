export type ItemRow = {
  index: number;
  number: string;
  name: string;
  detail: string;
  colour: string;
  placed: boolean;
  readyForSave: boolean;
  seal: string;
};

export const ItemRow = (fields: { index: number; number: string; name: string; detail: string; colour: string; placed: boolean; readyForSave?: boolean; seal?: string }): ItemRow => ({
  readyForSave: true,
  seal: fields.number,
  ...fields,
});
