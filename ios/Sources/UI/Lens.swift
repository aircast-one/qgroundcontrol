import SwiftUI

let LENS_VIEW = "view.lens"
let LENS_MEASURE = "camera.measureLens"
let LENS_FORGET = "camera.forgetLens"
let APPROXIMATE_AIM = "Approximate aim · measure the lens in camera details"

struct LensReading: Equatable {
    let summary: String
    let approximate: Bool
    let measuring: Bool
    let progress: String
    let error: String?
    let canMeasure: Bool
    let reason: String?
    let measuredZooms: [Double]
}

func lensReading(_ view: JSON?) -> LensReading? {
    guard let view, view["class"].string == "Lens" else { return nil }
    return LensReading(
        summary: view["summary"].string,
        approximate: view["approximate"].bool,
        measuring: view["measuring"].bool,
        progress: view["progress"].string,
        error: view["error"].string.nonBlank,
        canMeasure: view["canMeasure"].bool,
        reason: view["reason"].string.nonBlank,
        measuredZooms: view["samples"].array.compactMap { $0["zoom"].double }
    )
}

func measuredText(_ zooms: [Double]) -> String? {
    guard !zooms.isEmpty else { return nil }
    return "Measured at " + zooms.sorted().map { "\(saturatingInt($0))%" }.joined(separator: ", ") + " zoom"
}

struct LensSection: View {
    @QgcPath(LENS_VIEW) private var json
    @Environment(\.theme) private var theme

    var body: some View {
        if let lens = lensReading(json) {
            VStack(alignment: .leading, spacing: 0) {
                SectionHeader(text: "Lens")
                Text(lens.summary).font(.bodyMedium).padding(.horizontal, Space.s4).padding(.vertical, Space.s1)
                if let measured = measuredText(lens.measuredZooms) {
                    Text(measured).font(.bodySmall).padding(.horizontal, Space.s4)
                }
                if lens.measuring {
                    Text(lens.progress).font(.bodySmall).padding(.horizontal, Space.s4).padding(.vertical, Space.s1)
                }
                if let error = lens.error {
                    Text(error).font(.bodySmall).foregroundStyle(theme.colors.error).padding(.horizontal, Space.s4).padding(.vertical, Space.s1)
                }
                Button(lens.measuring ? "Measuring…" : "Measure the lens") { offMain { Qgc.invoke(LENS_MEASURE) } }
                    .buttonStyle(.text)
                    .disabled(!lens.canMeasure)
                    .padding(.horizontal, Space.s1)
                if !lens.measuring, let reason = lens.reason {
                    Text(reason).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).padding(.horizontal, Space.s4)
                }
                if !lens.measuredZooms.isEmpty && !lens.measuring {
                    Button("Forget measurements") { offMain { Qgc.invoke(LENS_FORGET) } }
                        .buttonStyle(.text)
                        .padding(.horizontal, Space.s1)
                }
            }
        }
    }
}
