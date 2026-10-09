import SwiftUI

let ESC_PATH = "view.escs"

struct EscMotor: Equatable {
    let title: String
    let healthy: Bool
    let rows: [(String, String)]

    static func == (lhs: EscMotor, rhs: EscMotor) -> Bool {
        lhs.title == rhs.title && lhs.healthy == rhs.healthy && lhs.rows.elementsEqual(rhs.rows, by: ==)
    }
}

struct EscSummary: Equatable {
    let onlineCount: Int
    let healthy: Bool
    let healthText: String
    let healthyMotorsText: String
    let totalErrors: Int64
    let motors: [EscMotor]
}

func escSummary(_ view: JSON?) -> EscSummary? {
    guard let it = view, it["shown"].bool else { return nil }
    return EscSummary(
        onlineCount: it["onlineCount"].int(0),
        healthy: it["healthy"].bool,
        healthText: it["healthText"].string,
        healthyMotorsText: it["healthyMotorsText"].string,
        totalErrors: it["totalErrors"].int64 ?? 0,
        motors: it["motors"].array.filter { $0.object != nil }.map { motor in
            EscMotor(
                title: motor["title"].string,
                healthy: motor["healthy"].bool,
                rows: [
                    ("RPM", motor["rpm"].string),
                    ("Temp", motor["temperature"].string),
                    ("Voltage", motor["voltage"].string),
                    ("Current", motor["current"].string),
                    ("Errors", motor["errors"].string),
                ]
            )
        }
    )
}

private let ESC_LIST_MAX_HEIGHT: CGFloat = 420

func escCellText(_ summary: EscSummary) -> String { "ESC \(summary.onlineCount) \(summary.healthText)" }

struct EscIndicatorCell: View {
    @Environment(\.theme) private var theme
    @QgcPath(ESC_PATH) private var view
    @State private var open = false
    @State private var motorsHeight: CGFloat?

    var body: some View {
        if let summary = escSummary(view) {
            let good = theme.aircast.success
            let bad = theme.colors.error
            Text(escCellText(summary))
                .font(.labelMedium)
                .foregroundStyle(summary.healthy ? good : bad)
                .lineLimit(1)
                .onTapGesture { open = true }
                .background {
                    if open {
                        AircastSheet(onDismissRequest: { open = false }) {
                            VStack(alignment: .leading, spacing: Space.s2) {
                                Text("ESC Status Overview").font(.titleMedium)
                                Text("Healthy Motors  \(summary.healthyMotorsText)").font(.bodyMedium)
                                Text("Total Errors  \(summary.totalErrors)").font(.bodyMedium)
                                ScrollView {
                                    VStack(alignment: .leading, spacing: 0) {
                                        ForEach(summary.motors, id: \.title) { motor in
                                            VStack(alignment: .leading, spacing: 0) {
                                                Text(motor.title).font(.titleSmall).foregroundStyle(motor.healthy ? good : bad)
                                                ForEach(motor.rows.filter { !$0.1.isBlank }, id: \.0) { label, value in
                                                    Text("\(label)  \(value)").font(.bodySmall)
                                                }
                                            }
                                            .padding(.vertical, 6)
                                        }
                                    }
                                    .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { motorsHeight = $0 }
                                }
                                .scrollBounceBehavior(.basedOnSize)
                                .frame(maxHeight: min(motorsHeight ?? ESC_LIST_MAX_HEIGHT, ESC_LIST_MAX_HEIGHT))
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.horizontal, Space.s5)
                            .padding(.bottom, Space.s6)
                        }
                    }
                }
        }
    }
}
