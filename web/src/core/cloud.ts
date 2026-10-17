type CloudCore = {
  cloudOpened(id: number): void;
  cloudReceived(id: number, bytes: Uint8Array): void;
  cloudClosed(id: number, reason: string): void;
};

type Session = { stopped: boolean; send: ((bytes: Uint8Array) => void) | null; close: (() => void) | null };

type Attempt = { reason: string; reached: boolean };

const INITIAL_BACKOFF_MS = 2000;
const MAX_BACKOFF_MS = 60_000;
const RELAY_LOST = 'The Aircast relay connection was lost. Reconnecting.';
const SOCKET_LOST = 'The WebSocket link was lost. Reconnecting.';

export const cloudBackoffMs = (failures: number): number => Math.min(INITIAL_BACKOFF_MS * 2 ** Math.min(failures, 5), MAX_BACKOFF_MS);

export const relayTokenRefusal = (status: number): string =>
  status === 401 ? 'Sign in to your Aircast account again.' : status === 403 ? 'This account cannot reach that device.' : `The Aircast account server answered ${status}.`;

export const usesWebSocket = (tokenUrl: string, sessionUrl: string): boolean => tokenUrl === '' && /^wss?:\/\//.test(sessionUrl);

const relayToken = async (tokenUrl: string, bearer: string): Promise<string> => {
  const answer = await fetch(tokenUrl, { headers: { Authorization: `Bearer ${bearer}` } });
  if (!answer.ok) throw new Error(relayTokenRefusal(answer.status));
  const { token } = (await answer.json()) as { token?: string };
  if (token === undefined || token === '') throw new Error('The Aircast account server gave no relay token.');
  return token;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const failed = (failure: unknown, session: Session): Attempt => ({ reason: failure instanceof Error ? failure.message : String(failure), reached: session.send !== null });

export const cloudHost = (core: CloudCore) => {
  const sessions = new Map<number, Session>();

  const relayOnce = async (id: number, session: Session, tokenUrl: string, sessionUrl: string, bearer: string): Promise<Attempt> => {
    try {
      const token = await relayToken(tokenUrl, bearer);
      const transport = new WebTransport(`${sessionUrl}?token=${encodeURIComponent(token)}`);
      session.close = () => transport.close();
      await transport.ready;
      const writer = transport.datagrams.writable.getWriter();
      session.send = (bytes) => void writer.write(new Uint8Array(bytes)).catch(() => undefined);
      core.cloudOpened(id);
      for await (const bytes of transport.datagrams.readable as unknown as AsyncIterable<Uint8Array>) {
        core.cloudReceived(id, bytes);
      }
      return { reason: RELAY_LOST, reached: true };
    } catch (failure) {
      return failed(failure, session);
    } finally {
      session.send = null;
      session.close = null;
    }
  };

  const socketOnce = async (id: number, session: Session, url: string): Promise<Attempt> => {
    try {
      const socket = new WebSocket(url);
      socket.binaryType = 'arraybuffer';
      session.close = () => socket.close();
      await new Promise<void>((resolve, reject) => {
        socket.onopen = () => resolve();
        socket.onerror = () => reject(new Error(`No MAVLink WebSocket answered at ${url}`));
      });
      session.send = (bytes) => socket.send(new Uint8Array(bytes));
      core.cloudOpened(id);
      await new Promise<void>((resolve) => {
        socket.onmessage = ({ data }) => core.cloudReceived(id, new Uint8Array(data as ArrayBuffer));
        socket.onclose = () => resolve();
      });
      return { reason: SOCKET_LOST, reached: true };
    } catch (failure) {
      return failed(failure, session);
    } finally {
      session.send = null;
      session.close = null;
    }
  };

  const keepConnected = async (id: number, tokenUrl: string, sessionUrl: string, bearer: string, failures: number): Promise<void> => {
    const session = sessions.get(id);
    if (session === undefined || session.stopped) return;
    const { reason, reached } = usesWebSocket(tokenUrl, sessionUrl) ? await socketOnce(id, session, sessionUrl) : await relayOnce(id, session, tokenUrl, sessionUrl, bearer);
    if (session.stopped) return;
    core.cloudClosed(id, reason);
    const next = reached ? 0 : failures + 1;
    await delay(cloudBackoffMs(next));
    return keepConnected(id, tokenUrl, sessionUrl, bearer, next);
  };

  return {
    open: (id: number, tokenUrl: string, sessionUrl: string, bearer: string): boolean => {
      if (!usesWebSocket(tokenUrl, sessionUrl) && typeof WebTransport === 'undefined') return false;
      sessions.set(id, { stopped: false, send: null, close: null });
      void keepConnected(id, tokenUrl, sessionUrl, bearer, 0);
      return true;
    },
    send: (id: number, bytes: Uint8Array): boolean => {
      const send = sessions.get(id)?.send ?? null;
      if (send === null) return false;
      send(bytes);
      return true;
    },
    close: (id: number): void => {
      const session = sessions.get(id);
      if (session === undefined) return;
      session.stopped = true;
      session.close?.();
      sessions.delete(id);
    },
  };
};
