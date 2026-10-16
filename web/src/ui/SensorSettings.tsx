import { useState } from 'react';
import { type Fact, Qgc, makeFact } from '@/bridge/Qgc';
import { type Json, isNull, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { Checkbox, DropdownMenu, DropdownMenuItem, OutlinedButton, Text } from '@/m3';
import { factFromControl } from '@/ui/ParameterForm';
import { SENSOR_SETTINGS_DIALOG, SETUP_DIALOG_OPENED } from '@/ui/ParameterLinks';
import { FactRow } from '@/ui/SettingsScreen';

export const SENSOR_SETTINGS_VIEW = 'view.sensorSettings';
export const SENSOR_SETTINGS_PRIORITY = 'sensorSettings.priority';

export type CompassSettings = { index: number; label: string; device: string; use: Fact | null; priority: number | null; orientation: Fact | null; orientationTitle: string };

export type Declination = { manual: boolean; autoDecPath: string; value: Fact | null };

export type SensorSettings = {
  boardRotation: Fact | null;
  boardTitle: string;
  compassesWhileCalibrating: boolean;
  compasses: readonly CompassSettings[];
  priorities: readonly string[];
  helpSet: string;
  helpCal: string;
  simpleAccelHelp: string;
  declination: Declination | null;
};

const controlFact = (json: Json, key: string): Fact | null => {
  const control = optJSONObject(json, key);
  return control === null ? null : factFromControl(control);
};

export const sensorSettings = (view: Json | null): SensorSettings | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const declination = optJSONObject(view, 'declination');
  return {
    boardRotation: controlFact(view, 'boardRotation'),
    boardTitle: optText(view, 'boardTitle'),
    compassesWhileCalibrating: optBoolean(view, 'compassesWhileCalibrating'),
    compasses: jsonObjects(optJSONArray(view, 'compasses')).map((compass) => ({
      index: optInt(compass, 'index'),
      label: optText(compass, 'label'),
      device: optText(compass, 'device'),
      use: controlFact(compass, 'use'),
      priority: isNull(compass, 'priority') ? null : optInt(compass, 'priority'),
      orientation: controlFact(compass, 'orientation'),
      orientationTitle: optText(compass, 'orientationTitle'),
    })),
    priorities: jsonStrings(optJSONArray(view, 'priorities')),
    helpSet: optText(view, 'helpSet'),
    helpCal: optText(view, 'helpCal'),
    simpleAccelHelp: optText(view, 'simpleAccelHelp'),
    declination: declination === null ? null : { manual: optBoolean(declination, 'manual'), autoDecPath: optText(declination, 'autoDecPath'), value: controlFact(declination, 'value') },
  };
};

export const ORIENTATIONS_HELP = 'Adjust orientations as needed.\n\nROTATION_NONE indicates component points in direction of flight.';

const useSensorSettings = (calibrating: boolean): readonly [SensorSettings | null, () => void] => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<SensorSettings | null>(null);
  useLaunchedEffect([], async () => {
    await Qgc.invoke(SETUP_DIALOG_OPENED, SENSOR_SETTINGS_DIALOG, calibrating);
  });
  useLaunchedEffect([revision], async (scope) => {
    const settings = sensorSettings(await Qgc.get(SENSOR_SETTINGS_VIEW));
    if (scope.isActive()) setRead(settings);
  });
  return [read, () => setRevision((it) => it + 1)];
};

export const CompassOrientations = () => {
  const [read, refresh] = useSensorSettings(false);
  const oriented = (read?.compasses ?? []).flatMap((compass) => (compass.orientation === null ? [] : [{ compass, orientation: compass.orientation }]));
  if (oriented.length === 0) return null;
  return (
    <div className="flex flex-col gap-1">
      <Text typography="bodySmall" className="block whitespace-pre-line">
        {ORIENTATIONS_HELP}
      </Text>
      {oriented.map(({ compass, orientation }) => (
        <FactRow key={compass.index} fact={orientation} title={compass.orientationTitle} onWrite={refresh} />
      ))}
    </div>
  );
};

export const SensorSettingsBlock = ({ calibrating, showCompasses, onSimpleAccel = null }: { calibrating: boolean; showCompasses: boolean; onSimpleAccel?: ((simple: boolean) => void) | null }) => {
  const [read, refresh] = useSensorSettings(calibrating);
  const [simple, setSimple] = useState(false);
  if (read === null) return null;
  const settings = read;
  return (
    <div className="flex flex-col gap-2">
      <Text typography="bodyMedium" className="block">
        {calibrating ? settings.helpCal : settings.helpSet}
      </Text>
      {settings.boardRotation !== null ? <FactRow fact={settings.boardRotation} title={settings.boardTitle} onWrite={refresh} /> : null}
      {onSimpleAccel !== null ? (
        <>
          <Text typography="bodySmall" className="block">
            {settings.simpleAccelHelp}
          </Text>
          <div className="flex items-center gap-2">
            <Checkbox
              checked={simple}
              onCheckedChange={(it) => {
                setSimple(it);
                onSimpleAccel(it);
              }}
              label="Simple accelerometer calibration"
            />
            <Text>Simple accelerometer calibration</Text>
          </div>
        </>
      ) : null}
      {showCompasses && (!calibrating || settings.compassesWhileCalibrating) ? (
        <>
          {settings.compasses.map((compass) => (
            <div key={compass.index} className="flex flex-col gap-2">
              <Text typography="titleSmall" className="block">
                {compass.label}
              </Text>
              {compass.device.trim() !== '' ? (
                <Text typography="bodySmall" className="block">
                  {compass.device}
                </Text>
              ) : null}
              {compass.use !== null ? <FactRow fact={compass.use} title="Use compass" onWrite={refresh} /> : null}
              {compass.priority !== null ? (
                <PriorityPicker
                  options={settings.priorities}
                  current={compass.priority}
                  onPick={(picked) =>
                    void (async () => {
                      await Qgc.invoke(SENSOR_SETTINGS_PRIORITY, compass.index, picked);
                      refresh();
                    })()
                  }
                />
              ) : null}
              {compass.orientation !== null ? <FactRow fact={compass.orientation} title={compass.orientationTitle} onWrite={refresh} /> : null}
            </div>
          ))}
          {settings.declination !== null ? <DeclinationRows declination={settings.declination} refresh={refresh} /> : null}
        </>
      ) : null}
    </div>
  );
};

const DeclinationRows = ({ declination, refresh }: { declination: Declination; refresh: () => void }) => (
  <>
    <Text typography="titleSmall" className="block">
      Magnetic declination
    </Text>
    <div className="flex items-center gap-2">
      <Checkbox
        checked={declination.manual}
        onCheckedChange={(manual) =>
          void (async () => {
            await Qgc.set(`${declination.autoDecPath}.rawValue`, manual ? 0 : 1);
            refresh();
          })()
        }
        label="Manual magnetic declination"
      />
      <Text>Manual magnetic declination</Text>
    </div>
    {declination.value !== null ? <FactRow fact={makeFact({ ...declination.value, enabled: declination.manual })} onWrite={refresh} /> : null}
  </>
);

const PriorityPicker = ({ options, current, onPick }: { options: readonly string[]; current: number; onPick: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative self-start">
      <OutlinedButton onClick={() => setOpen(true)}>{options[current] ?? ''}</OutlinedButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {options.map((option, index) => (
          <DropdownMenuItem
            key={index}
            text={option}
            onClick={() => {
              setOpen(false);
              onPick(index);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};
