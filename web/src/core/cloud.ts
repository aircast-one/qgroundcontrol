type CloudCore = {
  cloudOpened(id: number): void;
  cloudReceived(id: number, bytes: Uint8Array): void;
  cloudClosed(id: number, reason: string): void;
};

type Session = { stopped: boolean; transport: WebTransport | null; writer: WritableStreamDefaultWriter<Uint8Array> | null };

const INITIAL_BACKOFF_MS = 2000;
const MAX_BACKOFF_MS = 60_000;
const RELAY_LOST = 'The Aircast relay connection was lost. Reconnecting.';

export const cloudBackoffMs = (failures: number): number => Math.min(INITIAL_BACKOFF_MS * 2 ** Math.min(failures, 5), MAX_BACKOFF_MS);

export const relayTokenRefusal = (status: number): string =>
  status === 401 ? 'Sign in to your Aircast account again.' : status === 403 ? 'This account cannot reach that device.' : `The Aircast account server answered ${status}.`;

const relayToken = async (tokenUrl: string, bearer: string): Promise<string> => {
  const answer = await fetch(tokenUrl, { headers: { Authorization: `Bearer ${bearer}` } });
  if (!answer.ok) throw new Error(relayTokenRefusal(answer.status));
  const { token } = (await answer.json()) as { token?: string };
  if (token === undefined || token === '') throw new Error('The Aircast account server gave no relay token.');
  return token;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const cloudHost = (core: CloudCore) => {
  const sessions = new Map<number, Session>();

  const connectOnce = async (id: number, session: Session, tokenUrl: string, sessionUrl: string, bearer: string): Promise<{ reason: string; reached: boolean }> => {
    try {
      const token = await relayToken(tokenUrl, bearer);
      const transport = new WebTransport(`${sessionUrl}?token=${encodeURIComponent(token)}`);
      session.transport = transport;
      await transport.ready;
      session.writer = transport.datagrams.writable.getWriter();
      core.cloudOpened(id);
      for await (const bytes of transport.datagrams.readable as unknown as AsyncIterable<Uint8Array>) {
        core.cloudReceived(id, bytes);
      }
      return { reason: RELAY_LOST, reached: true };
    } catch (failure) {
      return { reason: failure instanceof Error ? failure.message : String(failure), reached: session.writer !== null };
    } finally {
      session.writer = null;
      session.transport = null;
    }
  };

  const keepConnected = async (id: number, tokenUrl: string, sessionUrl: string, bearer: string, failures: number): Promise<void> => {
    const session = sessions.get(id);
    if (session === undefined || session.stopped) return;
    const { reason, reached } = await connectOnce(id, session, tokenUrl, sessionUrl, bearer);
    if (session.stopped) return;
    core.cloudClosed(id, reason);
    const next = reached ? 0 : failures + 1;
    await delay(cloudBackoffMs(next));
    return keepConnected(id, tokenUrl, sessionUrl, bearer, next);
  };

  return {
    open: (id: number, tokenUrl: string, sessionUrl: string, bearer: string): boolean => {
      if (typeof WebTransport === 'undefined') return false;
      sessions.set(id, { stopped: false, transport: null, writer: null });
      void keepConnected(id, tokenUrl, sessionUrl, bearer, 0);
      return true;
    },
    send: (id: number, bytes: Uint8Array): boolean => {
      const writer = sessions.get(id)?.writer ?? null;
      if (writer === null) return false;
      writer.write(new Uint8Array(bytes)).catch(() => undefined);
      return true;
    },
    close: (id: number): void => {
      const session = sessions.get(id);
      if (session === undefined) return;
      session.stopped = true;
      session.transport?.close();
      sessions.delete(id);
    },
  };
};
