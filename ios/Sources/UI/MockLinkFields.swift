import SwiftUI

let MOCK_VEHICLES: [(String, String)] = [
    ("px4", "PX4 Vehicle"),
    ("apmCopter", "APM ArduCopter Vehicle"),
    ("apmPlane", "APM ArduPlane Vehicle"),
    ("apmSub", "APM ArduSub Vehicle"),
    ("apmRover", "APM ArduRover Vehicle"),
    ("generic", "Generic Vehicle"),
]

let MOCK_VIDEO_STREAMS = ["Disabled", "RTP/UDP H.264", "RTP/UDP H.265", "RTSP (H.264)", "MPEG-TS (UDP)", "MPEG-TS (TCP)"]

struct MockLinkChoices: Equatable {
    var sendStatusText = false
    var camera = false
    var gimbal = false
    var proximity = false
    var freshParams = false
    var vehicle = 0
    var videoStream = 0
}

func mockVehicleIsApm(_ choices: MockLinkChoices) -> Bool { MOCK_VEHICLES[choices.vehicle].0.hasPrefix("apm") }

func mockLinkArguments(_ choices: MockLinkChoices) -> [Any] {
    [
        MOCK_VEHICLES[choices.vehicle].0,
        choices.sendStatusText,
        choices.camera,
        choices.gimbal,
        choices.proximity,
        choices.freshParams && mockVehicleIsApm(choices),
        choices.videoStream,
    ]
}

func withMockVehicle(_ choices: MockLinkChoices, _ vehicle: Int) -> MockLinkChoices {
    let picked = withChanges(choices) { $0.vehicle = vehicle }
    return mockVehicleIsApm(picked) ? picked : withChanges(picked) { $0.freshParams = false }
}

private struct MockCheck: View {
    let text: String
    let checked: Bool
    let onChecked: (Bool) -> Void

    var body: some View {
        Toggle(isOn: Binding(get: { checked }, set: onChecked)) {
            Text(text).font(.bodyLarge)
        }
        .frame(minHeight: 48)
    }
}

struct MockLinkFields: View {
    let choices: MockLinkChoices
    let onChange: (MockLinkChoices) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ChoiceField(label: "Vehicle Type", value: MOCK_VEHICLES[choices.vehicle].1, options: MOCK_VEHICLES.map(\.1)) { onChange(withMockVehicle(choices, $0)) }
            MockCheck(text: "Send status text + voice", checked: choices.sendStatusText) { on in onChange(withChanges(choices) { $0.sendStatusText = on }) }
            MockCheck(text: "Enable camera", checked: choices.camera) { on in onChange(withChanges(choices) { $0.camera = on }) }
            MockCheck(text: "Enable gimbal", checked: choices.gimbal) { on in onChange(withChanges(choices) { $0.gimbal = on }) }
            MockCheck(text: "Enable proximity sensors", checked: choices.proximity) { on in onChange(withChanges(choices) { $0.proximity = on }) }
            if mockVehicleIsApm(choices) {
                MockCheck(text: "Start with fresh firmware parameters (setup required)", checked: choices.freshParams) { on in onChange(withChanges(choices) { $0.freshParams = on }) }
            }
            if choices.camera {
                ChoiceField(label: "Served Video Stream", value: MOCK_VIDEO_STREAMS[choices.videoStream], options: MOCK_VIDEO_STREAMS) { picked in onChange(withChanges(choices) { $0.videoStream = picked }) }
                    .padding(.top, 8)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
