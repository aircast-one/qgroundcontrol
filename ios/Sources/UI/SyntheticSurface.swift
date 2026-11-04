import QGCCore
import QuartzCore
import SwiftUI
#if os(macOS)
import AppKit
#endif

func nativeChosen(_ view: JSON?) -> Bool { view?["native"].bool == true }

func nativeDraw(_ view: JSON?) -> SyntheticDraw {
    switch view?["draw"].stringOrNil {
    case "drawn": .Drawn
    case "failed": .Failed
    default: .Loading
    }
}

// Hosts the core's Metal-drawn 3D view in a CAMetalLayer the size of this view, in device pixels.
final class SyntheticLayerView: PlatformView {
    private let metal = CAMetalLayer()
    private let orbit: Bool
    private var attached: CGSize?

    init(orbit: Bool) {
        self.orbit = orbit
        super.init(frame: .zero)
        #if os(iOS)
        backgroundColor = .black
        isUserInteractionEnabled = false
        layer.addSublayer(metal)
        #elseif os(macOS)
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        layer?.addSublayer(metal)
        #endif
    }

    required init?(coder: NSCoder) { nil }

    private var handle: UnsafeMutableRawPointer { Unmanaged.passUnretained(metal).toOpaque() }

    #if os(iOS)
    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil { release() } else { attach() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        attach()
    }

    private var pixelScale: CGFloat? { window?.screen.nativeScale }
    #elseif os(macOS)
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil { release() } else { attach() }
    }

    override func layout() {
        super.layout()
        attach()
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        attach()
    }

    // Moving to a screen with another pixel density changes the drawable size.
    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        attach()
    }

    // The aim and look gestures belong to the SwiftUI views around the surface, as with isUserInteractionEnabled off.
    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    private var pixelScale: CGFloat? { window?.backingScaleFactor }
    #endif

    private func attach() {
        guard let scale = pixelScale else { return }
        let pixels = CGSize(width: (bounds.width * scale).rounded(), height: (bounds.height * scale).rounded())
        guard pixels.width > 0, pixels.height > 0, pixels != attached else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        metal.frame = bounds
        metal.contentsScale = scale
        CATransaction.commit()
        attached = qgc_synthetic_attach(handle, Int32(pixels.width), Int32(pixels.height), orbit ? 1 : 0) != 0 ? pixels : nil
    }

    func release() {
        guard attached != nil else { return }
        attached = nil
        qgc_synthetic_release(handle)
    }
}

struct SyntheticSurface: PlatformViewRepresentable {
    typealias Coordinator = Void

    var orbit = false

    func makePlatformView(context: Context) -> SyntheticLayerView { SyntheticLayerView(orbit: orbit) }

    static func dismantlePlatformView(_ view: SyntheticLayerView, coordinator: ()) { view.release() }
}
