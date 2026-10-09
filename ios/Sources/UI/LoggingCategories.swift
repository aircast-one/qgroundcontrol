import SwiftUI

let LOG_CATEGORIES_VIEW = "view.logCategories"
let LOG_SET_CATEGORY = "appLog.setCategory"
let LOG_RESET_CATEGORIES = "appLog.resetCategories"
private let INDENT_PER_DEPTH = 12

struct LogCategory: Equatable, Hashable {
    let name: String
    let shortName: String
    let depth: Int
    let enabled: Bool
}

struct LogCategories: Equatable {
    let active: [String]
    let categories: [LogCategory]
}

func logCategories(_ view: JSON?) -> LogCategories? {
    guard let view else { return nil }
    return LogCategories(
        active: view["active"].strings,
        categories: view["categories"].objects.map {
            LogCategory(name: $0["name"].string, shortName: $0["shortName"].string, depth: $0["depth"].int(0), enabled: $0["enabled"].bool)
        }
    )
}

func filteredCategories(_ categories: [LogCategory], _ search: String) -> [LogCategory] {
    categories.filter { search.isEmpty || $0.name.range(of: search, options: .caseInsensitive) != nil }
}

func parentOf(_ categories: [LogCategory], _ category: LogCategory) -> LogCategory? {
    categories.first { candidate in
        candidate.depth == category.depth - 1
            && category.name.hasPrefix(candidate.name)
            && category.name.hasSuffix(category.shortName)
            && category.name.count > candidate.name.count + category.shortName.count
            && !category.name.dropFirst(candidate.name.count).dropLast(category.shortName.count)
                .contains { $0.isLetter || $0.isNumber || $0 == "_" }
    }
}

func parentNames(_ categories: [LogCategory]) -> Set<String> {
    Set(categories.compactMap { parentOf(categories, $0)?.name })
}

func shownInTree(_ categories: [LogCategory], _ expanded: Set<String>) -> [LogCategory] {
    func childrenOf(_ parent: LogCategory?) -> [LogCategory] { categories.filter { parentOf(categories, $0) == parent } }
    func walk(_ category: LogCategory) -> [LogCategory] {
        [category] + (expanded.contains(category.name) ? childrenOf(category).flatMap(walk) : [])
    }
    return childrenOf(nil).flatMap(walk)
}

struct LoggingCategoriesDialog: View {
    let onDismiss: () -> Void
    @State private var read: LogCategories?
    @State private var revision = 0
    @State private var search = ""
    @State private var expanded: Set<String> = []
    @Environment(\.theme) private var theme

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s2) {
                    HStack(spacing: Space.s2) {
                        TextField("Filter categories…", text: $search)
                            .textFieldStyle(.roundedBorder)
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                        Button("Clear") { search = "" }.buttonStyle(.borderless)
                    }
                    Text("Active categories").font(.titleSmall)
                    ForEach(read?.active ?? [], id: \.self) { name in
                        CategorySwitch(label: name, checked: true, depth: 0) { _ in act(LOG_SET_CATEGORY, name, false) }
                    }
                    Button("Reset all") { act(LOG_RESET_CATEGORIES) }.buttonStyle(.bordered)
                    tree(read?.categories ?? [])
                }
                .padding(Space.s4)
            }
            .navigationTitle("Logging categories")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("Close", action: onDismiss) }
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .task(id: revision) {
            read = await offMain { logCategories(Qgc.get(LOG_CATEGORIES_VIEW)) }
        }
    }

    @ViewBuilder
    private func tree(_ categories: [LogCategory]) -> some View {
        if search.isEmpty {
            let parents = parentNames(categories)
            Text("Categories").font(.titleSmall)
            ForEach(shownInTree(categories, expanded), id: \.name) { category in
                let opened = expanded.contains(category.name)
                CategorySwitch(
                    label: category.shortName,
                    checked: category.enabled,
                    depth: category.depth,
                    expander: parents.contains(category.name) ? opened : nil,
                    onExpand: { expanded = opened ? expanded.subtracting([category.name]) : expanded.union([category.name]) }
                ) { act(LOG_SET_CATEGORY, category.name, $0) }
            }
        } else {
            let found = filteredCategories(categories, search)
            Text("Search results").font(.titleSmall)
            ForEach(found, id: \.name) { category in
                CategorySwitch(label: category.name, checked: category.enabled, depth: 0) { act(LOG_SET_CATEGORY, category.name, $0) }
            }
            if found.isEmpty {
                Text("No matching categories").foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
    }

    private func act(_ path: String, _ args: Any?...) {
        Task {
            _ = await offMain { Qgc.call(path, arguments: args) }
            revision += 1
        }
    }
}

private struct CategorySwitch: View {
    let label: String
    let checked: Bool
    let depth: Int
    var expander: Bool? = nil
    var onExpand: () -> Void = {}
    let onChange: (Bool) -> Void

    var body: some View {
        HStack(spacing: 0) {
            if let opened = expander {
                Button(action: onExpand) {
                    Image(opened ? Icon.arrowDropDown : Icon.chevronRight)
                        .frame(width: 32, height: 32)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(opened ? "Collapse" : "Expand")
            }
            Text(label).frame(maxWidth: .infinity, alignment: .leading)
            Toggle(label, isOn: Binding(get: { checked }, set: onChange)).labelsHidden()
        }
        .padding(.leading, CGFloat(depth * INDENT_PER_DEPTH))
    }
}
