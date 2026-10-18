import SwiftUI

let AIRCAST_CLOUD_NAME = "Aircast cloud"
private let ACCOUNT = "account"
private let ACCOUNT_POLL_MS = 1000
private let API_BASE = "^https?://[^/]+"

struct CloudDrone: Equatable {
    var id: String
    var name: String
    var seen: String
}

struct AccountState: Equatable {
    var signedIn: Bool
    var signingIn: Bool
    var status: String
    var userCode: String
    var verificationUrl: String
    var apiBase = ""
    var drones: [CloudDrone] = []
    var dronesStatus = ""
}

func accountState(_ view: JSON?) -> AccountState? {
    guard let view, view["kind"].string == "object" else { return nil }
    return AccountState(
        signedIn: view["signedIn"].bool,
        signingIn: view["signingIn"].bool,
        status: view["status"].string,
        userCode: view["userCode"].string,
        verificationUrl: view["verificationUrl"].string,
        apiBase: view["apiBase"].string,
        drones: view["drones"].array.map { CloudDrone(id: $0["id"].string, name: $0["name"].string, seen: $0["seen"].string) },
        dronesStatus: view["dronesStatus"].string
    )
}

func adoptedApiBase(_ field: String, _ account: AccountState?) -> String? {
    guard let account, account.signedIn, !account.apiBase.isBlank, account.apiBase != field, field.isBlank else { return nil }
    return account.apiBase
}

func accountLine(_ state: AccountState) -> String { state.signedIn ? "Signed in" : state.status.ifBlank("Not signed in") }

func cloudApiBaseValid(_ apiBase: String) -> Bool { apiBase.trimmed.range(of: API_BASE, options: .regularExpression) != nil }

func cloudDeviceValid(_ deviceId: String) -> Bool { !deviceId.isBlank }

struct AircastCloudFields: View {
    let apiBase: String
    let deviceId: String
    let showErrors: Bool
    let onApiBase: (String) -> Void
    let onDeviceId: (String) -> Void
    @State private var account: AccountState?
    @State private var opened = ""
    @Environment(\.theme) private var theme
    @Environment(\.openURL) private var openURL

    private var pending: AccountState? { account.flatMap { $0.signingIn && !$0.verificationUrl.isBlank ? $0 : nil } }
    private var drones: [CloudDrone] { account.map { $0.signedIn ? $0.drones : [] } ?? [] }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            CloudField(label: "Account server", placeholder: "https://api.aircast.one", value: apiBase, onChange: onApiBase)
                .keyboardType(.URL)
            if showErrors && !cloudApiBaseValid(apiBase) {
                Text("Enter the account server address, starting with https://").font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            if !drones.isEmpty {
                Text("Your drones").font(.titleSmall)
                ForEach(drones, id: \.id) { drone in
                    Button { onDeviceId(drone.id) } label: {
                        HStack {
                            RadioIndicator(selected: deviceId.trimmed == drone.id)
                            VStack(alignment: .leading) {
                                Text(drone.name).foregroundStyle(theme.colors.onSurface)
                                Text(drone.seen).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                            }
                        }
                        .frame(minHeight: 48)
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(deviceId.trimmed == drone.id ? .isSelected : [])
                }
            }
            if let state = account, state.signedIn, !state.dronesStatus.isBlank {
                Text(state.dronesStatus).font(.bodySmall)
            }
            CloudField(label: "Device", placeholder: "", value: deviceId, onChange: onDeviceId)
            if showErrors && !cloudDeviceValid(deviceId) {
                Text("Set up from the device to fill this in").font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            if let state = account {
                HStack {
                    Text("Account  \(accountLine(state))").frame(maxWidth: .infinity, alignment: .leading)
                    if state.signedIn {
                        Button("Sign out") { offMain { AccountCommands.signOut() } }.buttonStyle(.bordered)
                    } else if state.signingIn {
                        Button("Cancel") { offMain { AccountCommands.cancelSignIn() } }.buttonStyle(.bordered)
                    } else {
                        Button("Sign in") {
                            let base = apiBase
                            offMain {
                                AccountCommands.setApiBase(base)
                                AccountCommands.signIn()
                            }
                        }
                        .buttonStyle(.bordered)
                    }
                }
                if state.signingIn {
                    Text("Your browser opened \(state.verificationUrl) — approve code \(state.userCode) there.").font(.bodySmall)
                }
            }
        }
        .task {
            while !Task.isCancelled {
                account = await offMain { accountState(Qgc.get(ACCOUNT)) }
                try? await Task.sleep(for: .milliseconds(ACCOUNT_POLL_MS))
            }
        }
        .onChange(of: adoptedApiBase(apiBase, account)) { _, adopted in
            if let adopted { onApiBase(adopted) }
        }
        .onChange(of: pending?.verificationUrl) { _, url in
            guard let url, url != opened else { return }
            opened = url
            if let target = URL(string: url) { openURL(target) }
        }
    }
}

private struct CloudField: View {
    let label: String
    let placeholder: String
    let value: String
    let onChange: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            TextField(placeholder, text: Binding(get: { value }, set: onChange))
                .textFieldStyle(.roundedBorder)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
        }
    }
}
