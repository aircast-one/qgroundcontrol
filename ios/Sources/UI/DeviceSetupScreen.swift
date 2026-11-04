import SwiftUI

private let SETUP_TOP = "device-setup-top"

struct DeviceSetupScreen: View {
    let props: DeviceScreenProps
    @DeviceView("setup") private var reading

    var body: some View {
        if let view = deviceView(reading, { parseDeviceSetup($0) }) {
            SetupWizard(view: view, onOpen: props.onOpen)
        } else {
            NativeForm { FootNote(text: DEVICE_WAITING) }
        }
    }
}

private func setupPrimaryTitle(_ nav: SetupNav) -> String {
    if nav.finish { return "Done" }
    return nav.next == .Skip ? "Skip" : "Next"
}

// The setup wizard as a native form: the step trail and the step's heading lead, the step's own rows follow. The
// navigation bar steps through it — its back goes to the previous step (OverridePageHeading), Next / Skip / Done
// sit where a page's primary action does — and "Finish setup later" closes Settings.
private struct SetupWizard: View {
    let view: DeviceSetupView
    let onOpen: (String) -> Void
    @Environment(AppNavigationState.self) private var navigation
    @Environment(\.LocalDeviceScroll) private var scroll
    @Environment(\.LocalChangeNotice) private var notice
    @State private var step: Int
    @State private var reconfiguring = false
    @State private var wizardStep = 1
    @State private var finishing = DeviceActionState()

    init(view: DeviceSetupView, onOpen: @escaping (String) -> Void) {
        self.view = view
        self.onOpen = onOpen
        _step = State(initialValue: view.start)
    }

    private func go(_ next: Int) { step = clampStep(next, view.steps.count) }

    private func close() { navigation.settingsOpen = false }

    private func finish() {
        let notice = notice, finishing = finishing, navigation = navigation
        Task { @MainActor in
            if await finishing.run(DeviceRequest(method: .POST, path: "/api/setup/complete"), notice: notice) { navigation.settingsOpen = false }
        }
    }

    private func primary(_ nav: SetupNav) {
        if nav.finish {
            finish()
        } else if nav.next != nil {
            go(step + 1)
        }
    }

    var body: some View {
        let total = view.steps.count
        let current = view.steps[clampStep(step, total) - 1]
        let wizardShown = view.camera.wizard.isNew || reconfiguring
        let nav = setupNav(step, total, cameraHoldsTheWizard(current.key, wizardShown, wizardStep))
        NativeForm {
            Section {
                StepTrail(steps: view.steps, step: step).id(SETUP_TOP)
                VStack(alignment: .leading, spacing: Space.s1) {
                    HStack(alignment: .firstTextBaseline, spacing: Space.s3) {
                        Text(current.title).font(.title3.weight(.semibold)).frame(maxWidth: .infinity, alignment: .leading).accessibilityAddTraits(.isHeader)
                        if let status = current.status { DeviceBadgeText(badge: status) }
                    }
                    if !current.lead.isEmpty {
                        Text(current.lead).font(.subheadline).foregroundStyle(Color.secondary)
                    }
                }
                .padding(.vertical, Space.s1)
            }
            switch current.key {
            case "network":
                Section { InternetStep(internet: view.internet, onOpen: onOpen) }
            case "tailscale":
                TailscaleSection(tailscale: view.remote)
            case "camera":
                Section {
                    CameraStep(
                        camera: view.camera,
                        wizardShown: wizardShown,
                        onReconfigure: {
                            wizardStep = 1
                            reconfiguring = true
                        },
                        onStop: { reconfiguring = false },
                        onWizardStep: { wizardStep = $0 },
                        onOpen: onOpen
                    )
                }
            case "telemetry":
                Section { TelemetryStep(telemetry: view.telemetry) }
            case "done":
                Section {
                    ForEach(view.checks, id: \.label) { check in
                        SetupRow(
                            title: check.label,
                            status: check.status.text,
                            state: check.status.level == DeviceLevel.Ready ? .Done : .Neutral,
                            onClick: { go(check.step) }
                        )
                    }
                }
            default:
                EmptyView()
            }
            if nav.later || finishing.refusal != nil {
                Section {
                    if nav.later { Button("Finish setup later", action: close) }
                    DeviceRefusalText(refusal: finishing.refusal)
                }
            }
        }
        .background {
            if nav.back { OverridePageHeading(title: DEVICE_SCREENS["/setup"]?.title ?? "", onBack: { go(step - 1) }) }
        }
        .pagePrimaryAction(setupPrimaryTitle(nav), enabled: nav.finish ? !finishing.busy : nav.next != nil) { primary(nav) }
        .onChange(of: step) { withAnimation { scroll?.scrollTo(SETUP_TOP, anchor: .top) } }
    }
}

