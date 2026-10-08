import SwiftUI
import UIKit

let OPTICAL_FLOW_SCREEN = "opticalFlow"
private let OPTICAL_FLOW_VIEW = "view.opticalFlow"
private let OPTICAL_FLOW_POLL_MS = 500

struct FlowFrame: Equatable {
    var width: Int
    var height: Int
    var rgba: [UInt8]
}

func flowFrame(_ view: JSON?) -> FlowFrame? {
    guard let image = view?["image"], image.object != nil else { return nil }
    return FlowFrame(
        width: image["width"].int(0),
        height: image["height"].int(0),
        rgba: Data(base64Encoded: image["data"].string, options: .ignoreUnknownCharacters).map { [UInt8]($0) } ?? []
    )
}

private func channel(_ bytes: [UInt8], _ at: Int) -> UInt32 {
    bytes.indices.contains(at) ? UInt32(bytes[at]) : 0
}

func argbPixels(_ rgba: [UInt8], _ count: Int) -> [UInt32] {
    (0..<count).map { pixel in
        let at = pixel * 4
        return (channel(rgba, at + 3) << 24) | (channel(rgba, at) << 16) | (channel(rgba, at + 1) << 8) | channel(rgba, at + 2)
    }
}

private func bitmapOf(_ frame: FlowFrame) -> UIImage? {
    guard frame.width > 0, frame.height > 0 else { return nil }
    let pixels = argbPixels(frame.rgba, frame.width * frame.height)
    let data = pixels.withUnsafeBufferPointer { Data(buffer: $0) }
    guard let provider = CGDataProvider(data: data as CFData),
          let image = CGImage(
              width: frame.width,
              height: frame.height,
              bitsPerComponent: 8,
              bitsPerPixel: 32,
              bytesPerRow: frame.width * 4,
              space: CGColorSpaceCreateDeviceRGB(),
              bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.first.rawValue | CGBitmapInfo.byteOrder32Little.rawValue),
              provider: provider,
              decode: nil,
              shouldInterpolate: false,
              intent: .defaultIntent
          )
    else { return nil }
    return UIImage(cgImage: image)
}

struct OpticalFlowScreen: View {
    @State private var index = -1
    @State private var image: UIImage?

    var body: some View {
        VStack(spacing: Space.s3) {
            Text("Optical flow camera").font(.titleMedium)
            if let image {
                GeometryReader { geometry in
                    Image(uiImage: image)
                        .resizable()
                        .scaledToFit()
                        .frame(width: geometry.size.width / 2, height: geometry.size.width / 2 * 3 / 4)
                        .frame(maxWidth: .infinity)
                        .accessibilityLabel("Optical flow camera image")
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .padding(Space.s4)
        .task {
            while !Task.isCancelled {
                let view = await offMain { Qgc.get(OPTICAL_FLOW_VIEW) }
                let latest = view["index"].int(0)
                if latest != index {
                    index = latest
                    image = await offMain { flowFrame(view).flatMap(bitmapOf) }
                }
                try? await Task.sleep(for: .milliseconds(OPTICAL_FLOW_POLL_MS))
            }
        }
    }
}
