import Foundation

struct NetworkFact: Equatable {
    let label: String
    let value: String
}

struct DeviceProblem: Equatable {
    let text: String
    let detail: String
}

struct ConfirmedAction: Equatable {
    let label: String
    let confirm: String
}

struct UplinkBackup: Equatable {
    let label: String
    let status: DeviceBadge
}

struct UplinkCard: Equatable {
    let icon: String
    let title: String
    let status: DeviceBadge
    let verdict: DeviceBadge
    let detail: String
    let addr: String
    let meta: String
    let warning: String
    let rate: String
    let failover: [UplinkBackup]
    let switchText: String
    let backup: String
}

struct SmsMessage: Equatable {
    let from: String
    let text: String
    let time: String
}

struct SmsInbox: Equatable {
    let summary: String
    let items: [SmsMessage]
}

struct SimSettings: Equatable {
    let summary: String
    let apn: String
    let placeholder: String
    let metered: String
    let options: [DeviceOption]
}

struct ModemCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let notices: [DeviceBadge]
    let operatorName: String
    let detail: String
    let signal: DeviceChartView?
    let facts: [NetworkFact]
    let messages: SmsInbox
    let sim: SimSettings?
}

struct StarlinkDemo: Equatable {
    let on: Bool
    let note: String
    let condition: String
    let conditions: [DeviceOption]
    let first: Bool
}

struct StarlinkAddress: Equatable {
    let value: String
    let fallback: String
    let defaultFirst: Bool
    let open: Bool
}

struct StarlinkCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let demo: StarlinkDemo?
    let notices: [DeviceBadge]
    let pollError: String
    let facts: [NetworkFact]
    let firmware: String
    let address: StarlinkAddress
}

struct WifiCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let joined: DeviceBadge?
    let reopen: WifiReopen
}

struct IPv4Settings: Equatable, Hashable {
    let method: String
    let metric: String
    let metricPlaceholder: String
    let address: String
    let gateway: String
    let dns: String
}

struct LinkEditor: Equatable {
    let iface: String
    let ipv4: IPv4Settings
    let confirm: String
    let uuid: String
    let deactivateConfirm: String
    let deleteConfirm: String
}

struct NetLink: Equatable {
    let name: String
    let kind: String
    let role: DeviceBadge?
    let place: String
    let local: Bool
    let usage: String
    let editor: LinkEditor?
}

struct SavedProfile: Equatable {
    let uuid: String
    let name: String
    let kind: String
    let ipv4: IPv4Settings
    let activateConfirm: String
    let deleteConfirm: String
}

struct OtherUsage: Equatable {
    let label: String
    let month: String
    let total: String
}

struct NewConnection: Equatable {
    let types: [DeviceOption]
    let ports: [DeviceOption]
    let autoconnect: [DeviceOption]
}

struct InterfacesCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let problem: DeviceProblem?
    let links: [NetLink]
    let saved: [SavedProfile]
    let other: [OtherUsage]
    let methods: [DeviceOption]
    let add: NewConnection
}

struct BudgetMeter: Equatable {
    let label: String
    let used: String
    let percent: Int
    let level: Int
    let remaining: String
    let note: String
}

struct GaugeStat: Equatable {
    let label: String
    let value: String
    let note: String
}

struct DataGauge: Equatable {
    let badges: [DeviceBadge]
    let hero: String
    let heroCaption: String
    let uplinkPercent: Int
    let uplink: String
    let downlink: String
    let meters: [BudgetMeter]
    let stats: [GaugeStat]
}

struct BudgetSettings: Equatable, Hashable {
    let summary: String
    let monthlyCapGB: String
    let flightCapMB: String
    let warnPercent: String
}

struct DataCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let gauge: DataGauge?
    let budget: BudgetSettings
}

struct TailscaleControl: Equatable {
    let title: String
    let open: Bool
    let server: String
    let authKeyPlaceholder: String
    let authKeySet: Bool
    let switchError: String
    let pending: String
    let switchTo: ConfirmedAction?
    let logOutToSwitch: ConfirmedAction?
    let hint: String
}