private struct StepTrail: View {
    let steps: [DeviceSetupStep]
    let step: Int
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 6) {
            ForEach(Array(steps.enumerated()), id: \.element.key) { index, entry in
                let here = index + 1 == step
                let done = !here && entry.status != nil
                if index > 0 {
                    Rectangle().fill(theme.colors.outlineVariant).frame(width: 8, height: 1)
                }
                ZStack {
                    Circle().fill(here ? theme.colors.primary : done ? theme.aircast.success : Color.clear)
                    Circle().stroke(here ? theme.colors.primary : done ? Color.clear : theme.colors.outlineVariant, lineWidth: 1)
                    if done {
                        Image(.check).font(.system(size: 12, weight: .bold)).foregroundStyle(theme.colors.surface).accessibilityLabel("done")
                    } else {
                        Text("\(index + 1)").font(.labelSmall).foregroundStyle(here ? theme.colors.onPrimary : theme.colors.onSurfaceVariant)
                    }
                }
                .frame(width: 24, height: 24)
                .accessibilityAddTraits(here ? .isSelected : [])
                if here {
                    Text(entry.label).font(.labelMedium).foregroundStyle(theme.colors.onSurface).lineLimit(1).truncationMode(.tail)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, Space.s1)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Setup steps")
    }
}

private struct PageRow: View {
    let page: DevicePage
    let onOpen: (String) -> Void

    var body: some View {
        SetupRow(
            title: page.title,
            state: page.needs ? .NeedsAttention : .Neutral,
            onClick: { onOpen(page.href) },
            icon: devicePageIcon(page.icon),
            subtitle: page.summary
        )
    }
}

private struct InternetStep: View {
    let internet: SetupInternet
    let onOpen: (String) -> Void

    var body: some View {
        DeviceNotice(notice: internet.online)
        WifiJoin(reopen: internet.reopen, initiallyOpen: internet.joinOpen)
        PageRow(page: internet.connection, onOpen: onOpen)
        DeviceText(text: internet.hint)
    }
}

private struct CameraStep: View {
    let camera: SetupCamera
    let wizardShown: Bool
    let onReconfigure: () -> Void
    let onStop: () -> Void
    let onWizardStep: (Int) -> Void
    let onOpen: (String) -> Void
    @Environment(\.LocalDeviceOrigin) private var origin

    var body: some View {
        if !camera.problem.isEmpty { DeviceNotice(notice: DeviceBadge(level: DeviceLevel.Warn, text: camera.problem)) }
        if wizardShown {
            CameraWizard(view: camera.wizard, onCancel: camera.wizard.isNew ? nil : onStop, onStep: onWizardStep, onDone: { _ in onStop() })
                .id(camera.wizard.isNew ? "" : camera.wizard.name)
        } else {
            DeviceButtonRow {
                if !camera.watch.isEmpty {
                    LinkButton(href: resolveDeviceUrl(origin, camera.watch), text: "▶ Watch live", look: .Filled, external: true)
                    Button("Reconfigure", action: onReconfigure).buttonStyle(.bordered)
                } else {
                    Button("Reconfigure", action: onReconfigure).buttonStyle(.filled)
                }
            }
            PageRow(page: camera.cameras, onOpen: onOpen)
            DeviceText(text: camera.hint)
        }
    }
}

private struct TelemetryStep: View {
    let telemetry: SetupTelemetry

    var body: some View {
        if let waiting = telemetry.waiting { DeviceNotice(notice: waiting) }
        EndpointRows(endpoints: telemetry.endpoints)
        DeviceText(text: telemetry.hint)
    }
}
