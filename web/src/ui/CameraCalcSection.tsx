import { useState } from 'react';
import { type Json, isNull, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import type { Fact } from '@/bridge/Qgc';
import { DropdownMenu, DropdownMenuItem, FilterChip, OutlinedButton, Text } from '@/m3';
import { factFromControl } from './ParameterForm';

const DISTANCE_SUFFIX = 'cameraCalc.distanceToSurface';
const DENSITY_SUFFIX = 'cameraCalc.imageDensity';
const SPACING_SUFFIXES: ReadonlySet<string> = new Set(['cameraCalc.adjustedFootprintFrontal', 'cameraCalc.adjustedFootprintSide']);

export type CameraCalcBlock = {
  brand: string;
  model: string;
  brands: readonly string[];
  models: readonly string[];
  manual: boolean;
  custom: boolean;
  valueSetIsDistance: boolean;
  valueSetIsDistancePath: string;
  brandPath: string;
  modelPath: string;
  facts: readonly (readonly [string, Fact])[];
  distanceMode: number | null;
  distanceModes: readonly (readonly [number, string])[];
  distanceModePath: string;
};

export const distanceModeTitle = (block: CameraCalcBlock): string | null => block.distanceModes.find(([raw]) => raw === block.distanceMode)?.[1] ?? null;

export const cameraCalc = (view: Json | null): CameraCalcBlock | null => {
  const block = optJSONObject(view, 'camera');
  if (block === null) return null;
  const brand = optText(block, 'brand');
  return {
    brand,
    model: optText(block, 'model'),
    brands: jsonStrings(optJSONArray(block, 'brands')),
    models: jsonStrings(optJSONArray(block, 'models')),
    manual: brand === optText(block, 'manualName'),
    custom: optBoolean(block, 'custom'),
    valueSetIsDistance: optBoolean(block, 'valueSetIsDistance', true),
    valueSetIsDistancePath: optText(block, 'valueSetIsDistancePath'),
    brandPath: optText(block, 'brandPath'),
    modelPath: optText(block, 'modelPath'),
    distanceMode: isNull(block, 'distanceMode') ? null : optInt(block, 'distanceMode'),
    distanceModes: jsonObjects(optJSONArray(block, 'distanceModes')).map((it) => [optInt(it, 'raw'), optText(it, 'title')] as const),
    distanceModePath: optText(block, 'distanceModePath'),
    facts: jsonObjects(optJSONArray(block, 'facts')).flatMap((control) => {
      const fact = factFromControl(control);
      return fact === null ? [] : [[optText(control, 'pathSuffix'), fact] as const];
    }),
  };
};

export const distanceLabel = (block: CameraCalcBlock): string => {
  const label = block.facts.find(([suffix]) => suffix === DISTANCE_SUFFIX)?.[1].shortLabel ?? '';
  return label.trim() === '' ? 'Altitude' : label;
};

export const shownCameraFacts = (block: CameraCalcBlock): readonly Fact[] =>
  block.facts
    .filter(([suffix]) =>
      block.manual
        ? suffix === DISTANCE_SUFFIX || SPACING_SUFFIXES.has(suffix)
        : suffix === DISTANCE_SUFFIX
          ? block.valueSetIsDistance
          : suffix === DENSITY_SUFFIX
            ? !block.valueSetIsDistance
            : true,
    )
    .map(([, fact]) => fact);

export const CameraCalcHeader = ({ block, onWrite }: { block: CameraCalcBlock; onWrite: (path: string, value: unknown) => void }) => {
  const current = distanceModeTitle(block);
  return (
    <div className="flex w-full flex-col gap-2 px-5 py-2">
      <Text typography="titleSmall">Camera</Text>
      <div className="flex flex-wrap items-center gap-2">
        <Choice label={block.brand} options={block.brands} onPick={(it) => onWrite(block.brandPath, it)} />
        {!block.manual && !block.custom ? <Choice label={block.model === '' ? 'Model' : block.model} options={block.models} onPick={(it) => onWrite(block.modelPath, it)} /> : null}
      </div>
      {current !== null ? (
        <div className="flex flex-wrap items-center gap-2">
          <Text typography="bodyMedium">Altitude</Text>
          <Choice
            label={current}
            options={block.distanceModes.map(([, title]) => title)}
            onPick={(title) => {
              const mode = block.distanceModes.find(([, it]) => it === title);
              if (mode !== undefined) onWrite(block.distanceModePath, mode[0]);
            }}
          />
        </div>
      ) : null}
      {!block.manual ? (
        <div className="flex flex-wrap items-center gap-2">
          <Text typography="bodyMedium">Set by</Text>
          <FilterChip selected={block.valueSetIsDistance} onClick={() => onWrite(block.valueSetIsDistancePath, true)} label={distanceLabel(block)} />
          <FilterChip selected={!block.valueSetIsDistance} onClick={() => onWrite(block.valueSetIsDistancePath, false)} label="Ground res" />
        </div>
      ) : null}
    </div>
  );
};

const Choice = ({ label, options, onPick }: { label: string; options: readonly string[]; onPick: (option: string) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <OutlinedButton onClick={() => setOpen(true)} enabled={options.length > 0}>
        {label}
      </OutlinedButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {options.map((option) => (
          <DropdownMenuItem
            key={option}
            text={option}
            onClick={() => {
              setOpen(false);
              onPick(option);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};
