import AppKit
import QGCVideoC
import SwiftUI

final class VideoLayerView: NSView {
    private var buffer = [UInt8]()
    private var timer: Timer?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        layer?.contentsGravity = .resizeAspect
    }

    required init?(coder: NSCoder) {
        nil
    }

    func start() {
        guard timer == nil else { return }
        let timer = Timer(timeInterval: 1.0 / 30.0, repeats: true) { [weak self] _ in self?.draw() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    private func draw() {
        let width = Int(qgc_video_width())
        let height = Int(qgc_video_height())
        guard width > 0, height > 0 else { return }

        let needed = width * height * 4
        if buffer.count < needed {
            buffer = [UInt8](repeating: 0, count: needed)
        }

        var copiedWidth: Int32 = 0
        var copiedHeight: Int32 = 0
        var stride: Int32 = 0
        let copied = buffer.withUnsafeMutableBytes { destination -> Bool in
            guard let base = destination.baseAddress else { return false }
            return qgc_video_copy_frame(base, Int32(destination.count),
                                        &copiedWidth, &copiedHeight, &stride)
        }
        guard copied, copiedWidth > 0, copiedHeight > 0, stride > 0 else { return }

        layer?.contents = VideoLayerView.image(from: buffer,
                                               width: Int(copiedWidth),
                                               height: Int(copiedHeight),
                                               stride: Int(stride))
    }

    static func image(from bytes: [UInt8], width: Int, height: Int, stride: Int) -> CGImage? {
        let needed = stride * height
        guard needed > 0, bytes.count >= needed else { return nil }
        guard let provider = CGDataProvider(data: Data(bytes[0..<needed]) as CFData) else { return nil }
        return CGImage(width: width,
                       height: height,
                       bitsPerComponent: 8,
                       bitsPerPixel: 32,
                       bytesPerRow: stride,
                       space: CGColorSpaceCreateDeviceRGB(),
                       bitmapInfo: CGBitmapInfo(rawValue:
                           CGImageAlphaInfo.premultipliedFirst.rawValue
                               | CGBitmapInfo.byteOrder32Little.rawValue),
                       provider: provider,
                       decode: nil,
                       shouldInterpolate: true,
                       intent: .defaultIntent)
    }
}

struct NativeVideoView: NSViewRepresentable {
    func makeNSView(context: Context) -> VideoLayerView {
        let view = VideoLayerView(frame: .zero)
        view.start()
        return view
    }

    func updateNSView(_ nsView: VideoLayerView, context: Context) {}

    static func dismantleNSView(_ nsView: VideoLayerView, coordinator: ()) {
        nsView.stop()
    }
}
