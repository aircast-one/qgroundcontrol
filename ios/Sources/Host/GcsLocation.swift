import CoreLocation
import Foundation
import QGCCore

final class GcsLocation: NSObject, CLLocationManagerDelegate {
    static let sourceToken = "internalGps"
    private static let accessError: Int64 = 0
    private static let closedError: Int64 = 1
    private static var shared: GcsLocation?

    private let manager = CLLocationManager()
    private var announced = false

    static func start() {
        guard shared == nil else { return }
        let location = GcsLocation()
        shared = location
        location.manager.delegate = location
        location.manager.desiredAccuracy = kCLLocationAccuracyBest
        location.manager.headingFilter = 1
        location.manager.requestWhenInUseAuthorization()
        location.listen()
    }

    static func stop() {
        shared?.manager.stopUpdatingLocation()
        shared?.manager.stopUpdatingHeading()
        shared = nil
    }

    private func listen() {
        switch manager.authorizationStatus {
        case .authorizedAlways, .authorizedWhenInUse:
            if !announced {
                announced = true
                qgc_core_gcs_position_source(GcsLocation.sourceToken)
            }
            manager.startUpdatingLocation()
            if CLLocationManager.headingAvailable() { manager.startUpdatingHeading() }
        case .denied, .restricted:
            qgc_core_gcs_position_error(GcsLocation.accessError)
        default:
            break
        }
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        listen()
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        locations.last.map(report)
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        let denied = (error as? CLError)?.code == .denied
        qgc_core_gcs_position_error(denied ? GcsLocation.accessError : GcsLocation.closedError)
    }

    private func report(_ location: CLLocation) {
        let valid = { (value: Double) in value >= 0 ? value : .nan }
        qgc_core_gcs_position_update(
            location.coordinate.latitude,
            location.coordinate.longitude,
            location.verticalAccuracy >= 0 ? location.altitude : .nan,
            valid(location.horizontalAccuracy),
            valid(location.verticalAccuracy),
            valid(location.course),
            valid(location.courseAccuracy)
        )
    }
}
