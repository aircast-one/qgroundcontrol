import SwiftUI

extension EnvironmentValues {
    @Entry var LocalNativeList = false
}

struct NativeForm<Content: View>: View {
    let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        Form { content }
            .formStyle(.grouped)
            .buttonStyle(.borderless)
            .scrollDismissesKeyboard(.interactively)
            .environment(\.LocalNativeList, true)
    }
}

private struct LegacyRowPadding: ViewModifier {
    let insets: EdgeInsets
    @Environment(\.LocalNativeList) private var nativeList

    func body(content: Content) -> some View {
        content.padding(nativeList ? EdgeInsets() : insets)
    }
}

extension View {
    func rowPadding(_ insets: EdgeInsets) -> some View { modifier(LegacyRowPadding(insets: insets)) }

    func rowPadding(horizontal: CGFloat = 0, vertical: CGFloat = 0) -> some View {
        rowPadding(EdgeInsets(top: vertical, leading: horizontal, bottom: vertical, trailing: horizontal))
    }

    func legacyFormRow() -> some View {
        environment(\.LocalNativeList, false)
            .listRowInsets(EdgeInsets())
            .listRowBackground(Color.clear)
            .listRowSeparator(.hidden)
    }
}

struct NativeRowIcon: View {
    let icon: Icon
    var tint: Color? = nil
    @Environment(\.theme) private var theme

    var body: some View {
        Image(icon)
            .font(.system(size: NATIVE_ICON_GLYPH, weight: .medium))
            .foregroundStyle(Color.white)
            .frame(width: NATIVE_ICON_SIZE, height: NATIVE_ICON_SIZE)
            .background(tint ?? theme.colors.primary, in: RoundedRectangle(cornerRadius: NATIVE_ICON_CORNER, style: .continuous))
            .accessibilityHidden(true)
    }
}

let NATIVE_ICON_SIZE: CGFloat = 29
private let NATIVE_ICON_GLYPH: CGFloat = 15
private let NATIVE_ICON_CORNER: CGFloat = 7

struct NativeRowLabel: View {
    let title: String
    var subtitle: Text? = nil
    var status: String = ""
    var statusColor: Color? = nil
    var icon: Icon? = nil
    var iconTint: Color? = nil
    var dimmed: Bool = false
    var chevron: Bool = true

    var body: some View {
        HStack(spacing: Space.s3) {
            if let icon { NativeRowIcon(icon: icon, tint: iconTint) }
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .foregroundStyle(dimmed ? Color.secondary : Color.primary)
                    .multilineTextAlignment(.leading)
                if let subtitle {
                    subtitle
                        .font(.subheadline)
                        .foregroundStyle(Color.secondary)
                        .multilineTextAlignment(.leading)
                        .lineLimit(3)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if !status.isBlank {
                Text(status)
                    .foregroundStyle(statusColor ?? Color.secondary)
                    .lineLimit(1)
            }
            if chevron {
                Image(systemName: "chevron.right")
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(Color.secondary.opacity(0.6))
                    .accessibilityHidden(true)
            }
        }
        .contentShape(Rectangle())
    }
}

extension View {
    func listRowButton() -> some View {
        return buttonStyle(.automatic)
    }
}

struct FieldRow: View {
    let label: String
    let value: String
    var prompt: String = ""
    var error: String? = nil
    var hint: String = ""
    var enabled: Bool = true
    var keyboard: UIKeyboardType = .default
    var autocapitalize = false
    var secure = false
    var onSubmit: () -> Void = {}
    let onChange: (String) -> Void
    @Environment(\.theme) private var theme
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            LabeledContent {
                field
                    .multilineTextAlignment(.trailing)
                    .keyboardType(keyboard)
                    .textInputAutocapitalization(autocapitalize ? TextInputAutocapitalization.sentences : TextInputAutocapitalization.never)
                    .autocorrectionDisabled(!autocapitalize)
                    .submitLabel(.done)
                    .onSubmit(onSubmit)
                    .focused($focused)
                    .disabled(!enabled)
                    .toolbar {
                        if focused && KEYBOARDS_WITHOUT_RETURN.contains(keyboard) {
                            ToolbarItemGroup(placement: .keyboard) {
                                Spacer()
                                Button("Done") { focused = false }
                            }
                        }
                    }
            } label: {
                Text(label).foregroundStyle(error == nil ? Color.primary : theme.colors.error)
            }
            if let error, !error.isBlank {
                Text(error).font(.footnote).foregroundStyle(theme.colors.error)
            } else if !hint.isBlank {
                Text(hint).font(.footnote).foregroundStyle(Color.secondary)
            }
        }
        .accessibilityElement(children: .contain)
    }

    @ViewBuilder private var field: some View {
        let text = Binding(get: { value }, set: onChange)
        Group {
            if secure {
                SecureField(label, text: text, prompt: Text(prompt))
            } else {
                TextField(label, text: text, prompt: Text(prompt))
            }
        }
        .labelsHidden()
    }
}

struct FormNote: View {
    let text: String
    var color: Color? = nil

    var body: some View {
        Text(text)
            .font(.footnote)
            .foregroundStyle(color ?? Color.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .listRowBackground(Color.clear)
            .listRowInsets(EdgeInsets(top: Space.s1, leading: Space.s1, bottom: Space.s1, trailing: Space.s1))
    }
}

extension View {
    func pagePrimaryAction(_ title: String, enabled: Bool = true, action: @escaping () -> Void) -> some View {
        toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(title, action: action).disabled(!enabled)
            }
        }
    }

    func pageCancelAction(_ title: String = "Cancel", action: @escaping () -> Void) -> some View {
        navigationBarBackButtonHidden(true)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(title, action: action)
                }
            }
    }

    func nativePage(_ title: String, large: Bool = false) -> some View { modifier(NativePage(title: title, large: large)) }
}

struct NavigationBackLabel: View {
    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: "chevron.backward").fontWeight(.semibold)
            Text("Back")
        }
    }
}

enum PageBack: Equatable {
    case System
    case Refused(String)
    case Step
}

func pageBack(_ blockedReason: String?, _ stepsBack: Bool) -> PageBack {
    if let blockedReason { return .Refused(blockedReason) }
    return stepsBack ? .Step : .System
}

private struct NativePage: ViewModifier {
    let title: String
    let large: Bool
    @Environment(AppNavigationState.self) private var navigation
    @State private var heading = PageHeadingSlot()
    @State private var refusal: String?

    func body(content: Content) -> some View {
        let override = heading.value
        let back = pageBack(navigation.blockedReason, override?.back != nil)
        content
            .environment(\.LocalPageHeading, heading)
            .navigationTitle(override?.title ?? title)
            .navigationBarTitleDisplayMode(large ? .large : .inline)
            .navigationBarBackButtonHidden(back != .System)
            .toolbar {
                if back != .System {
                    ToolbarItem(placement: .cancellationAction) {
                        Button {
                            switch pageBack(navigation.blockedReason, heading.value?.back != nil) {
                            case .Refused(let reason): refusal = navigationRefusal(reason, true)
                            case .Step: heading.value?.back?()
                            case .System: break
                            }
                        } label: {
                            NavigationBackLabel()
                        }
                        .accessibilityLabel("Back")
                    }
                }
            }
            .alert(refusal ?? "", isPresented: presented($refusal)) {
                Button("OK", role: .cancel) {}
            }
    }
}
