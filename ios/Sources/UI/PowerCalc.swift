import SwiftUI

private let POWER_CALC_VIEW = "view.powerCalc"
private let POWER_CALCULATE = "vehicleConfig.calculate"
private let READING_POLL_MS = 1000

struct PowerCalculator: Equatable, Hashable {
    var title: String
    var help: String
    var measure: String
    var measuredLabel: String
    var readingLabel: String
    var paramLabel: String
    var button: String
    var noReading: String
    var batteryIndex: Int
    var param: String
}

func powerCalculator(_ json: JSON?) -> PowerCalculator? {
    guard let json, !json["param"].string.isBlank else { return nil }
    return PowerCalculator(
        title: json["title"].string,
        help: json["help"].string,
        measure: json["measure"].string,
        measuredLabel: json["measuredLabel"].string,
        readingLabel: json["readingLabel"].string,
        paramLabel: json["paramLabel"].string,
        button: json["button"].string,
        noReading: json["noReading"].string,
        batteryIndex: json["batteryIndex"].int(0),
        param: json["param"].string
    )
}

func powerCalcPath(_ calculator: PowerCalculator) -> String {
    "\(POWER_CALC_VIEW)(\(calculator.measure),\(calculator.batteryIndex),\(calculator.param))"
}

struct PowerCalcDialog: View {
    let calculator: PowerCalculator
    let onDone: () -> Void
    @State private var measured = ""
    @State private var live: JSON?
    @State private var refusal: String?
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme

    var body: some View {
        let available = live?["readingAvailable"].bool == true
        SetupDialog(title: sentenceCase(calculator.title)) {
            Text(calculator.help).font(.bodySmall)
            if !available && !calculator.noReading.isBlank {
                Text(calculator.noReading).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(calculator.measuredLabel).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                TextField(calculator.measuredLabel, text: $measured.decimalPoint)
                    .keyboardType(.decimalPad)
                    .textFieldStyle(.roundedBorder)
            }
            if available || calculator.noReading.isBlank {
                Text("\(calculator.readingLabel) \(live?["readingText"].string ?? "")")
            }
            Text("\(calculator.paramLabel) \(live?["paramText"].string ?? "")")
            if let refusal {
                Text(refusal).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        } buttons: {
            Button("Close", action: onDone)
            Button(sentenceCase(calculator.button)) {
                let entered = measured
                let shown = calculator
                scope.launch {
                    let answer = await offMain { Qgc.refusalOf(POWER_CALCULATE, shown.param, shown.measure, shown.batteryIndex, entered) }
                    guard !Task.isCancelled else { return }
                    refusal = answer
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(!available)
        }
        .task(id: calculator) {
            let path = powerCalcPath(calculator)
            while !Task.isCancelled {
                let reading = await offMain { Qgc.get(path) }
                guard !Task.isCancelled else { return }
                live = reading
                try? await Task.sleep(for: .milliseconds(READING_POLL_MS))
            }
        }
        .onDisappear { scope.cancel() }
    }
}
