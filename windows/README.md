# Aircast QGC — Windows head

WinUI 3 (C#, .NET 9) UI over the Rust `groundstation` core, ARM-native (`win-arm64` + `aarch64-pc-windows-msvc`).
Feature parity target: the Android head. See `NATIVE_WINDOWS_REWRITE.md`.

## Build (on the Windows ARM64 box)

```powershell
# core DLL only
.\build-core.ps1

# core + head (stages to a local dir, dodging the VirtioFS share)
.\build-app.ps1
```

Requirements (all present on the `winvm` build box): Rust `1.95.0-aarch64-pc-windows-msvc`,
VS 2022 with the VC ARM64 toolset, LLVM/clang (a core C dep needs it), .NET SDK 9.
The Windows App SDK is pulled from NuGet on first restore.

## Layout

```text
windows/
  build-core.ps1          groundstation -> groundstation.dll (aarch64)
  build-app.ps1           core + dotnet build, staged locally
  src/AircastQGC/
    Core/                 NativeMethods, CoreHost, Bridge, BridgeWatch  (the core binding)
    App.xaml(.cs)         boots the core, opens MainWindow
    MainWindow.xaml(.cs)  shell: title bar, nav pane, Fly view composed from Controls/
    Controls/             one UserControl per Fly surface, each watching its own view.* paths
```

## Notes

- Build off a local `C:` path, not the `Z:\` VirtioFS share — deep reads flake with OS error 53.
  `build-core.ps1` keeps `CARGO_TARGET_DIR` local and copies the ArduPilot param set locally
  (`QGC_APM_PARAMS_DIR`); `build-app.ps1` stages the project to `C:\aircast-head`.
- Do not set `AppWindow.TitleBar.PreferredTheme`: on WindowsAppSDK 1.8 it blanks the whole window to black
  (UI tree alive, nothing presented). The dark app theme already themes the caption buttons.
- WinUI clamps `CornerRadius` per axis, so `RadiusPill` (999) draws an ellipse on anything not square.
  Give pills an explicit height and a radius of half of it.
- Unpackaged, self-contained WinUI 3 (no MSIX yet). MSIX/Store packaging is a later step
  (`../deploy/windows` has the older Qt plan for reference only).
