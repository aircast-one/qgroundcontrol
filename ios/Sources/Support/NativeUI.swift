import SwiftUI

// NATIVE UI — how list and form screens are built (iOS and the Mac app share these).
//
// List and form screens use the platform's own containers so they read as iOS (and, unchanged, as a Mac grouped
// form). The Fly and Plan maps stay map-first; only their sheets and panels follow these rules.
//
// Page / navigation
//   • A page lives in its host's NavigationStack (Settings: SettingsSheet; Analyze: AnalyzeScreen's
//     NavigationSplitView; link editors: their own sheet stack). Never draw a top bar or a back arrow.
//   • Title: the host's `.nativePage(title)` (inline; `large: true` for a tab's first page). A sheet's own root
//     uses `.navigationTitle` + `.navigationBarTitleDisplayMode(.inline)`. `PageTopBar(title:…)` still compiles
//     and now only lends its title to the host's nativePage.
//   • Push with `.navigationDestination(item:)` / `.navigationDestination(isPresented:)` bound to the screen's
//     EXISTING state (e.g. `openComponent`), or append to the host's route path. Declare destinations on the
//     page root, never inside a Form/List/Lazy* (they are ignored there). The system draws back + swipe-back.
//   • Every pushed page gets `.nativePage(title)` from its host: the inline title, and the BlocksNavigation guard
//     (calibration, log download, flashing hide back/swipe-back and explain why instead). Pages themselves do
//     not set navigationTitle; to retitle from inside, or to step back through the page's own sub-steps first,
//     place `OverridePageHeading(title:onBack:)` in the page.
//
// Containers
//   • `NativeForm { … }` — the inset-grouped Form every list/form page uses (sets LocalNativeList so the shared
//     row primitives below drop their hand-drawn padding/borders; makes plain Buttons borderless so several
//     buttons in one row stay independent).
//   • Section: `Section { rows } header: { SectionHeader(text: "Sensors") } footer: { Text(note) }` — SectionHeader
//     renders as plain header text inside a NativeForm. A device section: `DeviceSection(title:badge:) { rows }`
//     becomes a real Section inside a NativeForm by itself.
//   • A composite view not yet converted (draws its own headers/padding): put it in a section with
//     `.legacyFormRow()` so it renders as before, edge to edge, on the grouped background.
//   • Explanatory text under a section: the section footer, or `FormNote(text)` as a free-standing note row.
//
// Rows
//   • Disclosure row (opens a page or sheet): `SetupRow(title:status:state:onClick:icon:subtitle:)` — icon badge,
//     title, secondary subtitle, trailing status, chevron. Inside a NavigationSplitView sidebar / List(selection:)
//     use `NavigationLink(value:) { NativeRowLabel(…, chevron: false) }` instead.
//   • Value row (read-only): `LabeledContent("Firmware", value: text)` (DeviceFact does this in a NativeForm).
//   • Editable text: `FieldRow(label:value:prompt:keyboard:error:onChange:)` — label left, value right-aligned,
//     the keyboard type kept, the validation error as red secondary text under the row. DeviceTextField and
//     the settings FactRow field do the same inside a NativeForm.
//   • Choice: `ChoiceField(label:value:options:onPick:)` (a menu row inside a NativeForm) or a native `Picker`.
//     On/off: `Toggle`. Numbers with steps: `Stepper` or the FactRow value sheet.
//   • Collapsible group (Advanced): `DisclosureGroup("Advanced", isExpanded:) { rows }` inside a Section, or
//     `AdvancedToggle` as its own row followed by the extra sections.
//
// Actions
//   • Page primary action (Connect, Save, Start): `.pagePrimaryAction("Connect", enabled:) { … }` — the trailing
//     nav-bar button. Never a full-width pill at the bottom of the page.
//   • Cancel / close of a modal: `.pageCancelAction { … }` (leading, replaces back). One dismiss control per screen.
//   • An action inside the content: a standard `Button(…).buttonStyle(.borderedProminent)` (or `.bordered`)
//     at its regular size, or a tinted text Button row in its own Section.
//   • Destructive: `Button(role: .destructive)` as a row in its own Section, keeping its existing confirmation
//     alert / dialog (and hold-to-confirm for flight actions) untouched.
//
// Sheets
//   • `.queuedSheet(isPresented:) { NavigationStack { NativeForm { … }.navigationTitle(…)
//     .pageCancelAction { … }.pagePrimaryAction(…) { … } }.presentationDetents([.medium, .large])
//     .nativeSheetFrame() }` — a navigation bar with Cancel left and the primary action right; detents on iOS,
//     a sized sheet with its buttons on the Mac.

