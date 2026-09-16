"""Make the UDP rig reachable over the TCP path the emulator needs.

    python3 tools/apmvehicle.py 127.0.0.1 &     # sends MAVLink to UDP 14550
    python3 tools/udptcp.py &                   # serves it on TCP 5771
    adb reverse tcp:5771 tcp:5771

apmvehicle.py only speaks UDP, and `adb reverse` carries TCP only, so the
emulator cannot reach it. Rather than write a second MAVLink source, this
relays the one that already works: datagrams out to every TCP client, and
anything a client sends back to every vehicle address it has heard from, so
parameter and mission requests still get answers. Every address rather than the
newest: two fakes are two addresses, and one vehicle's command must not land on
the other.
"""

import os
import socket
import threading

UDP_PORT = int(os.environ.get("RIG_UDP_PORT", "14550"))
TCP_PORT = int(os.environ.get("RIG_TCP_PORT", "5771"))

clients: set[socket.socket] = set()
lock = threading.Lock()
vehicles: set[tuple[str, int]] = set()


def pump_udp(udp: socket.socket) -> None:
    while True:
        data, source = udp.recvfrom(65535)
        with lock:
            vehicles.add(source)
            for client in list(clients):
                try:
                    client.sendall(data)
                except OSError:
                    clients.discard(client)
                    client.close()


def serve(client: socket.socket, udp: socket.socket) -> None:
    with lock:
        clients.add(client)
    print(f"client {client.getpeername()} - {len(clients)} connected", flush=True)
    try:
        while True:
            data = client.recv(65535)
            if not data:
                break
            with lock:
                targets = list(vehicles)
            for target in targets:
                udp.sendto(data, target)
    except OSError:
        pass
    finally:
        with lock:
            clients.discard(client)
        client.close()
        print(f"client gone - {len(clients)} connected", flush=True)


def main() -> None:
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    udp.bind(("127.0.0.1", UDP_PORT))
    threading.Thread(target=pump_udp, args=(udp,), daemon=True).start()

    tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    tcp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    tcp.bind(("0.0.0.0", TCP_PORT))
    tcp.listen(8)
    print(f"relaying UDP {UDP_PORT} to TCP {TCP_PORT}", flush=True)
    while True:
        client, _ = tcp.accept()
        client.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        threading.Thread(target=serve, args=(client, udp), daemon=True).start()


if __name__ == "__main__":
    main()
