# iOS and macOS ↔ Android feature parity

The native Apple head (`ios/`) and the native Android head (`android/`) sit on the same Rust core and read
the same core paths. The same `ios/Sources` build two apps: the iOS app (target `Aircast`) and a native
macOS app (target `AircastMac`, no Qt), with platform differences behind `#if os(macOS)`. Every Android
feature that the platform can support is ported. This file records what Android has that each Apple app
deliberately does not, so a future gap check does not re-open them.

## How the gap was measured

1. Android-only source files: `comm -23` of the Kotlin and Swift file basenames under
   `android/app/src/main/java` and `ios/Sources`.
2. Android-only core paths: every quoted `name.name` literal in the Android sources that the iOS sources
   never reference. This catches features hidden inside files both heads share.

Re-run both after porting something; anything the two lists show that is not below is a gap. Three entries
are expected and are not gaps: `QgcCalls.kt` (Kotlin's off-main helpers, which iOS has in `Bridge/Qgc.swift`),
`cameraCalc.frontalOverlap`/`sideOverlap` (iOS shows both overlaps through its default case), and
`mavlinkInspector.activeSystem.*` (unused constants on Android).

## Ported to iOS

| Area | Android source | iOS |
|---|---|---|
| Device tab: home, connect, telemetry, ground station links, flight log, device, engineering, video engine, detection, recordings, setup, status chip | `Device*.kt` | `Device*.swift`, Settings → Device |
| Device tab: connection and Wi-Fi wizard | `DeviceNetwork*.kt`, `DeviceWifiWizard.kt` | same names |
| Device tab: cameras, camera setup and wizards, kit camera | `DeviceCamera*.kt`, `CameraWizard.kt`, `DeviceKitCamera.kt` | same names |
| Press and hold the video to look at or fly to a spot | `PictureHold.kt` | `PictureHold.swift` |
| Heading tape | `HeadingTape.kt` | `HeadingTape.swift` |
| Route recording | `RouteRecording.kt` | `RouteRecording.swift` |
| Lens measurement | `Lens.kt` | `Lens.swift` |
| Gimbal video mode (drag / point / hold) | `GimbalIndicator.kt`, `RcCameraControls.kt` | same names |
| Synthetic view steers a gimbal | `SyntheticView.kt` | same name |
| Go to preview: distance, verdict, return-home offer, map preview | `MapClickMenu.kt`, map `VehicleMap.kt` | `MapClickMenu.swift`, `Map/GotoMarker.swift` |
| Payload status line | `GripperPanel.kt` | same name |
| Telemetry placeholders in the pilot's units | `FlyPortrait.kt` | same name |

## Not ported, and why

| Android | Why iOS does not have it |
|---|---|
| `AppUpdate.kt`, `AppInstall.kt` (download and install a new APK) | iOS apps update only through the App Store or TestFlight. |
| `FlasherScreen.kt` (write an image to an SD card) | iOS gives apps no raw disk access. |
| `PhoneNetwork.kt` (phone Wi-Fi SSID) | Only the flasher uses it. |
| `firmware.flash` / `view.firmwarePorts` (USB firmware upload) | iOS has no USB serial access for apps. |
| `PacketRadioSection.kt` (wfb-ng over a USB Wi-Fi adapter) | iOS cannot drive a USB Wi-Fi adapter. |
| `HostDevice.kt`, `joystick.builtInSticks` (RadioMaster built-in radio) | Android-only handheld hardware. |
| `CloseGuard.kt` (warn before closing the app) | iOS apps are not closed by the user from inside the app. |
| `VehicleLinkService.kt` (foreground service holding the link and Wi-Fi lock) | Keeping the link in the background needs iOS background modes. That is a separate design, not a port. |

## macOS

The Mac app has everything above, plus Android features a desktop can do that a phone or tablet running
iOS cannot:

| Android | macOS |
|---|---|
| USB firmware flashing (`view.firmwarePorts`, `firmware.flash`) | `FirmwareFlashMac.swift`: the core reaches serial ports natively on macOS |
| `PacketRadioSection.kt` (wfb-ng over a USB Wi-Fi adapter) | `PacketRadioSection.swift`; `src/Bridge/QGCWfbC.cc` and libs/wfb are linked into the app |
| `AppUpdate.kt`, `AppInstall.kt` | `AppUpdate.swift`, `AppInstall.swift`: the core verifies the `.dmg`, the app opens it and quits |

Still missing on macOS:

| Android | Why |
|---|---|
| `FlasherScreen.kt` (write an SD card) | The screen is ported (`FlasherScreen.swift`) but not in Settings: the core's `platformdisk` has no macOS backend, and raw disk writes need administrator rights. |
| `PhoneNetwork.kt` | Only the flasher uses it; a Mac reads its SSID only with Location Services granted. |
| `HostDevice.kt`, `joystick.builtInSticks` | Android-only handheld hardware. |
| `CloseGuard.kt` | Not ported yet. A Mac app can ask before quitting, but the iOS-shaped app does not do it. |
| `VehicleLinkService.kt` | Not needed: App Nap is held off while a vehicle is connected. |

macOS differences inside ported features: MapLibre's logo and attribution sit in the map's lower-left
corner (its AppKit SDK has no placement API); orientation, keyboard types and the status bar do nothing;
the Mac build is Apple Silicon only. The Mac links Homebrew's libusb and libsodium; a distributed `.dmg`
must link them statically or embed them.

## Known differences inside ported features

- The Go to map preview does not hide a target the vehicle has already reached: the iOS `GotoLocation`
  carries no `arrived` state yet.
- The phone's own camera as a video source does not play: the core builds it on `ahcsrc`, which only
  Android's GStreamer has. `DeviceCameraHost` (camera permission and rotation) is ported, but it has no effect
  until the core has an iOS camera source.
- A double tap on the picture-hold reticle's Stop or on the gimbal mode switch can also count as a double tap
  on the video. Android keeps video gestures off on-screen controls with `videoControl()`, which iOS lacks.