struct TailscaleBanner: Equatable {
    let title: String
    let subtitle: String
    let status: DeviceBadge
}

struct TailscaleSignIn: Equatable {
    let label: String
    let href: String
}

struct TailscaleCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let problem: DeviceProblem?
    let banner: TailscaleBanner?
    let facts: [NetworkFact]
    let addresses: [String]
    let signIn: TailscaleSignIn?
    let connect: String
    let cancel: Bool
    let logOut: String
    let hint: String
    let control: TailscaleControl?
}

struct IceServer: Equatable, Hashable {
    var url: String = ""
    var username: String = ""
    var password: String = ""
}

struct RelayCard: Equatable {
    let status: DeviceBadge
    let summary: String
    let on: Bool
    let configured: Bool
    let reason: String
    let notice: DeviceBadge
    let servers: [IceServer]
}

struct NetworkView: Equatable {
    let uplink: UplinkCard
    let modem: ModemCard?
    let starlink: StarlinkCard
    let wifi: WifiCard
    let interfaces: InterfacesCard
    let data: DataCard
    let tailscale: TailscaleCard
    let relay: RelayCard
}

private func netBadge(_ json: JSON, _ key: String) -> DeviceBadge { parseBadge(json[key].objectOrNil) }

private func netFacts(_ json: JSON) -> [NetworkFact] { json["facts"].objects.map { NetworkFact(label: $0["label"].string, value: $0["value"].string) } }

private func netProblem(_ json: JSON) -> DeviceProblem? {
    json["problem"].objectOrNil.map { DeviceProblem(text: $0["text"].string, detail: $0["detail"].string) }
}

private func netConfirmed(_ json: JSON?) -> ConfirmedAction? {
    json.map { ConfirmedAction(label: $0["label"].string, confirm: $0["confirm"].string) }
}

private func netIPv4(_ json: JSON) -> IPv4Settings {
    IPv4Settings(
        method: json["method"].string,
        metric: json["metric"].string,
        metricPlaceholder: json["metricPlaceholder"].string,
        address: json["address"].string,
        gateway: json["gateway"].string,
        dns: json["dns"].string
    )
}

private func netUplink(_ json: JSON) -> UplinkCard {
    UplinkCard(
        icon: json["icon"].string,
        title: json["title"].string,
        status: netBadge(json, "status"),
        verdict: netBadge(json, "verdict"),
        detail: json["detail"].string,
        addr: json["addr"].string,
        meta: json["meta"].string,
        warning: json["warning"].string,
        rate: json["rate"].string,
        failover: json["failover"].objects.map { UplinkBackup(label: $0["label"].string, status: netBadge($0, "status")) },
        switchText: json["switch"].string,
        backup: json["backup"].string
    )
}

private func netSim(_ json: JSON?) -> SimSettings? {
    json.map {
        SimSettings(summary: $0["summary"].string, apn: $0["apn"].string, placeholder: $0["placeholder"].string, metered: $0["metered"].string, options: parseOptions($0, "options"))
    }
}

private func netModem(_ json: JSON?) -> ModemCard? {
    guard let json else { return nil }
    let messages = json["messages"]
    return ModemCard(
        status: netBadge(json, "status"),
        summary: json["summary"].string,
        notices: parseBadges(json, "notices"),
        operatorName: json["operator"].string,
        detail: json["detail"].string,
        signal: parseChart(json["signal"].objectOrNil),
        facts: netFacts(json),
        messages: SmsInbox(
            summary: messages["summary"].string,
            items: messages["items"].objects.map { SmsMessage(from: $0["from"].string, text: $0["text"].string, time: $0["when"].string) }
        ),
        sim: netSim(json["sim"].objectOrNil)
    )
}

private func netDemo(_ json: JSON?) -> StarlinkDemo? {
    json.map {
        StarlinkDemo(on: $0["on"].bool, note: $0["note"].string, condition: $0["condition"].string, conditions: parseOptions($0, "conditions"), first: $0["first"].bool)
    }
}