extension EnvironmentValues {
    // True inside a NativeForm: the shared row primitives leave padding, borders and backgrounds to the list.
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
    // The padding a row draws for itself on a hand-laid-out page; none inside a NativeForm, whose rows are inset.
    func rowPadding(_ insets: EdgeInsets) -> some View { modifier(LegacyRowPadding(insets: insets)) }

    func rowPadding(horizontal: CGFloat = 0, vertical: CGFloat = 0) -> some View {
        rowPadding(EdgeInsets(top: vertical, leading: horizontal, bottom: vertical, trailing: horizontal))
    }

    // A composite view that still lays itself out by hand, shown inside a NativeForm as it always looked.
    func legacyFormRow() -> some View {
        environment(\.LocalNativeList, false)
            .listRowInsets(EdgeInsets())
            .listRowBackground(Color.clear)
            .listRowSeparator(.hidden)
    }
}

// The rounded-square glyph at the start of a native row, as iOS Settings draws them, in Aircast's colours.
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

// A native row's content: icon badge, title, secondary subtitle, trailing status and (for rows that open a page
// from a Button) the disclosure chevron. SetupRow draws this inside a NativeForm.
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
    // A whole-row button inside a list: the row highlights on iOS; on the Mac a grouped form would draw a push
    // button, so it stays a plain row with a hover tint.
    func listRowButton() -> some View {
        #if os(macOS)
        return buttonStyle(.plain).macHover()
        #else
        return buttonStyle(.automatic)
        #endif
    }
}

// An inline labelled text field: the label on the left, the value typed on the right, as iOS Settings > Wi-Fi >
// Configure IP. The error (or a hint) shows under the row in secondary text.
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
        // The label is drawn by LabeledContent; the field keeps it only for accessibility (an empty prompt, so
        // iOS does not repeat the label as the placeholder, and hidden labels, so the Mac does not draw it twice).
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

// Free-standing explanatory text between sections, drawn like a section footer.
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
    // The page's primary action as the trailing navigation-bar button (Connect, Save, Start), in place of a
    // full-width button at the foot of the page.
    func pagePrimaryAction(_ title: String, enabled: Bool = true, action: @escaping () -> Void) -> some View {
        toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(title, action: action).disabled(!enabled)
            }
        }
    }

    // A modal page's Cancel, leading, in place of the back button: the screen's one dismiss control.
    func pageCancelAction(_ title: String = "Cancel", action: @escaping () -> Void) -> some View {
        navigationBarBackButtonHidden(true)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(title, action: action)
                }
            }
    }

    // A modal sheet's size on the Mac, where a sheet is only as big as its content asks; nothing on iOS.
    func nativeSheetFrame() -> some View {
        #if os(macOS)
        return frame(minWidth: NATIVE_SHEET_MIN_WIDTH, minHeight: NATIVE_SHEET_MIN_HEIGHT)
        #else
        return self
        #endif
    }

    // A pushed page's navigation bar: its title (inline, or large for a tab's first page), the slot through which
    // OverridePageHeading / PageTopBar inside the page retitle it or step back through their own sub-steps, and
    // the BlocksNavigation guard — while a calibration, log download or flash holds, back and swipe-back are
    // replaced by a back that explains why instead of leaving.
    func nativePage(_ title: String, large: Bool = false) -> some View { modifier(NativePage(title: title, large: large)) }
}

#if os(macOS)
private let NATIVE_SHEET_MIN_WIDTH: CGFloat = 480
private let NATIVE_SHEET_MIN_HEIGHT: CGFloat = 440
#endif

// The back button's own look, for the stand-ins that replace it.
struct NavigationBackLabel: View {
    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: "chevron.backward").fontWeight(.semibold)
            Text("Back")
        }
    }
}

// Whether a page's back is replaced, and by what: BlocksNavigation's refusal wins over a page's own step back.
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
