import SwiftUI
import UIKit

typealias PlatformColor = UIColor
typealias PlatformImage = UIImage
typealias PlatformFont = UIFont
typealias PlatformEdgeInsets = UIEdgeInsets
typealias PlatformView = UIView

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

extension Image {
    init(platformImage: PlatformImage) {
        self.init(uiImage: platformImage)
    }
}

extension Color {
    init(platformColor: PlatformColor) {
        self.init(uiColor: platformColor)
    }
}

extension View {
    func heavyImpactFeedback<Trigger: Equatable>(trigger: Trigger) -> some View {
        return sensoryFeedback(.impact(weight: .heavy), trigger: trigger)
    }
}

enum PlatformPasteboard {
    static func copy(_ text: String) {
        UIPasteboard.general.string = text
    }
}

@MainActor
func announceForAccessibility(_ text: String) {
    UIAccessibility.post(notification: .announcement, argument: text)
}

