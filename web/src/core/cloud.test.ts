import { afterEach, describe, expect, it, vi } from 'vitest';
import { SOCKET_ANSWER_MS, cloudBackoffMs, relayTokenRefusal, socketAnswered, usesWebSocket } from './cloud';

describe('the browser cloud relay', () => {
  it('backs off like the native relay, doubling from 2 s to a 60 s ceiling', () => {
    expect([0, 1, 4, 5, 9].map(cloudBackoffMs)).toEqual([2000, 4000, 32_000, 60_000, 60_000]);
  });

  it('names why the account server refused a relay token', () => {
    expect(relayTokenRefusal(401)).toBe('Sign in to your Aircast account again.');
    expect(relayTokenRefusal(403)).toBe('This account cannot reach that device.');
    expect(relayTokenRefusal(500)).toBe('The Aircast account server answered 500.');
  });

  it('opens a plain WebSocket for a saved WebSocket link, which needs no relay token', () => {
    expect(usesWebSocket('', 'wss://sitl.aircast.one/ws/mavlink')).toBe(true);
    expect(usesWebSocket('', 'ws://192.168.1.10:5760')).toBe(true);
    expect(usesWebSocket('https://api.aircast.one/v1/user/devices/d/webtransport-token', 'https://api.aircast.one/v1/mavlink/device/d/webtransport')).toBe(false);
  });

  describe('a WebSocket that never answers', () => {
    afterEach(() => vi.useRealTimers());

    const silentSocket = () => ({ onopen: null, onerror: null, close: vi.fn() }) as unknown as Pick<WebSocket, 'onopen' | 'onerror' | 'close'>;

    it('is given up within seconds, so a blackholed address is named as unreachable instead of waiting out the browser', async () => {
      vi.useFakeTimers();
      const socket = silentSocket();
      const answer = socketAnswered(socket, 'ws://10.255.255.1:5760');
      const settled = expect(answer).rejects.toThrow('No MAVLink WebSocket answered at ws://10.255.255.1:5760');
      await vi.advanceTimersByTimeAsync(SOCKET_ANSWER_MS);
      await settled;
      expect(socket.close).toHaveBeenCalledOnce();
      expect(SOCKET_ANSWER_MS).toBeLessThanOrEqual(10_000);
    });

    it('that opens in time is kept', async () => {
      vi.useFakeTimers();
      const socket = silentSocket();
      const answer = socketAnswered(socket, 'ws://192.168.1.10:5760');
      socket.onopen?.call(socket as WebSocket, new Event('open'));
      await expect(answer).resolves.toBeUndefined();
      await vi.advanceTimersByTimeAsync(SOCKET_ANSWER_MS);
      expect(socket.close).not.toHaveBeenCalled();
    });
  });
});
