import { afterEach, describe, expect, it, vi } from 'vitest';
import { closeReason, socketHost } from './socket';

class FakeSocket {
  static readonly OPEN = 1;
  static made: FakeSocket[] = [];
  readyState = 0;
  binaryType = '';
  sent: Uint8Array[] = [];
  closed = false;
  onopen: (() => void) | null = null;
  onmessage: ((message: { data: ArrayBuffer }) => void) | null = null;
  onclose: ((closed: { code: number; reason: string }) => void) | null = null;
  constructor(readonly url: string) {
    FakeSocket.made = [...FakeSocket.made, this];
  }
  send(bytes: Uint8Array) {
    this.sent = [...this.sent, bytes];
  }
  close() {
    this.closed = true;
  }
}

const core = () => ({ socketOpened: vi.fn(), socketReceived: vi.fn(), socketClosed: vi.fn() });

describe('socketHost', () => {
  afterEach(() => {
    FakeSocket.made = [];
    vi.unstubAllGlobals();
  });

  it('holds what the core sends until the relay socket is open, then hands over every message', () => {
    vi.stubGlobal('WebSocket', FakeSocket);
    const told = core();
    const host = socketHost(told);
    expect(host.open(7, 'ws://falcon.local/fly/socket?tcp=10.0.0.9%3A30003')).toBe(true);
    const socket = FakeSocket.made[0];
    expect(socket?.binaryType).toBe('arraybuffer');
    host.send(7, new Uint8Array([1, 2]));
    expect(socket?.sent).toEqual([]);
    socket!.readyState = FakeSocket.OPEN;
    socket!.onopen?.();
    expect(socket?.sent).toEqual([new Uint8Array([1, 2])]);
    expect(told.socketOpened).toHaveBeenCalledWith(7);
    socket!.onmessage?.({ data: new Uint8Array([9]).buffer });
    expect(told.socketReceived).toHaveBeenCalledWith(7, new Uint8Array([9]));
    socket!.onclose?.({ code: 4000, reason: 'dial tcp 10.0.0.9:30003: connect: connection refused' });
    expect(told.socketClosed).toHaveBeenCalledWith(7, 'dial tcp 10.0.0.9:30003: connect: connection refused');
    expect(host.send(7, new Uint8Array([3]))).toBe(false);
  });

  it('a socket the core closes tells the core nothing more', () => {
    vi.stubGlobal('WebSocket', FakeSocket);
    const told = core();
    const host = socketHost(told);
    host.open(1, 'ws://d/fly/socket?udp=14550');
    const socket = FakeSocket.made[0];
    host.close(1);
    expect(socket?.closed).toBe(true);
    expect(socket?.onclose).toBeNull();
    expect(told.socketClosed).not.toHaveBeenCalled();
  });

  it('a clean close reads as the host hanging up and anything else keeps its reason', () => {
    expect(closeReason(1000, 'the host closed the connection')).toBe('');
    expect(closeReason(4000, 'listen udp :14550: address already in use')).toBe('listen udp :14550: address already in use');
    expect(closeReason(1006, '')).toBe('The device closed the connection (1006).');
  });
});