private func netStarlink(_ json: JSON) -> StarlinkCard {
    let address = json["address"]
    return StarlinkCard(
        status: netBadge(json, "status"),
        summary: json["summary"].string,
        demo: netDemo(json["demo"].objectOrNil),
        notices: parseBadges(json, "notices"),
        pollError: json["pollError"].string,
        facts: netFacts(json),
        firmware: json["firmware"].string,
        address: StarlinkAddress(
            value: address["value"].string,
            fallback: address["default"].string,
            defaultFirst: address["defaultFirst"].bool,
            open: address["open"].bool
        )
    )
}

private func netWifi(_ json: JSON) -> WifiCard {
    let reopen = json["reopen"]
    return WifiCard(
        status: netBadge(json, "status"),
        summary: json["summary"].string,
        joined: optDeviceBadge(json, "joined"),
        reopen: WifiReopen(host: reopen["host"].string, hostname: reopen["hostname"].string)
    )
}

private func netEditor(_ json: JSON?) -> LinkEditor? {
    json.map {
        LinkEditor(
            iface: $0["interface"].string,
            ipv4: netIPv4($0["ipv4"]),
            confirm: $0["confirm"].string,
            uuid: $0["uuid"].string,
            deactivateConfirm: $0["deactivateConfirm"].string,
            deleteConfirm: $0["deleteConfirm"].string
        )
    }
}

private func netInterfaces(_ json: JSON) -> InterfacesCard {
    let add = json["add"]
    return InterfacesCard(
        status: netBadge(json, "status"),
        summary: json["summary"].string,
        problem: netProblem(json),
        links: json["links"].objects.map {
            NetLink(
                name: $0["name"].string,
                kind: $0["kind"].string,
                role: optDeviceBadge($0, "role"),
                place: $0["where"].string,
                local: $0["local"].bool,
                usage: $0["usage"].string,
                editor: netEditor($0["editor"].objectOrNil)
            )
        },
        saved: json["saved"].objects.map {
            SavedProfile(
                uuid: $0["uuid"].string,
                name: $0["name"].string,
                kind: $0["kind"].string,
                ipv4: netIPv4($0["ipv4"]),
                activateConfirm: $0["activateConfirm"].string,
                deleteConfirm: $0["deleteConfirm"].string
            )
        },
        other: json["other"].objects.map { OtherUsage(label: $0["label"].string, month: $0["month"].string, total: $0["total"].string) },
        methods: parseOptions(json, "methods"),
        add: NewConnection(types: parseOptions(add, "types"), ports: parseOptions(add, "ports"), autoconnect: parseOptions(add, "autoconnect"))
    )
}

private func netGauge(_ json: JSON?) -> DataGauge? {
    guard let json else { return nil }
    return DataGauge(
        badges: parseBadges(json, "badges"),
        hero: json["hero"].string,
        heroCaption: json["heroCaption"].string,
        uplinkPercent: json["uplinkPercent"].int(0),
        uplink: json["uplink"].string,
        downlink: json["downlink"].string,
        meters: json["meters"].objects.map {
            BudgetMeter(
                label: $0["label"].string,
                used: $0["used"].string,
                percent: $0["percent"].int(0),
                level: $0["level"].int(0),
                remaining: $0["remaining"].string,
                note: $0["note"].string
            )
        },
        stats: json["stats"].objects.map { GaugeStat(label: $0["label"].string, value: $0["value"].string, note: $0["note"].string) }
    )
}

private func netData(_ json: JSON) -> DataCard {
    let budget = json["budget"]
    return DataCard(
        status: netBadge(json, "status"),
        summary: json["summary"].string,
        gauge: netGauge(json["gauge"].objectOrNil),
        budget: BudgetSettings(
            summary: budget["summary"].string,
            monthlyCapGB: budget["monthlyCapGB"].string,
            flightCapMB: budget["flightCapMB"].string,
            warnPercent: budget["warnPercent"].string
        )
    )
}

