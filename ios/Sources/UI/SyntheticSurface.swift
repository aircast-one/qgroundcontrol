import QGCCore
import QuartzCore
import SwiftUI

func nativeChosen(_ view: JSON?) -> Bool { view?["native"].bool == true }

func nativeDraw(_ view: JSON?) -> SyntheticDraw {
    switch view?["draw"].stringOrNil {
    case "drawn": .Drawn
    case "failed": .Failed
    default: .Loading
    }
}

final class SyntheticLayerView: UIView {
    private let metal = CAMetalLayer()
    private var attached: CGSize?

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .black
        isUserInteractionEnabled = false
        layer.addSublayer(metal)
    }

    required init?(coder: NSCoder) { nil }

    private var handle: UnsafeMutableRawPointer { Unmanaged.passUnretained(metal).toOpaque() }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil { release() } else { attach() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        attach()
    }

    private func attach() {
        guard let scale = window?.screen.nativeScale else { return }
        let pixels = CGSize(width: (bounds.width * scale).rounded(), height: (bounds.height * scale).rounded())
        guard pixels.width > 0, pixels.height > 0, pixels != attached else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        metal.frame = bounds
        metal.contentsScale = scale
        CATransaction.commit()
        attached = qgc_synthetic_attach(handle, Int32(pixels.width), Int32(pixels.height)) != 0 ? pixels : nil
    }

    func release() {
        guard attached != nil else { return }
        attached = nil
        qgc_synthetic_release(handle)
    }
}

struct SyntheticSurface: UIViewRepresentable {
    func makeUIView(context: Context) -> SyntheticLayerView { SyntheticLayerView(frame: .zero) }

    func updateUIView(_ uiView: SyntheticLayerView, context: Context) {}

    static func dismantleUIView(_ uiView: SyntheticLayerView, coordinator: ()) { uiView.release() }
}
