import Foundation

// The geometry the C ABI reports for one decoded frame. Separated from the layer so the bounds
// arithmetic is reachable by the checks: a stride or a height the head trusts without checking
// is a read past the end of a buffer, in the one path that runs thirty times a second.
struct VideoFrame: Equatable {
    let width: Int
    let height: Int
    let stride: Int

    init?(width: Int32, height: Int32, stride: Int32) {
        guard width > 0, height > 0, stride > 0 else { return nil }
        // A stride narrower than the pixels it claims to carry cannot be a row of them.
        guard Int(stride) >= Int(width) * VideoFrame.bytesPerPixel else { return nil }
        self.width = Int(width)
        self.height = Int(height)
        self.stride = Int(stride)
    }

    static let bytesPerPixel = 4

    var byteCount: Int { stride * height }

    // A decoder pads each row to an alignment, so a frame is stride * height and stride can
    // exceed width * bytesPerPixel. Asking for width * height * bytesPerPixel - which is what
    // this did - leaves the buffer short for every padded width, and a short buffer is a frame
    // dropped and a black panel with nothing said. 1920 hides it because 7680 is already
    // aligned; an odd width does not.
    //
    // rowAlignment is a BOUND, not a known value: the C ABI reports the real stride only after
    // it has copied, so this asks for enough to cover any alignment up to it, and fits() still
    // refuses a frame that somehow needs more.
    static let rowAlignment = 64

    static func capacity(width: Int, height: Int) -> Int {
        guard width > 0, height > 0 else { return 0 }
        let row = width * bytesPerPixel
        let padded = (row + rowAlignment - 1) / rowAlignment * rowAlignment
        return padded * height
    }

    func fits(_ available: Int) -> Bool { available >= byteCount }
}
