import Foundation

let GEOTAG_ROOT = "geoTag"

struct GeoTagState: Equatable {
    let logFile: String
    let imageDirectory: String
    let saveDirectory: String
    let errorMessage: String
    let progress: Double
    let inProgress: Bool
    let tagged: Int
    let skipped: Int
    let failed: Int
    let timeOffsetSecs: Double
    let previewMode: Bool
    var images: [GeoTagImage] = []
}

struct GeoTagCoordinate: Equatable {
    let latitude: Double
    let longitude: Double
}

struct GeoTagImage: Equatable {
    let fileName: String
    let status: Int
    let statusString: String
    let errorMessage: String
    let coordinate: GeoTagCoordinate?
}

let GEOTAG_TAGGED = 2

func geoTagImages(_ json: JSON) -> [GeoTagImage] {
    json["imageModel"].objects.map { row in
        GeoTagImage(
            fileName: row["fileName"].string,
            status: row["status"].int(0),
            statusString: row["statusString"].string,
            errorMessage: row["errorMessage"].string,
            coordinate: row["coordinate"].object != nil
                ? GeoTagCoordinate(latitude: row["coordinate"]["latitude"].double(.nan), longitude: row["coordinate"]["longitude"].double(.nan))
                : nil
        )
    }
}

func geoTagImageText(_ image: GeoTagImage) -> String { image.errorMessage.ifBlank(image.statusString) }

func geoTagCoordinate(_ image: GeoTagImage) -> String? {
    guard image.status == GEOTAG_TAGGED, let at = image.coordinate else { return nil }
    return String(format: "%.6f, %.6f", at.latitude, at.longitude)
}

func geoTagError(_ state: GeoTagState) -> String? {
    !state.errorMessage.isBlank && !state.inProgress ? state.errorMessage : nil
}

func geoTagState(_ json: JSON?) -> GeoTagState? {
    guard let it = json, it["class"].string == "GeoTagController" else { return nil }
    return GeoTagState(
        logFile: it["logFile"].string,
        imageDirectory: it["imageDirectory"].string,
        saveDirectory: it["saveDirectory"].string,
        errorMessage: it["errorMessage"].string,
        progress: it["progress"].double(0),
        inProgress: it["inProgress"].bool,
        tagged: it["taggedCount"].int(0),
        skipped: it["skippedCount"].int(0),
        failed: it["failedCount"].int(0),
        timeOffsetSecs: it["timeOffsetSecs"].double(0),
        previewMode: it["previewMode"].bool,
        images: geoTagImages(it)
    )
}

func geoTagButton(_ state: GeoTagState) -> String {
    state.inProgress ? "Cancel" : state.previewMode ? "Preview" : "Start tagging"
}

func geoTagSummary(_ state: GeoTagState) -> String? {
    guard !state.inProgress, state.tagged > 0 else { return nil }
    let details = [
        state.skipped > 0 ? "\(state.skipped) skipped" : nil,
        state.failed > 0 ? "\(state.failed) failed" : nil,
    ].compactMap { $0 }
    return "Successfully tagged \(state.tagged) images" + (details.isEmpty ? "" : " (\(details.joined(separator: ", ")))")
}

func geoTagStep(_ done: Bool, _ number: Int) -> String { done ? "✓" : "\(number)" }

let GEOTAG_IMAGE_EXTENSIONS = "jpg,jpeg,tiff,tif,dng"

func isGeoTagImage(_ name: String) -> Bool {
    let ext = name.contains(".") ? String(name.split(separator: ".", omittingEmptySubsequences: false).last ?? "") : ""
    return GEOTAG_IMAGE_EXTENSIONS.split(separator: ",").map(String.init).contains(ext.lowercased())
}
