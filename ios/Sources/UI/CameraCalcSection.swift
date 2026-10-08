import SwiftUI

private let DISTANCE_SUFFIX = "cameraCalc.distanceToSurface"
private let DENSITY_SUFFIX = "cameraCalc.imageDensity"
private let SPACING_SUFFIXES: Set<String> = ["cameraCalc.adjustedFootprintFrontal", "cameraCalc.adjustedFootprintSide"]

struct CameraCalcBlock {
    var brand: String
    var model: String
    var brands: [String]
    var models: [String]
    var manual: Bool
    var custom: Bool
    var valueSetIsDistance: Bool
    var valueSetIsDistancePath: String
    var brandPath: String
    var modelPath: String
    var facts: [(String, Fact)]
    var distanceMode: Int? = nil
    var distanceModes: [(Int, String)] = []
    var distanceModePath: String = ""
}

func distanceModeTitle(_ block: CameraCalcBlock) -> String? {
    block.distanceModes.first { $0.0 == block.distanceMode }?.1
}

private func strings(_ block: JSON, _ key: String) -> [String] { block[key].strings }

func cameraCalc(_ view: JSON?) -> CameraCalcBlock? {
    guard let block = view?["camera"], block.object != nil else { return nil }
    let brand = block["brand"].string
    return CameraCalcBlock(
        brand: brand,
        model: block["model"].string,
        brands: strings(block, "brands"),
        models: strings(block, "models"),
        manual: brand == block["manualName"].string,
        custom: block["custom"].bool,
        valueSetIsDistance: block["valueSetIsDistance"].bool(true),
        valueSetIsDistancePath: block["valueSetIsDistancePath"].string,
        brandPath: block["brandPath"].string,
        modelPath: block["modelPath"].string,
        facts: block["facts"].array.filter { $0.object != nil }.compactMap { control in
            factFromControl(control).map { (control["pathSuffix"].string, $0) }
        },
        distanceMode: block["distanceMode"].isNull ? nil : block["distanceMode"].int(0),
        distanceModes: block["distanceModes"].array.filter { $0.object != nil }.map { ($0["raw"].int(0), $0["title"].string) },
        distanceModePath: block["distanceModePath"].string
    )
}

func distanceLabel(_ block: CameraCalcBlock) -> String {
    block.facts.first { $0.0 == DISTANCE_SUFFIX }.flatMap { $0.1.shortLabel.isBlank ? nil : $0.1.shortLabel } ?? "Altitude"
}

func shownCameraFacts(_ block: CameraCalcBlock) -> [Fact] {
    block.facts.filter { suffix, _ in
        block.manual ? (suffix == DISTANCE_SUFFIX || SPACING_SUFFIXES.contains(suffix))
            : suffix == DISTANCE_SUFFIX ? block.valueSetIsDistance
            : suffix == DENSITY_SUFFIX ? !block.valueSetIsDistance
            : true
    }.map(\.1)
}

struct CameraCalcHeader: View {
    let block: CameraCalcBlock
    let onWrite: (String, Any) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text("Camera").font(.titleSmall)
            HStack(spacing: Space.s2) {
                Choice(label: block.brand, options: block.brands) { onWrite(block.brandPath, $0) }
                if !block.manual && !block.custom {
                    Choice(label: block.model.ifEmpty("Model"), options: block.models) { onWrite(block.modelPath, $0) }
                }
            }
            if let current = distanceModeTitle(block) {
                HStack(spacing: Space.s2) {
                    Text("Altitude").font(.bodyMedium)
                    Choice(label: current, options: block.distanceModes.map(\.1)) { title in
                        if let mode = block.distanceModes.first(where: { $0.1 == title }) { onWrite(block.distanceModePath, mode.0) }
                    }
                }
            }
            if !block.manual {
                HStack(spacing: Space.s2) {
                    Text("Set by").font(.bodyMedium)
                    PlanChip(label: distanceLabel(block), selected: block.valueSetIsDistance) { onWrite(block.valueSetIsDistancePath, true) }
                    PlanChip(label: "Ground res", selected: !block.valueSetIsDistance) { onWrite(block.valueSetIsDistancePath, false) }
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s2)
    }
}

private struct Choice: View {
    let label: String
    let options: [String]
    let onPick: (String) -> Void

    var body: some View {
        Menu {
            ForEach(options, id: \.self) { option in
                Button(option) { onPick(option) }
            }
        } label: {
            Text(label)
        }
        .buttonStyle(.bordered)
        .disabled(options.isEmpty)
    }
}
