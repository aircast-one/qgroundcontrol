import Foundation

// The macOS DMG ships TWO apps built from ONE executable: AircastQGC.app draws Qt's UI and
// AircastQGCNative.app draws this head. They are the same binary under two names because a
// second configure-and-build costs the entire Qt build -- forty minutes on CI -- to produce
// bytes identical to the first, and because two binaries can drift while one cannot.
//
// Until now the head was reachable only by passing --native-window on a command line, so every
// shipped macOS app opened Qt's UI and this head had never been in a release at all. The Android
// session found the same thing one directory over on the same evening: their shipped APK was
// org.mavlink.qgroundcontrol, the Qt app, and their Compose head was in no release either.
enum LaunchMode {
    static let flag = "--native-window"
    static let nativeSuffix = "Native"

    // The flag stays, and stays FIRST: the dev rig launches the Debug bundle by its ordinary name
    // and would otherwise have no way in. The name is what makes a double-click work, which is the
    // only way an operator ever starts an app.
    static func drawsNativeWindows(arguments: [String], executable: String) -> Bool {
        arguments.contains(LaunchMode.flag) || executable.hasSuffix(LaunchMode.nativeSuffix)
    }
}
