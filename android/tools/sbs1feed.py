import math
import os
import socket
import sys
import threading
import time

PORT = int(os.environ.get("SBS_PORT", "30003"))
CONTACTS = int(os.environ.get("SBS_CONTACTS", "3"))
ALERT = os.environ.get("SBS_ALERT") == "1"
CENTRE_LAT = float(os.environ.get("SIM_LAT", "41.7151"))
CENTRE_LON = float(os.environ.get("SIM_LON", "44.8271"))

FIELD_ICAO = 4
FIELD_CALLSIGN = 10
FIELD_ALTITUDE = 11
FIELD_GROUND_SPEED = 12
FIELD_TRACK = 13
FIELD_LATITUDE = 14
FIELD_LONGITUDE = 15
FIELD_VERTICAL_RATE = 16
FIELD_EMERGENCY = 19


def line(kind, fields):
    return ",".join(
        "MSG" if at == 0 else str(kind) if at == 1 else fields.get(at, "")
        for at in range(22)
    )


def contact_lines(index, elapsed):
    icao = "ABCDE%X" % index
    bearing = 2 * math.pi * index / max(1, CONTACTS) + elapsed * 0.02
    spread = 0.01 + 0.01 * index
    return [
        line(1, {FIELD_ICAO: icao, FIELD_CALLSIGN: " SWR%03d " % index}),
        line(3, {
            FIELD_ICAO: icao,
            FIELD_ALTITUDE: "%d" % (1000 + 500 * index),
            FIELD_LATITUDE: "%.6f" % (CENTRE_LAT + spread * math.cos(bearing)),
            FIELD_LONGITUDE: "%.6f" % (CENTRE_LON + spread * math.sin(bearing)),
            FIELD_EMERGENCY: "1" if ALERT and index == 0 else "0",
        }),
        line(4, {
            FIELD_ICAO: icao,
            FIELD_GROUND_SPEED: "%d" % (120 + 20 * index),
            FIELD_TRACK: "%.1f" % ((math.degrees(bearing) + 180.0) % 360.0),
            FIELD_VERTICAL_RATE: "%d" % (-64 * index),
        }),
    ]


def serve(connection, started):
    while True:
        elapsed = time.time() - started
        payload = "\r\n".join(
            text
            for index in range(CONTACTS)
            for text in contact_lines(index, elapsed)
        ) + "\r\n"
        connection.sendall(payload.encode())

        time.sleep(1.0)


def main():
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("0.0.0.0", PORT))
    listener.listen(1)
    print("listening on %d" % PORT, flush=True)
    def attend(connection, peer):
        print("attached %s" % (peer,), flush=True)
        try:
            serve(connection, time.time())
        except OSError as failure:
            print("detached %s: %s" % (peer, failure), flush=True)
        finally:
            connection.close()

    while True:
        connection, peer = listener.accept()
        threading.Thread(target=attend, args=(connection, peer), daemon=True).start()


if __name__ == "__main__":
    sys.exit(main())
