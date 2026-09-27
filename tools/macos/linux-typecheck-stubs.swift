// Stand-ins for the few UI-side names that non-UI files reach into, so tools/macos/linux-typecheck.sh
// can typecheck every file that imports no UI framework on a Linux toolchain. Typecheck only:
// nothing here is compiled into the app or into the pure-logic checks.

struct MissionMap {
    static var lastRender: [String: [String: Any]] = [:]
    static var lastScale: [String: MapScaleBar] = [:]
}
final class VehicleSetupWindow {
    static let shared = VehicleSetupWindow()
    func show() {}
}
