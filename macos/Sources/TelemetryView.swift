import SwiftUI

struct TelemetryView: View {
    @ObservedObject var telemetry: Telemetry

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            GroupBox("Attitude") {
                VStack(spacing: 4) {
                    ForEach(telemetry.gauges) { gauge in
                        row(gauge.label, value: gauge.reading.text, units: gauge.reading.units)
                    }
                }
                .padding(6)
            }
            GroupBox("Bridge round trip") { health.padding(6) }
            GroupBox("Watched paths") { watched.padding(6) }
            Spacer(minLength: 0)
        }
        .padding(16)
        .frame(minWidth: 380, minHeight: 560)
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Circle()
                    .fill(telemetry.connected ? Color.green : Color.secondary.opacity(0.4))
                    .frame(width: 9, height: 9)
                Text(telemetry.connected ? telemetry.vehicleType : "No vehicle")
                    .font(.headline)
                Spacer()
                if telemetry.armed {
                    Text("ARMED")
                        .font(.caption.weight(.semibold))
                        .padding(.horizontal, 7).padding(.vertical, 2)
                        .background(Color.red.opacity(0.16))
                        .foregroundColor(.red)
                        .cornerRadius(4)
                }
            }
            Text(telemetry.flightMode)
                .font(.title2.weight(.medium))
        }
    }

    private var health: some View {
        VStack(spacing: 4) {
            row("Last tick", value: String(format: "%.2f", telemetry.lastTickMillis), units: "ms")
            row("Worst tick", value: String(format: "%.2f", telemetry.worstTickMillis), units: "ms")
            row("Ticks", value: String(telemetry.ticks), units: "@ 10 Hz")
            row("Over \(Int(Telemetry.budgetMillis)) ms", value: String(telemetry.overBudget), units: "")
            HStack {
                Text(verdict).font(.callout.weight(.semibold)).foregroundColor(verdictColor)
                Spacer()
                Button("Reset", action: telemetry.resetMetrics).controlSize(.small)
            }
            .padding(.top, 4)
        }
    }

    private var watched: some View {
        VStack(alignment: .leading, spacing: 3) {
            if telemetry.events.isEmpty {
                Text("waiting for a change…")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(.secondary)
            }
            ForEach(telemetry.events, id: \.self) { event in
                Text(event)
                    .font(.system(size: 11, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private var verdict: String {
        if telemetry.ticks == 0 { return "no samples yet" }
        return telemetry.overBudget == 0
            ? "PASS — every tick under budget"
            : "FAIL — \(telemetry.overBudget) of \(telemetry.ticks) over budget"
    }

    private var verdictColor: Color {
        telemetry.ticks == 0 ? .secondary : (telemetry.overBudget == 0 ? .green : .red)
    }

    private func row(_ label: String, value: String, units: String) -> some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value).font(.body.monospacedDigit())
            if !units.isEmpty {
                Text(units).foregroundColor(.secondary).font(.caption)
                    .frame(width: 42, alignment: .leading)
            } else {
                Spacer().frame(width: 42)
            }
        }
    }
}
