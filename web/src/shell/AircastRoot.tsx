import { useQgcPath } from '@/bridge/QgcState';
import { LinkCommands } from '@/bridge/BridgeCommands';
import { optBoolean, optText } from '@/bridge/Json';
import { MaterialIcon } from '@/res/Icon';

export const AircastRoot = () => {
  const fly = useQgcPath('view.flyState');
  return (
    <main className="grid h-full place-items-center bg-surface text-on-surface">
      <section className="grid w-[min(420px,calc(100vw-32px))] gap-3 rounded-shape-lg bg-surface-container-high p-5">
        <h1 className="flex items-center gap-2 text-title-large"><MaterialIcon name="flight" /> Aircast</h1>
        <p className="text-body-medium text-on-surface-variant">{optBoolean(fly, 'connected') ? `${optText(fly, 'mode')} · ${optText(fly, 'stateText')}` : 'No vehicle connected'}</p>
        <button className="rounded-shape-md bg-primary px-4 py-3 text-label-large text-on-primary" onClick={() => void LinkCommands.startMock(["apmCopter"])}>
          Start mock link
        </button>
      </section>
    </main>
  );
};
