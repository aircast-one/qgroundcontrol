# Aircast USB radio

`aircast-usb.lua` turns an EdgeTX radio into a USB joystick for the Aircast app. It streams every channel the model mixes over the radio's USB serial port, and shows on the radio whether the app is getting them.

## Needs EdgeTX 2.10 or newer

The radio streams its sticks on any EdgeTX, but on 2.9 and older it never hears the app: `serialRead()` returns nothing on firmware built with the serial CLI, which includes the standard builds (EdgeTX #4605, fixed in 2.10.0 by #4644). The radio then stays on "Waiting for the app" however the app is set up. Check **SYS → About** and update first; a TX16S on 2.9.4 answered as soon as it ran 2.12.5.

## Install on the radio

1. Plug the radio into a computer and pick **USB Storage (SD)** on the radio.
2. Copy `aircast-usb.lua` to `SCRIPTS/TOOLS/` on the SD card. Delete `SCRIPTS/TOOLS/aircast-usb.luac` if it is there, so the radio does not keep running an older compiled copy.
3. Eject the card and unplug.
4. On the radio, open **SYS → Hardware** and set **USB-VCP** to **LUA**.

## Connect it to the app

1. Plug the radio into the device running the app and pick **USB Serial (VCP)** on the radio.
2. On the radio, open **SYS → Tools → Aircast USB radio**.
3. The app recognises the radio by the stick lines it sends and asks "An EdgeTX radio is streaming its sticks on USB. Fly with it?" — press **Use**. To pick it yourself, open **Links → Add link… → RC transmitter (EdgeTX script)**, or **Settings → Control → Radio on USB (EdgeTX script) → Device**. In a browser, allow the USB port there first and choose the radio.

Do not add the radio as a **USB telemetry radio**: that link reads MAVLink, which the radio does not send.

## Use the web app from a device over plain http

Chrome lets only `https` and `localhost` pages use USB. The app an Aircast device serves at `http://<device>.local/fly/` is neither, so Chrome hides USB and the app shows how to fix it instead of **Allow a USB port**.

Open the same page over https: the app's **Open over https** button goes to `https://<device>.local/fly/`. Accept the device's certificate once and USB works there.

Or, once per computer, let Chrome treat the http page as secure:

1. In Chrome open `chrome://flags/#unsafely-treat-insecure-origin-as-secure`.
2. Add the device as `http://` and its host name only, without `/fly/`: for example `http://falcon.local`. Separate several devices with commas.
3. Set the flag to **Enabled** and press **Relaunch**.

Safari and Firefox have no USB serial in web pages at all; use Chrome or Edge.

## What the radio says

| On the radio | Meaning |
|---|---|
| App flying vehicle N | The app is receiving the sticks and flying vehicle N with them |
| App: 4G standby, vehicle N | The app is receiving the sticks and takes over if the vehicle's own radio link fails |
| A joystick is flying vehicle N | The app is receiving, but another joystick is flying |
| App receiving, no vehicle connected | The app is receiving, but no vehicle is linked |
| Waiting for the app · In app: Control > Radio on USB | Nothing has answered for 1.5 s: the app is closed, the radio is not picked under Control > Radio on USB, or the app is older than this script |
| USB cable not connected | The radio sees no USB cable |

**RTN** leaves the script. While the app is flying on these sticks, the first RTN only asks; press it again within 3 s to stop sending sticks.

## Channel names

Each row shows the channel the app receives, then what drives it. While the app answers and knows the vehicle, the stick channels are named by what the vehicle does with them, following its RCMAP (ArduPilot) or RC_MAP (PX4) parameters: Roll, Pitch, Thr, Yaw on a copter, helicopter or sub; Ail, Ele, Thr, Rud on a plane or VTOL; Steer and Thr on a rover or boat. Every other channel, and every channel while the app is silent, keeps EdgeTX's source name. EdgeTX itself names the sticks Ail, Ele, Thr and Rud on every model.

The app answers `@N` with no vehicle, otherwise `@<F|S|J>,<vehicle>,<C|P|R>,<roll>,<pitch>,<throttle>,<yaw>`, dropping everything after the vehicle when it does not know the vehicle type.

## Test

```sh
lua5.4 tools/edgetx/aircast-usb.test.lua
luajit tools/edgetx/aircast-usb.test.lua
```

The test runs the script without string methods and without the `//`, `&`, `<<`, `>>` operators, as EdgeTX 2.9's Lua does.
