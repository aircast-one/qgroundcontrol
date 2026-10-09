import Foundation

let GCS_POSITION_VIEW = "view.gcsPosition"

enum OperatorBridge {
    static func read() -> JSON? {
        let view = Qgc.get(GCS_POSITION_VIEW)
        return view.objectOrNil
    }
}

func operatorHeading(_ view: JSON?) -> Double {
    guard let view, view["usable"].bool, !view["heading"].isNull else { return .nan }
    return view["heading"].double ?? .nan
}

func operatorPoint(_ view: JSON?) -> TrackPoint? {
    guard let view, view["usable"].bool else { return nil }
    return coordinate(view)
}
