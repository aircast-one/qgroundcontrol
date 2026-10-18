type SocketCore = {
  socketOpened(id: number): void;
  socketReceived(id: number, bytes: Uint8Array): void;
  socketClosed(id: number, reason: string): void;
};

type Socket = { socket: WebSocket; waiting: Uint8Array<ArrayBuffer>[] };

export const closeReason = (code: number, reason: string): string => (code === 1000 ? '' : reason || `The device closed the connection (${code}).`);

export const socketHost = (core: SocketCore) => {
  const sockets = new Map<number, Socket>();

  return {
    open(id: number, url: string): boolean {
      try {
        const entry: Socket = { socket: new WebSocket(url), waiting: [] };
        entry.socket.binaryType = 'arraybuffer';
        entry.socket.onopen = () => {
          entry.waiting.forEach((bytes) => entry.socket.send(bytes));
          entry.waiting = [];
          core.socketOpened(id);
        };
        entry.socket.onmessage = (message) => core.socketReceived(id, new Uint8Array(message.data as ArrayBuffer));
        entry.socket.onclose = (closed) => {
          sockets.delete(id);
          core.socketClosed(id, closeReason(closed.code, closed.reason));
        };
        sockets.set(id, entry);
        return true;
      } catch {
        return false;
      }
    },
    send(id: number, bytes: Uint8Array<ArrayBuffer>): boolean {
      const entry = sockets.get(id);
      if (entry === undefined) return false;
      if (entry.socket.readyState === WebSocket.OPEN) entry.socket.send(bytes);
      else entry.waiting = [...entry.waiting, bytes];
      return true;
    },
    close(id: number): void {
      const entry = sockets.get(id);
      sockets.delete(id);
      if (entry === undefined) return;
      entry.socket.onclose = null;
      entry.socket.close();
    },
  };
};
