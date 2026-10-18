# Aircast USB radio

`aircast-usb.lua` turns an EdgeTX radio into a USB joystick for the Aircast app. It streams every channel the model mixes over the radio's USB serial port, and shows on the radio whether the app is getting them.

## Install on the radio

1. Plug the radio into a computer and pick **USB Storage (SD)** on the radio.
2. Copy `aircast-usb.lua` to `SCRIPTS/TOOLS/` on the SD card. Delete `SCRIPTS/TOOLS/aircast-usb.luac` if it is there, so the radio does not keep running an older compiled copy.
3. Eject the card and unplug.
4. On the radio, open **SYS → Hardware** and set **USB-VCP** to **LUA**.

## Connect it to the app

1. Plug the radio into the device running the app and pick **USB Serial (VCP)** on the radio.
2. On the radio, open **SYS → Tools → Aircast USB radio**.
3. In the app, open **Settings → Transmission → Links → Advanced → Radio on USB (EdgeTX script)** and pick the radio as **Device**. In a browser, press **Allow a USB port** there first and choose the radio.

## Use the web app from a device over plain http

Chrome lets only `https` and `localhost` pages use USB. The app an Aircast device serves at `http://<device>.local/fly/` is neither, so Chrome hides USB and the app shows how to fix it instead of **Allow a USB port**. Once per computer:

1. In Chrome open `chrome://flags/#unsafely-treat-insecure-origin-as-secure`.
2. Add the device's address exactly as the address bar shows it, for example `http://falcon.local`. Separate several devices with commas.
3. Set the flag to **Enabled** and press **Relaunch**.

Safari and Firefox have no USB serial in web pages at all; use Chrome or Edge.

## What the radio says

| On the radio | Meaning |
|---|---|
| App flying vehicle N | The app is receiving the sticks and flying vehicle N with them |
| App: 4G standby, vehicle N | The app is receiving the sticks and takes over if the vehicle's own radio link fails |
| A joystick is flying vehicle N | The app is receiving, but another joystick is flying |
| App receiving, no vehicle connected | The app is receiving, but no vehicle is linked |
| Waiting for the Aircast app | Nothing has answered for 1.5 s: the app is closed, the radio is not picked in Settings, or the app is older than this script |
| USB cable not connected | The radio sees no USB cable |

**RTN** leaves the script. While the app is flying on these sticks, the first RTN only asks; press it again within 3 s to stop sending sticks.

## Test

```sh
lua5.4 tools/edgetx/aircast-usb.test.lua
luajit tools/edgetx/aircast-usb.test.lua
```

The test runs the script without string methods and without the `//`, `&`, `<<`, `>>` operators, as EdgeTX 2.9's Lua does.
