import Foundation
import SwiftUI

struct DeviceScreenProps {
    let href: String
    let query: [String: String]
    let onOpen: (String) -> Void
    let onOpenFrom: (_ href: String, _ returnTo: String) -> Void
    let onClose: () -> Void
}

func openedFrom(_ stack: [String], _ href: String, _ returnTo: String) -> [String] { Array(stack.dropLast()) + [returnTo, href] }

struct DeviceScreenEntry {
    let title: String
    let screen: @MainActor (DeviceScreenProps) -> AnyView
}

let DEVICE_SCREENS: [String: DeviceScreenEntry] = [
    "/telemetry": DeviceScreenEntry(title: "Telemetry") { AnyView(DeviceTelemetryScreen(props: $0)) },
    "/connect": DeviceScreenEntry(title: "Ground station links") { AnyView(DeviceConnectScreen(props: $0)) },
    "/flights": DeviceScreenEntry(title: "Flight log") { AnyView(DeviceFlightsScreen(props: $0)) },
    "/settings/device": DeviceScreenEntry(title: "Device") { AnyView(DeviceSettingsScreen(props: $0)) },
    "/settings/engineering": DeviceScreenEntry(title: "Engineering") { AnyView(DeviceEngineeringScreen(props: $0)) },
    "/settings/video": DeviceScreenEntry(title: "Video engine") { AnyView(DeviceVideoScreen(props: $0)) },
    "/settings/detection": DeviceScreenEntry(title: "Detection") { AnyView(DeviceDetectionScreen(props: $0)) },
    "/settings/recordings": DeviceScreenEntry(title: "Recordings") { _ in AnyView(DeviceRecordingsScreen()) },
    "/network": DeviceScreenEntry(title: "Connection") { AnyView(DeviceNetworkScreen(props: $0)) },
    "/streaming": DeviceScreenEntry(title: "Cameras") { AnyView(DeviceCamerasScreen(props: $0)) },
    "/settings/camera": DeviceScreenEntry(title: "Watching & recording") { AnyView(DeviceCameraSetupScreen(props: $0)) },
    "/setup": DeviceScreenEntry(title: "Device setup") { AnyView(DeviceSetupScreen(props: $0)) },
]

struct DevicePath: Equatable {
    let path: String
    let query: [String: String]
}

private func decoded(_ text: String) -> String {
    text.replacingOccurrences(of: "+", with: " ").removingPercentEncoding ?? text
}

func devicePath(_ href: String) -> DevicePath {
    guard let components = URLComponents(string: href) else {
        return DevicePath(path: href.components(separatedBy: "?").first ?? href, query: [:])
    }
    let pairs: [(String, String)] = (components.percentEncodedQuery ?? "").split(separator: "&").map { pair in
        let parts = pair.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
        return (decoded(String(parts[0])), parts.count > 1 ? decoded(String(parts[1])) : "")
    }
    return DevicePath(path: components.path.isEmpty ? "/" : components.path, query: Dictionary(pairs, uniquingKeysWith: { _, last in last }))
}
