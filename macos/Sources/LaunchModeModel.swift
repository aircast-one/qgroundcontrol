import Foundation

enum LaunchMode {
    static let flag = "--native-window"
    static let infoKey = "QGCNativeUI"

    static func drawsNativeWindows(arguments: [String], bundleDeclaresNativeUI: Bool) -> Bool {
        arguments.contains(LaunchMode.flag) || bundleDeclaresNativeUI
    }

    static func bundleDeclaresNativeUI(_ info: [String: Any]?) -> Bool {
        (info?[LaunchMode.infoKey] as? NSNumber)?.boolValue ?? false
    }
}
