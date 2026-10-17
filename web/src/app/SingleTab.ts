const CORE_LOCK = 'aircast-core';
const TAB_CHANNEL = 'aircast-tabs';
const TAKEOVER = 'takeover';
const YIELDED = 'aircast.yielded';

const yielded = (): boolean => {
  try {
    return sessionStorage.getItem(YIELDED) === '1';
  } catch {
    return false;
  }
};

const markYielded = (on: boolean): void => {
  try {
    if (on) sessionStorage.setItem(YIELDED, '1');
    else sessionStorage.removeItem(YIELDED);
  } catch {
    return;
  }
};

const handOverOnRequest = (): void => {
  const channel = new BroadcastChannel(TAB_CHANNEL);
  channel.onmessage = ({ data }) => {
    if (data !== TAKEOVER) return;
    markYielded(true);
    location.reload();
  };
};

export const claimTab = (): Promise<boolean> =>
  !('locks' in navigator) || typeof BroadcastChannel === 'undefined'
    ? Promise.resolve(true)
    : yielded()
      ? Promise.resolve(false)
      : new Promise<boolean>((resolve) => {
          void navigator.locks.request(CORE_LOCK, { ifAvailable: true }, (lock) => {
            if (lock === null) {
              resolve(false);
              return undefined;
            }
            resolve(true);
            handOverOnRequest();
            return new Promise<void>(() => undefined);
          });
        });

export const takeOverHere = async (): Promise<void> => {
  markYielded(false);
  new BroadcastChannel(TAB_CHANNEL).postMessage(TAKEOVER);
  await navigator.locks.request(CORE_LOCK, async () => undefined);
  location.reload();
};
