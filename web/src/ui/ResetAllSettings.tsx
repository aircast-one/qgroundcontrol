import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { opt } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { AlertDialog, OutlinedButton, Text, TextButton } from '@/m3';

export const GENERAL_PAGE = 'General';
export const CLEAR_SETTINGS_NEXT_BOOT = 'settings.appSettings.clearSettingsNextBoot';

export const resetPending = (value: unknown): boolean => value === true || value === 1 || value === 'true';

export const ResetAllSettingsRow = () => {
  const [reads, setReads] = useState(0);
  const [pending, setPending] = useState(false);
  const [asking, setAsking] = useState(false);

  useLaunchedEffect([reads], async (scope) => {
    const read = resetPending(opt(await Qgc.get(CLEAR_SETTINGS_NEXT_BOOT), 'value'));
    if (scope.isActive()) setPending(read);
  });

  const write = (value: boolean) =>
    void (async () => {
      await Qgc.set(CLEAR_SETTINGS_NEXT_BOOT, value);
      setReads((it) => it + 1);
    })();

  return (
    <>
      <div className="flex w-full items-center px-5 py-3">
        <Text typography="bodySmall" className="flex-1">
          {pending ? 'All settings will be cleared on next start' : ''}
        </Text>
        <OutlinedButton onClick={() => (pending ? write(false) : setAsking(true))}>{pending ? 'Cancel reset' : 'Reset all settings…'}</OutlinedButton>
      </div>
      {asking ? (
        <AlertDialog
          onDismissRequest={() => setAsking(false)}
          title="Reset all settings"
          text="All settings will be cleared the next time Aircast starts. This cannot be undone."
          confirmButton={
            <TextButton
              onClick={() => {
                setAsking(false);
                write(true);
              }}
            >
              Ok
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setAsking(false)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};