private func netControl(_ json: JSON?) -> TailscaleControl? {
    guard let json else { return nil }
    return TailscaleControl(
        title: json["title"].string,
        open: json["open"].bool,
        server: json["server"].string,
        authKeyPlaceholder: json["authKeyPlaceholder"].string,
        authKeySet: json["authKeySet"].bool,
        switchError: json["switchError"].string,
        pending: json["pending"].string,
        switchTo: netConfirmed(json["switch"].objectOrNil),
        logOutToSwitch: netConfirmed(json["logOutToSwitch"].objectOrNil),
        hint: json["hint"].string
    )
}

func parseTailscale(_ json: JSON?) -> TailscaleCard {
    let card = json ?? JSON.object([:])
    return TailscaleCard(
        status: netBadge(card, "status"),
        summary: card["summary"].string,
        problem: netProblem(card),
        banner: card["banner"].objectOrNil.map { TailscaleBanner(title: $0["title"].string, subtitle: $0["subtitle"].string, status: netBadge($0, "status")) },
        facts: netFacts(card),
        addresses: card["addresses"].strings,
        signIn: card["signIn"].objectOrNil.map { TailscaleSignIn(label: $0["label"].string, href: $0["href"].string) },
        connect: card["connect"].string,
        cancel: card["cancel"].bool,
        logOut: card["logOut"].string,
        hint: card["hint"].string,
        control: netControl(card["control"].objectOrNil)
    )
}

private func netRelay(_ json: JSON) -> RelayCard {
    RelayCard(
        status: netBadge(json, "status"),
        summary: json["summary"].string,
        on: json["on"].bool,
        configured: json["configured"].bool,
        reason: json["reason"].string,
        notice: netBadge(json, "notice"),
        servers: json["servers"].objects.map { IceServer(url: $0["url"].string, username: $0["username"].string, password: $0["password"].string) }
    )
}

func parseNetwork(_ json: JSON) -> NetworkView? {
    guard let uplink = json["uplink"].objectOrNil else { return nil }
    return NetworkView(
        uplink: netUplink(uplink),
        modem: netModem(json["modem"].objectOrNil),
        starlink: netStarlink(json["starlink"]),
        wifi: netWifi(json["wifi"]),
        interfaces: netInterfaces(json["interfaces"]),
        data: netData(json["data"]),
        tailscale: parseTailscale(json["tailscale"].objectOrNil),
        relay: netRelay(json["relay"])
    )
}

private let UPLINK_ICONS: [String: Icon] = [
    "satellite-dish": .satelliteAlt,
    "signal": .signalCellularAlt,
    "wifi": .wifi,
    "ethernet-port": .usb,
]

func uplinkIcon(_ icon: String) -> Icon { UPLINK_ICONS[icon] ?? .lan }

let GOOGLE_STUN = "stun:stun.l.google.com:19302"

func iceRows(_ servers: [IceServer]) -> [IceServer] { servers.isEmpty ? [IceServer()] : servers }

let ICE_SERVERS = "deviceNetwork.iceServers"

func parseIceServerLines(_ answer: JSON?) -> String? {
    guard let answer, answer["ok"].bool else { return nil }
    return answer["result"].string
}

private func iceServersJson(_ rows: [IceServer]) -> JSON {
    .array(rows.map { .object(["url": .string($0.url), "username": .string($0.username), "password": .string($0.password)]) })
}

func iceServerLines(_ rows: [IceServer]) async -> String? {
    let sent = iceServersJson(rows)
    let answer: JSON? = await offMain { Qgc.call(ICE_SERVERS, sent) }
    return parseIceServerLines(answer)
}

func manualAddressing(_ method: String) -> Bool { method == "manual" }

private let CONNECTION_FIELD_TYPES = ["ssid": "wifi", "psk": "wifi", "apn": "gsm", "pin": "gsm"]

func connectionFieldShown(_ type: String, _ field: String) -> Bool { CONNECTION_FIELD_TYPES[field].map { $0 == type } ?? true }
