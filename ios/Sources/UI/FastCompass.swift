import SwiftUI

struct FastCompass: Equatable {
    let invocation: String
    let help: String
    let vehicleHasPosition: Bool
    let gcsLatitude: Double?
    let gcsLongitude: Double?
}

struct FastCompassChoice: Equatable {
    var enabled: Bool
    var useGcs: Bool
    var latitude: String
    var longitude: String
    var useMap: Bool = false
    var mapPosition: TrackPoint? = nil
}

func fastCompass(_ json: JSON?) -> FastCompass? {
    guard let json, json.object != nil else { return nil }
    let gcs = json["gcsPosition"]
    let valid = gcs["valid"].bool
    return FastCompass(
        invocation: json["invocation"].string,
        help: json["help"].string,
        vehicleHasPosition: json["vehicleHasPosition"].bool,
        gcsLatitude: valid ? gcs["latitude"].double(.nan) : nil,
        gcsLongitude: valid ? gcs["longitude"].double(.nan) : nil
    )
}

func initialFastCompassChoice(_ fast: FastCompass, mapPosition: TrackPoint? = FlightMapPosition.latest) -> FastCompassChoice {
    FastCompassChoice(enabled: false, useGcs: fast.gcsLatitude != nil, latitude: "0.00", longitude: "0.00", mapPosition: mapPosition)
}

func fastCompassArguments(_ fast: FastCompass, _ choice: FastCompassChoice) -> [JSON] {
    if choice.useGcs, let latitude = fast.gcsLatitude, let longitude = fast.gcsLongitude {
        return [.number(latitude), .number(longitude)]
    }
    if choice.useMap, let map = choice.mapPosition {
        return [.number(map.latitude), .number(map.longitude)]
    }
    return [choice.latitude, choice.longitude].map { typed in Double(typed).map(JSON.number) ?? .string(typed) }
}

struct FastCompassBlock: View {
    let fast: FastCompass
    let choice: FastCompassChoice
    let onChange: (FastCompassChoice) -> Void

    var body: some View {
        let asksForPosition = choice.enabled && !fast.vehicleHasPosition
        let shownMap = choice.useMap ? choice.mapPosition : nil
        VStack(alignment: .leading, spacing: Space.s2) {
            Text(fast.help).font(.bodySmall)
            LabeledCheckbox(label: "Fast Calibration", checked: choice.enabled) { on in onChange(withChanges(choice) { $0.enabled = on }) }
            if asksForPosition {
                Text("Vehicle has no valid position, please provide it").font(.bodySmall)
            }
            if asksForPosition && fast.gcsLatitude != nil {
                LabeledCheckbox(label: "Use GCS position instead", checked: choice.useGcs) { on in onChange(withChanges(choice) { $0.useGcs = on }) }
            }
            if asksForPosition && fast.gcsLatitude == nil && choice.mapPosition != nil {
                LabeledCheckbox(label: "Use current map position instead", checked: choice.useMap) { on in onChange(withChanges(choice) { $0.useMap = on }) }
            }
            if let shownMap {
                Text(String(format: "Lat: %.4f Lon: %.4f", shownMap.latitude, shownMap.longitude)).font(.bodySmall)
            }
            if choice.enabled && !choice.useGcs && shownMap == nil {
                CoordinateField(label: "Latitude", value: choice.latitude) { typed in onChange(withChanges(choice) { $0.latitude = typed }) }
                CoordinateField(label: "Longitude", value: choice.longitude) { typed in onChange(withChanges(choice) { $0.longitude = typed }) }
            }
        }
    }
}

private struct LabeledCheckbox: View {
    let label: String
    let checked: Bool
    let onCheck: (Bool) -> Void

    var body: some View {
        Toggle(label, isOn: Binding(get: { checked }, set: onCheck))
    }
}

private struct CoordinateField: View {
    let label: String
    let value: String
    let onValue: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            Text(label).font(.labelMedium).foregroundStyle(Double(value) == nil ? theme.colors.error : theme.colors.onSurfaceVariant)
            TextField(label, text: Binding(get: { value }, set: onValue))
                .keyboardType(.numbersAndPunctuation)
                .textFieldStyle(.roundedBorder)
                .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(Double(value) == nil ? theme.colors.error : .clear, lineWidth: 1))
        }
        .frame(maxWidth: .infinity)
    }
}
