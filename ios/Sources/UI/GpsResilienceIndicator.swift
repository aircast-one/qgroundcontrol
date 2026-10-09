import SwiftUI

let GPS_RESILIENCE_PATH = "view.gpsResilience"

struct ResilienceMark: Equatable {
    let shown: Bool
    let colour: String
}

struct ResilienceSection: Equatable {
    let title: String
    let rows: [(String, String)]

    static func == (lhs: ResilienceSection, rhs: ResilienceSection) -> Bool {
        lhs.title == rhs.title && lhs.rows.elementsEqual(rhs.rows, by: ==)
    }
}

struct GpsResilience: Equatable {
    let interference: ResilienceMark
    let authentication: ResilienceMark
    let sections: [ResilienceSection]
}

private func mark(_ json: JSON, _ key: String) -> ResilienceMark {
    json[key].objectOrNil.map { ResilienceMark(shown: $0["shown"].bool, colour: $0["colour"].string) } ?? ResilienceMark(shown: false, colour: "")
}

func gpsResilience(_ view: JSON?) -> GpsResilience? {
    guard let it = view, it["shown"].bool else { return nil }
    return GpsResilience(
        interference: mark(it, "interference"),
        authentication: mark(it, "authentication"),
        sections: it["sections"].objects.map { section in
            ResilienceSection(
                title: section["title"].string,
                rows: section["rows"].objects.map { ($0["label"].string, $0["text"].string) }
            )
        }
    )
}

private func markColour(_ colour: String, _ theme: Theme) -> Color {
    switch colour {
    case "good": theme.aircast.success
    case "warning": theme.aircast.warning
    case "alert": theme.aircast.alert
    case "error": theme.colors.error
    default: theme.colors.onSurfaceVariant
    }
}

struct GpsResilienceCell: View {
    @Environment(\.theme) private var theme
    @QgcPath(GPS_RESILIENCE_PATH) private var view
    @State private var open = false

    var body: some View {
        if let resilience = gpsResilience(view) {
            HStack(spacing: Space.s1) {
                if resilience.interference.shown {
                    Text("RF").font(.labelMedium).foregroundStyle(markColour(resilience.interference.colour, theme))
                }
                if resilience.authentication.shown {
                    Text("AUTH").font(.labelMedium).foregroundStyle(markColour(resilience.authentication.colour, theme))
                }
            }
            .minimumTouchTarget()
            .onTapGesture { open = true }
            .background {
                if open {
                    AircastSheet(onDismissRequest: { open = false }) {
                        VStack(alignment: .leading, spacing: 6) {
                            ForEach(Array(resilience.sections.enumerated()), id: \.offset) { _, section in
                                Text(section.title).font(.titleSmall)
                                ForEach(Array(section.rows.enumerated()), id: \.offset) { _, row in
                                    Text("\(row.0)  \(row.1)").font(.bodySmall)
                                }
                            }
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
