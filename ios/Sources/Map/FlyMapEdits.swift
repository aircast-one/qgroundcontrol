import SwiftUI

@Observable
final class FlyMapEdits {
    var orbit: OrbitCircle?
    var gotoLoiter: LoiterEdit?
    var gotoPreview: TrackPoint?
    var gotoPreviewFrom: TrackPoint?
}
