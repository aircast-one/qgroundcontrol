import { useState } from 'react';
import { Button, Text } from '@/m3';
import { AircastTheme } from '@/map/Theme';
import { md } from '@/theme/Colors';
import { takeOverHere } from '@/app/SingleTab';

export const OTHER_TAB_TITLE = 'Aircast is open in another tab';
export const OTHER_TAB_TEXT = 'Only one tab flies at a time, so links and settings stay in one place. Use it here to move it from the other tab.';

export const OtherTabScreen = () => {
  const [moving, setMoving] = useState(false);
  return (
    <AircastTheme dark>
      <main className="flex h-full w-full items-center justify-center bg-surface p-page text-on-surface">
        <div className="flex max-w-[480px] flex-col gap-form">
          <h1>
            <Text typography="headlineSmall">{OTHER_TAB_TITLE}</Text>
          </h1>
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {OTHER_TAB_TEXT}
          </Text>
          <Button
            enabled={!moving}
            className="self-start"
            onClick={() => {
              setMoving(true);
              void takeOverHere();
            }}
          >
            {moving ? 'Moving…' : 'Use here'}
          </Button>
        </div>
      </main>
    </AircastTheme>
  );
};
