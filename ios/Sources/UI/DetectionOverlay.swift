import SwiftUI

private let BOX_COLOUR = Color(hex: 0x4DD0E1)
private let TARGET_COLOUR = Color(hex: 0xFFD54F)

struct DetectionOverlay: View {
    @Environment(\.theme) private var theme
    @QgcPath(DETECTIONS) private var json
    @QgcPath(VIDEO_VIEW) private var videoJson

    var body: some View {
        let reading = detections(json)
        let boxes = visibleBoxes(reading)
        let trouble = detectionTrouble(reading)
        let source = videoReading(videoJson)?.sourceSize
        GeometryReader { geometry in
            let picture = paintedRect(Double(geometry.size.width), Double(geometry.size.height), source)
            ZStack(alignment: .topLeading) {
                Canvas { context, _ in
                    boxes.forEach { box in
                        let rect = CGRect(x: picture.left + box.x * picture.width, y: picture.top + box.y * picture.height, width: box.w * picture.width, height: box.h * picture.height)
                        context.stroke(Path(rect), with: .color(box.target ? TARGET_COLOUR : BOX_COLOUR), lineWidth: box.target ? 2 : 1)
                    }
                }
                ForEach(Array(boxes.enumerated()), id: \.offset) { _, box in
                    let caption = boxCaption(box)
                    if !caption.isBlank {
                        Text(caption)
                            .font(.labelSmall)
                            .foregroundStyle(.black)
                            .padding(.horizontal, 3)
                            .background(box.target ? TARGET_COLOUR : BOX_COLOUR)
                            .offset(x: picture.left + box.x * picture.width, y: picture.top + box.y * picture.height)
                    }
                }
                if let trouble {
                    Text("Detections: \(trouble)")
                        .font(.labelSmall)
                        .foregroundStyle(theme.colors.onErrorContainer)
                        .padding(.horizontal, 4)
                        .padding(.vertical, 2)
                        .background(theme.colors.errorContainer)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
                }
            }
        }
        .allowsHitTesting(false)
    }
}
