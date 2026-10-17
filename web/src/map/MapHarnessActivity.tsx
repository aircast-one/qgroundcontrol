import { useState } from 'react';
import { FlyMapEdits, LocalFlyMapEdits } from './FlyMapEdits';
import { PlanMapScreen } from './PlanMap';
import { AircastTheme } from './Theme';

export const MapHarnessActivity = () => {
  const [edits] = useState(() => new FlyMapEdits());
  return (
    <AircastTheme>
      <LocalFlyMapEdits.Provider value={edits}>
        <PlanMapScreen className="size-full" />
      </LocalFlyMapEdits.Provider>
    </AircastTheme>
  );
};
