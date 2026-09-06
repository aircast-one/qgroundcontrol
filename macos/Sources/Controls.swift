import AppKit
import SwiftUI

// The shared control layer every native view composes from. QGC's QmlControls grew to
// 25k lines partly by reimplementing what the toolkit already provides, so this holds
// only the label/value arrangements that AppKit has no opinion about, and nothing that
// wraps a control SwiftUI already ships.

struct LabelledField: View {
    let label: String
    let value: String
    let commit: (String) -> Void

    @State private var draft = ""
    @FocusState private var editing: Bool

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .leading)
            TextField("", text: $draft)
                .onAppear { draft = value }
                // Only track the model while the field is not being edited, or every
                // keystroke fights the value read back from the fact.
                .onChange(of: value) { latest in if !editing { draft = latest } }
                .focused($editing)
                .onSubmit { commit(draft) }
                .onChange(of: editing) { focused in if !focused { commit(draft) } }
        }
    }
}

struct LabelledPicker: View {
    let label: String
    let options: [(String, String)]
    let selection: String
    let commit: (String) -> Void

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .leading)
            Picker("", selection: Binding(get: { selection }, set: commit)) {
                // A stored value the hardware no longer offers must still be visible,
                // or the picker silently misreports what the link is set to.
                if !options.contains(where: { $0.1 == selection }) {
                    Text(selection.isEmpty ? "Not set" : selection).tag(selection)
                }
                ForEach(options, id: \.1) { option in
                    Text(option.0).tag(option.1)
                }
            }
            .labelsHidden()
        }
    }
}

struct SectionCard<Content: View>: View {
    let title: String
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title)
                .font(.headline)
                .padding(.bottom, 8)
            content
        }
    }
}

struct MetricRow: View {
    let label: String
    let value: String
    let units: String

    var body: some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value).font(.body.monospacedDigit())
            if !units.isEmpty {
                Text(units).font(.caption).foregroundColor(.secondary)
                    .frame(width: 40, alignment: .leading)
            }
        }
    }
}

struct Notice: View {
    let text: String

    var body: some View {
        VStack {
            Spacer()
            Text(text).foregroundColor(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }
}
