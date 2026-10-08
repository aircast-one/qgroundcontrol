import SwiftUI

let SENSOR_SETTINGS_VIEW = "view.sensorSettings"
let SENSOR_SETTINGS_PRIORITY = "sensorSettings.priority"

struct CompassSettings: Equatable {
    var index: Int
    var label: String
    var device: String
    var use: Fact?
    var priority: Int?
    var orientation: Fact?
    var orientationTitle: String
}

struct Declination: Equatable {
    var manual: Bool
    var autoDecPath: String
    var value: Fact?
}

struct SensorSettings: Equatable {
    var boardRotation: Fact?
    var boardTitle: String
    var compassesWhileCalibrating: Bool
    var compasses: [CompassSettings]
    var priorities: [String]
    var helpSet: String
    var helpCal: String
    var simpleAccelHelp: String
    var declination: Declination?
}

private func control(_ json: JSON) -> Fact? {
    json.object != nil ? factFromControl(json) : nil
}

func sensorSettings(_ view: JSON?) -> SensorSettings? {
    guard let view, view["available"].bool else { return nil }
    let declination = view["declination"]
    return SensorSettings(
        boardRotation: control(view["boardRotation"]),
        boardTitle: view["boardTitle"].string,
        compassesWhileCalibrating: view["compassesWhileCalibrating"].bool,
        compasses: view["compasses"].array.filter { $0.object != nil }.map {
            CompassSettings(
                index: $0["index"].int(0),
                label: $0["label"].string,
                device: $0["device"].string,
                use: control($0["use"]),
                priority: $0["priority"].isNull ? nil : $0["priority"].int(0),
                orientation: control($0["orientation"]),
                orientationTitle: $0["orientationTitle"].string
            )
        },
        priorities: view["priorities"].array.map(\.string),
        helpSet: view["helpSet"].string,
        helpCal: view["helpCal"].string,
        simpleAccelHelp: view["simpleAccelHelp"].string,
        declination: declination.object == nil ? nil : Declination(
            manual: declination["manual"].bool,
            autoDecPath: declination["autoDecPath"].string,
            value: control(declination["value"])
        )
    )
}

let ORIENTATIONS_HELP = "Adjust orientations as needed.\n\nROTATION_NONE indicates component points in direction of flight."

struct CompassOrientations: View {
    @State private var revision = 0
    @State private var read: SensorSettings?

    var body: some View {
        let oriented = (read?.compasses ?? []).compactMap { compass in compass.orientation.map { (compass, $0) } }
        VStack(alignment: .leading, spacing: Space.s1) {
            if !oriented.isEmpty {
                Text(ORIENTATIONS_HELP).font(.bodySmall)
                ForEach(oriented, id: \.0.index) { compass, orientation in
                    FactRow(fact: orientation, title: compass.orientationTitle, onWrite: { revision += 1 })
                }
            }
        }
        .task { _ = await offMain { Qgc.invoke(SETUP_DIALOG_OPENED, SENSOR_SETTINGS_DIALOG, false) } }
        .task(id: revision) { read = await offMain { sensorSettings(Qgc.get(SENSOR_SETTINGS_VIEW)) } }
    }
}

struct SensorSettingsBlock: View {
    let calibrating: Bool
    let showCompasses: Bool
    var onSimpleAccel: ((Bool) -> Void)? = nil
    @State private var revision = 0
    @State private var read: SensorSettings?
    @State private var simple = false

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            if let settings = read {
                Text(calibrating ? settings.helpCal : settings.helpSet).font(.bodyMedium)
                if let board = settings.boardRotation {
                    FactRow(fact: board, title: settings.boardTitle, onWrite: refresh)
                }
                if let report = onSimpleAccel {
                    Text(settings.simpleAccelHelp).font(.bodySmall)
                    Toggle("Simple accelerometer calibration", isOn: Binding(get: { simple }, set: { on in
                        simple = on
                        report(on)
                    }))
                }
                if showCompasses && (!calibrating || settings.compassesWhileCalibrating) {
                    ForEach(settings.compasses, id: \.index) { compass in
                        compassBlock(settings, compass)
                    }
                    if let declination = settings.declination {
                        declinationBlock(declination)
                    }
                }
            }
        }
        .task { _ = await offMain { Qgc.invoke(SETUP_DIALOG_OPENED, SENSOR_SETTINGS_DIALOG, calibrating) } }
        .task(id: revision) { read = await offMain { sensorSettings(Qgc.get(SENSOR_SETTINGS_VIEW)) } }
    }

    private func refresh() { revision += 1 }

    @ViewBuilder
    private func compassBlock(_ settings: SensorSettings, _ compass: CompassSettings) -> some View {
        Text(compass.label).font(.titleSmall)
        if !compass.device.isBlank { Text(compass.device).font(.bodySmall) }
        if let use = compass.use {
            FactRow(fact: use, title: "Use compass", onWrite: refresh)
        }
        if let slot = compass.priority {
            PriorityPicker(options: settings.priorities, current: slot) { picked in
                let index = compass.index
                Task {
                    _ = await offMain { Qgc.invoke(SENSOR_SETTINGS_PRIORITY, index, picked) }
                    refresh()
                }
            }
        }
        if let orientation = compass.orientation {
            FactRow(fact: orientation, title: compass.orientationTitle, onWrite: refresh)
        }
    }

    @ViewBuilder
    private func declinationBlock(_ declination: Declination) -> some View {
        Text("Magnetic declination").font(.titleSmall)
        Toggle("Manual magnetic declination", isOn: Binding(get: { declination.manual }, set: { manual in
            let path = "\(declination.autoDecPath).rawValue"
            Task {
                _ = await offMain { Qgc.set(path, manual ? 0 : 1) }
                refresh()
            }
        }))
        if let value = declination.value {
            FactRow(fact: enabled(value, declination.manual), onWrite: refresh)
        }
    }

    private func enabled(_ fact: Fact, _ on: Bool) -> Fact {
        var shown = fact
        shown.enabled = on
        return shown
    }
}

private struct PriorityPicker: View {
    let options: [String]
    let current: Int
    let onPick: (Int) -> Void

    var body: some View {
        Menu {
            ForEach(Array(options.enumerated()), id: \.offset) { index, option in
                Button(option) { onPick(index) }
            }
        } label: {
            Text(options.indices.contains(current) ? options[current] : "")
        }
        .buttonStyle(.bordered)
    }
}
