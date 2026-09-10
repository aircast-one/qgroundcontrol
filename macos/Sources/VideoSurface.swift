import AppKit
import QGCVideoC
import SwiftUI

final class VideoLayerView: NSView {
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

    // One copy per frame, not two. This used to copy the frame into a Swift array and then copy
    // the whole array again to hand CoreGraphics something it could keep. The second copy was
    // load-bearing - CoreAnimation reads layer.contents on its own schedule and must not alias a
    // buffer overwritten thirty times a second - so the fix is to make the copy the C ABI itself
    // performs land in the storage that gets kept, rather than to remove a copy that guards a
    // real race. Data(count:) zero-fills, so this trades a per-frame memcpy for a per-frame
    // memset rather than removing the work outright - the saving is real but it is one pass, not
    // two, and the old persistent buffer was allocated once.
    private func draw() {
        let capacity = VideoFrame.capacity(width: Int(qgc_video_width()),
                                           height: Int(qgc_video_height()))
        guard capacity > 0 else { return }

        var copiedWidth: Int32 = 0
        var copiedHeight: Int32 = 0
        var stride: Int32 = 0
        var bytes = Data(count: capacity)
        let copied = bytes.withUnsafeMutableBytes { destination -> Bool in
            guard let base = destination.baseAddress else { return false }
            return qgc_video_copy_frame(base, Int32(destination.count),
                                        &copiedWidth, &copiedHeight, &stride)
        }
        guard copied,
              let frame = VideoFrame(width: copiedWidth, height: copiedHeight, stride: stride),
              frame.fits(capacity) else { return }

        layer?.contents = VideoLayerView.image(from: bytes, frame: frame)
    }

    static func image(from bytes: Data, frame: VideoFrame) -> CGImage? {
        guard frame.fits(bytes.count) else { return nil }
        guard let provider = CGDataProvider(data: bytes as CFData) else { return nil }
        return CGImage(width: frame.width,
                       height: frame.height,
                       bitsPerComponent: 8,
                       bitsPerPixel: 32,
                       bytesPerRow: frame.stride,
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
