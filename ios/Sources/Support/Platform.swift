import SwiftUI
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

// One name per native type, so code shared by the iOS app and the native macOS app reads the same on both.
#if os(iOS)
typealias PlatformColor = UIColor
typealias PlatformImage = UIImage
typealias PlatformFont = UIFont
typealias PlatformEdgeInsets = UIEdgeInsets
typealias PlatformView = UIView
#elseif os(macOS)
typealias PlatformColor = NSColor
typealias PlatformImage = NSImage
typealias PlatformFont = NSFont
typealias PlatformEdgeInsets = NSEdgeInsets
typealias PlatformView = NSView
#endif

// A SwiftUI wrapper of a native view that compiles on both platforms. Conform to it instead of
// UIViewRepresentable / NSViewRepresentable and write the platform-neutral requirements; the
// extension forwards UIKit's makeUIView/updateUIView/dismantleUIView (or AppKit's ...NSView ones)
// to them. The native view type (PlatformViewType, which is also UIViewType / NSViewType) is inferred from
// makePlatformView's return type, and makeCoordinator() / Coordinator / Context are the usual representable ones:
//
// A wrapper with no coordinator must say so with `typealias Coordinator = Void`: Swift cannot infer
// the representable's default Coordinator through this protocol.
//
//     struct Surface: PlatformViewRepresentable {
//         typealias Coordinator = Void
//         func makePlatformView(context: Context) -> MyView { MyView() }
//         func updatePlatformView(_ view: MyView, context: Context) {}           // optional
//         static func dismantlePlatformView(_ view: MyView, coordinator: ()) {}  // optional
//     }
#if os(iOS)
@MainActor
protocol PlatformViewRepresentable: UIViewRepresentable where UIViewType == PlatformViewType {
    associatedtype PlatformViewType: UIView
    func makePlatformView(context: UIViewRepresentableContext<Self>) -> PlatformViewType
    func updatePlatformView(_ view: PlatformViewType, context: UIViewRepresentableContext<Self>)
    static func dismantlePlatformView(_ view: PlatformViewType, coordinator: Coordinator)
}

extension PlatformViewRepresentable {
    func makeUIView(context: UIViewRepresentableContext<Self>) -> PlatformViewType { makePlatformView(context: context) }

    func updateUIView(_ uiView: PlatformViewType, context: UIViewRepresentableContext<Self>) { updatePlatformView(uiView, context: context) }

    static func dismantleUIView(_ uiView: PlatformViewType, coordinator: Coordinator) { dismantlePlatformView(uiView, coordinator: coordinator) }

    func updatePlatformView(_ view: PlatformViewType, context: UIViewRepresentableContext<Self>) {}

    static func dismantlePlatformView(_ view: PlatformViewType, coordinator: Coordinator) {}
}
#elseif os(macOS)
@MainActor
protocol PlatformViewRepresentable: NSViewRepresentable where NSViewType == PlatformViewType {
    associatedtype PlatformViewType: NSView
    func makePlatformView(context: NSViewRepresentableContext<Self>) -> PlatformViewType
    func updatePlatformView(_ view: PlatformViewType, context: NSViewRepresentableContext<Self>)
    static func dismantlePlatformView(_ view: PlatformViewType, coordinator: Coordinator)
}

extension PlatformViewRepresentable {
    func makeNSView(context: NSViewRepresentableContext<Self>) -> PlatformViewType { makePlatformView(context: context) }

    func updateNSView(_ nsView: PlatformViewType, context: NSViewRepresentableContext<Self>) { updatePlatformView(nsView, context: context) }

    static func dismantleNSView(_ nsView: PlatformViewType, coordinator: Coordinator) { dismantlePlatformView(nsView, coordinator: coordinator) }

    func updatePlatformView(_ view: PlatformViewType, context: NSViewRepresentableContext<Self>) {}

    static func dismantlePlatformView(_ view: PlatformViewType, coordinator: Coordinator) {}
}
#endif

extension Image {
    init(platformImage: PlatformImage) {
        #if os(iOS)
        self.init(uiImage: platformImage)
        #elseif os(macOS)
        self.init(nsImage: platformImage)
        #endif
    }
}

extension Color {
    init(platformColor: PlatformColor) {
        #if os(iOS)
        self.init(uiColor: platformColor)
        #elseif os(macOS)
        self.init(nsColor: platformColor)
        #endif
    }
}

#if os(macOS)
// UIImage's spellings, so PlatformImage(cgImage:) and image.cgImage read the same on both platforms.
extension NSImage {
    convenience init(cgImage: CGImage) {
        self.init(cgImage: cgImage, size: .zero)
    }

    var cgImage: CGImage? { self.cgImage(forProposedRect: nil, context: nil, hints: nil) }
}

extension NSEdgeInsets {
    static var zero: NSEdgeInsets { NSEdgeInsets(top: 0, left: 0, bottom: 0, right: 0) }
}
#endif

extension View {
    // A Mac trackpad has no impact haptic; the level-change tap is the nearest it plays.
    func heavyImpactFeedback<Trigger: Equatable>(trigger: Trigger) -> some View {
        #if os(iOS)
        return sensoryFeedback(.impact(weight: .heavy), trigger: trigger)
        #elseif os(macOS)
        return sensoryFeedback(.levelChange, trigger: trigger)
        #endif
    }
}

enum PlatformPasteboard {
    static func copy(_ text: String) {
        #if os(iOS)
        UIPasteboard.general.string = text
        #elseif os(macOS)
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        #endif
    }
}

@MainActor
func announceForAccessibility(_ text: String) {
    #if os(iOS)
    UIAccessibility.post(notification: .announcement, argument: text)
    #elseif os(macOS)
    let element: Any = NSApplication.shared.mainWindow.map { $0 as Any } ?? NSApplication.shared
    NSAccessibility.post(element: element, notification: .announcementRequested,
                         userInfo: [.announcement: text, .priority: NSAccessibilityPriorityLevel.high.rawValue])
    #endif
}

// Keeps the Mac awake and out of App Nap while something the pilot depends on runs (a connected vehicle, a
// recording) — what idle-timer and background-task holds do on iOS. Each holder names itself; an end without a
// begin is ignored.
#if os(macOS)
@MainActor
enum MacActivity {
    private static var held: [String: NSObjectProtocol] = [:]

    static func begin(_ reason: String, keepDisplayOn: Bool) {
        guard held[reason] == nil else { return }
        let options: ProcessInfo.ActivityOptions = keepDisplayOn ? [.userInitiated, .idleDisplaySleepDisabled] : [.userInitiated]
        held[reason] = ProcessInfo.processInfo.beginActivity(options: options, reason: reason)
    }

    static func end(_ reason: String) {
        guard let activity = held.removeValue(forKey: reason) else { return }
        ProcessInfo.processInfo.endActivity(activity)
    }
}
#endif
