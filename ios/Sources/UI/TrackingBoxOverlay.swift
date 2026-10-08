import SwiftUI

private let TRACK_COLOUR = Color(hex: 0x00E5FF)

struct TrackingBoxOverlay: View {
    @QgcPath(CAMERA_VIEW) private var cameraJson
    @QgcPath(VIDEO_VIEW) private var videoJson

    var body: some View {
        if let tracking = trackingReading(cameraJson) {
            let source = videoReading(videoJson)?.sourceSize
            GeometryReader { geometry in
                let picture = paintedRect(Double(geometry.size.width), Double(geometry.size.height), source)
                ZStack(alignment: .topLeading) {
                    if trackingCanAim(tracking) {
                        Color.clear
                            .contentShape(Rectangle())
                            .gesture(
                                DragGesture(minimumDistance: 0).onEnded { drag in
                                    sendTracking(trackingRequest(drag.startLocation.x, drag.startLocation.y, drag.location.x, drag.location.y, picture))
                                }
                            )
                    }
                    if let box = tracking.box {
                        Path(CGRect(x: picture.left + box.x * picture.width, y: picture.top + box.y * picture.height, width: box.width * picture.width, height: box.height * picture.height))
                            .stroke(TRACK_COLOUR, lineWidth: 2)
                            .allowsHitTesting(false)
                    }
                }
            }
        }
    }
}

private func sendTracking(_ request: TrackingRequest?) {
    switch request {
    case nil:
        break
    case .Box(let rect):
        offMain { Qgc.invoke(CAMERA_START_TRACKING, trackingRectObject(rect)) }
    case .Point(let x, let y, let radius):
        offMain { Qgc.invoke(CAMERA_START_TRACKING, trackingPointObject(x, y), radius) }
    }
}
