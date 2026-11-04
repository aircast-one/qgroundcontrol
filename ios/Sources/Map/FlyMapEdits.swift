import SwiftUI

@Observable
final class FlyMapEdits {
    var orbit: OrbitCircle?
    var gotoLoiter: LoiterEdit?
    var gotoPreview: TrackPoint?
    var gotoPreviewFrom: TrackPoint?
    // A Mac context-menu choice the map-click sheet picks up when it opens on that point.
    var clickRequest: MapClickRequest?
}
