import SwiftUI

let OPTICAL_FLOW_SCREEN = "opticalFlow"
private let OPTICAL_FLOW_VIEW = "view.opticalFlow"
private let OPTICAL_FLOW_POLL_MS = 500
private let FLOW_IMAGE_MAX_WIDTH: CGFloat = 360

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

private let RGBA_BYTES = 4

func rgbaPixels(_ rgba: [UInt8], _ count: Int) -> [UInt8] {
    let size = count * RGBA_BYTES
    return Array(rgba.prefix(size)) + Array(repeating: 0, count: max(size - rgba.count, 0))
}

func bitmapOf(_ frame: FlowFrame) -> PlatformImage? {
    guard frame.width > 0, frame.height > 0 else { return nil }
    let data = Data(rgbaPixels(frame.rgba, frame.width * frame.height))
    guard let provider = CGDataProvider(data: data as CFData),
          let image = CGImage(
              width: frame.width,
              height: frame.height,
              bitsPerComponent: 8,
              bitsPerPixel: 8 * RGBA_BYTES,
              bytesPerRow: frame.width * RGBA_BYTES,
              space: CGColorSpaceCreateDeviceRGB(),
              bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
              provider: provider,
              decode: nil,
              shouldInterpolate: false,
              intent: .defaultIntent
          )
    else { return nil }
    return PlatformImage(cgImage: image)
}

struct OpticalFlowScreen: View {
    @State private var index = -1
    @State private var image: PlatformImage?

    var body: some View {
        NativeForm {
            Section {
                if let image {
                    Image(platformImage: image)
                        .resizable()
                        .scaledToFit()
                        .frame(maxWidth: FLOW_IMAGE_MAX_WIDTH)
                        .frame(maxWidth: .infinity)
                        .accessibilityLabel("Optical flow camera image")
                } else {
                    Text("Waiting for an image from the camera.").foregroundStyle(Color.secondary)
                }
            } header: {
                SectionHeader(text: "Optical flow camera")
            }
        }
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
