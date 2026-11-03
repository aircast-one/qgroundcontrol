import Foundation

struct PortOption: Equatable {
    let value: String
    let label: String
    let address: String
}

struct PiCamera: Equatable {
    let note: String
    let action: String
}

struct CameraWizardView: Equatable {
    let name: String
    let title: String
    let isNew: Bool
    let suggestedName: String
    let rtsp: String
    let rtspHint: String
    let addressOpen: Bool
    let lookingFor: String
    let checks: [String]
    let piCamera: PiCamera?
    let iface: String
    let interfaces: [PortOption]
    let transport: String
    let transports: [DeviceOption]
    let coverage: String
    let json: JSON
}

struct WizardCamera: Equatable, Hashable {
    let host: String
    let url: String
    let needsAuth: Bool
    let pathFound: Bool
    let used: String
    let note: String
    let action: String
}

struct WizardChip: Equatable {
    let cidr: String
    let text: String
}

struct WizardPort: Equatable {
    let iface: String
    let label: String
    let detail: String
    let open: Bool
    let suggest: String
    let hint: String
    let go: String
    let options: [WizardChip]
    let busy: Bool
    let status: String
}

struct WizardMake: Equatable {
    let id: String
    let label: String
    let sub: String
    let action: String
}

struct WizardFound: Equatable {
    let n: Int
    let refusal: String
    let camera: WizardCamera?
    let name: String
}

struct WizardApply: Equatable {
    let n: Int
    let state: String
    let text: String
    let field: String
    let url: String
    let camera: String
}

struct WizardNotice: Equatable {
    let n: Int
    let text: String
}

struct DeviceWizard: Equatable {
    let session: Int64
    let scanning: Bool
    let scans: Int
    let again: Int
    let adopt: WizardCamera?
    let note: String
    let checklist: String
    let cams: [WizardCamera]
    let camHeading: String
    let camsHint: String
    let details: String
    let todoHeading: String
    let todo: [WizardPort]
    let rest: [WizardPort]
    let restTitle: String
    let templates: [WizardMake]
    let templateBusy: Bool
    let templateNote: String
    let template: WizardFound
    let apply: WizardApply
    let notice: WizardNotice
}

struct WizardRefusal: Equatable {
    let text: String
    let warn: Bool
    let field: String?
}

struct WizardNext: Equatable {
    let kind: String
    let rtsp: String
    let name: String?
    let camera: String
    let refusal: WizardRefusal?
    let dupAck: String
    let iface: String
    let ip: String
}

struct WizardForm: Equatable {
    let next: String
    let skipsPort: Bool
    let applyNote: String
}

let PI_CAMERA = "rpiCamera"
let DEVICE_WIZARD = "view.deviceWizard"
private let WIZARD_OPEN = "deviceWizard.open"
private let WIZARD_SCAN = "deviceWizard.scan"
private let WIZARD_FIX_PORT = "deviceWizard.fixPort"
private let WIZARD_TEMPLATE = "deviceWizard.template"
private let WIZARD_NEXT = "deviceWizard.next"
private let WIZARD_FORM = "deviceWizard.form"
private let WIZARD_SIGN_IN = "deviceWizard.signIn"
private let WIZARD_APPLY = "deviceWizard.apply"
private let WIZARD_RESTART = "deviceWizard.restart"

let STEP_NAMES = ["Camera", "Port", "Picture"]

let NO_FORM = WizardForm(next: "Continue", skipsPort: false, applyNote: "")

private func textOrNull(_ json: JSON) -> String? { json.isNull ? nil : json.string }

func parseCameraWizard(_ json: JSON?) -> CameraWizardView? {
    guard let json, json.object != nil else { return nil }
    return CameraWizardView(
        name: json["name"].string,
        title: json["title"].string,
        isNew: json["new"].bool,
        suggestedName: json["suggestedName"].string,
        rtsp: json["rtsp"].string,
        rtspHint: json["rtspHint"].string,
        addressOpen: json["addressOpen"].bool,
        lookingFor: json["lookingFor"].string,
        checks: json["checks"].array.map(\.string),
        piCamera: json["piCamera"].objectOrNil.map { PiCamera(note: $0["note"].string, action: $0["action"].string) },
        iface: json["interface"].string,
        interfaces: json["interfaces"].objects.map { PortOption(value: $0["value"].string, label: $0["label"].string, address: $0["address"].string) },
        transport: json["transport"].string,
        transports: parseOptions(json, "transports"),
        coverage: json["coverage"].string,
        json: json
    )
}

private func parseWizardCamera(_ json: JSON?) -> WizardCamera? {
    guard let json, json.object != nil else { return nil }
    return WizardCamera(
        host: json["host"].string,
        url: json["url"].string,
        needsAuth: json["needsAuth"].bool,
        pathFound: json["pathFound"].bool,
        used: json["used"].string,
        note: json["note"].string,
        action: json["action"].string
    )
}

private func parseWizardPort(_ json: JSON) -> WizardPort {
    WizardPort(
        iface: json["iface"].string,
        label: json["label"].string,
        detail: json["detail"].string,
        open: json["open"].bool,
        suggest: json["suggest"].string,
        hint: json["hint"].string,
        go: json["go"].string,
        options: json["options"].objects.map { WizardChip(cidr: $0["cidr"].string, text: $0["text"].string) },
        busy: json["busy"].bool,
        status: json["status"].string
    )
}

func parseDeviceWizard(_ json: JSON?) -> DeviceWizard {
    let json = json ?? .null
    let template = json["template"]
    let apply = json["apply"]
    let notice = json["notice"]
    return DeviceWizard(
        session: json["session"].int64 ?? -1,
        scanning: json["scanning"].bool,
        scans: json["scans"].int(0),
        again: json["again"].int(0),
        adopt: parseWizardCamera(json["adopt"].objectOrNil),
        note: json["note"].string,
        checklist: json["checklist"].string,
        cams: json["cams"].objects.compactMap { parseWizardCamera($0) },
        camHeading: json["camHeading"].string,
        camsHint: json["camsHint"].string,
        details: json["details"].string,
        todoHeading: json["todoHeading"].string,
        todo: json["todo"].objects.map { parseWizardPort($0) },
        rest: json["rest"].objects.map { parseWizardPort($0) },
        restTitle: json["restTitle"].string,
        templates: json["templates"].objects.map { WizardMake(id: $0["id"].string, label: $0["label"].string, sub: $0["sub"].string, action: $0["action"].string) },
        templateBusy: json["templateBusy"].bool,
        templateNote: json["templateNote"].string,
        template: WizardFound(n: template["n"].int(0), refusal: template["refusal"].string, camera: parseWizardCamera(template["camera"].objectOrNil), name: template["name"].string),
        apply: WizardApply(
            n: apply["n"].int(0),
            state: apply["state"].string,
            text: apply["text"].string,
            field: apply["field"].string,
            url: apply["url"].string,
            camera: apply["camera"].string
        ),
        notice: WizardNotice(n: notice["n"].int(0), text: notice["text"].string)
    )
}

func parseWizardNext(_ json: JSON?) -> WizardNext? {
    guard let json, json.object != nil else { return nil }
    return WizardNext(
        kind: json["kind"].string,
        rtsp: json["rtsp"].string,
        name: textOrNull(json["name"]),
        camera: json["camera"].string,
        refusal: json["refusal"].objectOrNil.map { WizardRefusal(text: $0["text"].string, warn: $0["warn"].bool, field: textOrNull($0["field"])) },
        dupAck: json["dupAck"].string,
        iface: json["iface"].string,
        ip: json["ip"].string
    )
}

func parseWizardForm(_ json: JSON?) -> WizardForm {
    guard let json, json.object != nil else { return NO_FORM }
    return WizardForm(next: json["next"].string, skipsPort: json["skipsPort"].bool, applyNote: json["applyNote"].string)
}

func continueLeads(_ rtsp: String) -> Bool { !rtsp.trimmed.isEmpty }

func repairing(_ view: CameraWizardView) -> Bool { !view.isNew && !view.lookingFor.isEmpty }

func nextOffered(_ view: CameraWizardView, _ rtsp: String, picked: Bool) -> Bool { !repairing(view) || picked || !rtsp.trimmed.isEmpty }

func scanLeads(_ rtsp: String, _ picked: Bool) -> Bool { rtsp.trimmed.isEmpty && !picked }

private func wizardCall(_ path: String, _ args: [JSON]) async -> JSON? {
    await offMain { Qgc.call(path, arguments: args.map { $0 as Any? }) }
}

private func wizardResult(_ path: String, _ args: [JSON]) async -> JSON? {
    await wizardCall(path, args)?["result"].objectOrNil
}

func openWizard(_ view: CameraWizardView, _ scan: Bool) async -> Int64 {
    await wizardResult(WIZARD_OPEN, [view.json, .bool(scan)])?["session"].int64 ?? -1
}

func scanForCameras() async {
    _ = await wizardCall(WIZARD_SCAN, [])
}

func fixWizardPort(_ iface: String, _ address: String) async {
    _ = await wizardCall(WIZARD_FIX_PORT, [.string(iface), .string(address)])
}

func setUpCameraMake(_ id: String) async {
    _ = await wizardCall(WIZARD_TEMPLATE, [.string(id)])
}

func wizardNext(_ rtsp: String, _ name: String, _ dupAck: String) async -> WizardNext? {
    parseWizardNext(await wizardResult(WIZARD_NEXT, [.string(rtsp), .string(name), .string(dupAck)]))
}

func wizardForm(_ rtsp: String, _ iface: String, _ ip: String, _ current: String) async -> WizardForm {
    parseWizardForm(await wizardResult(WIZARD_FORM, [.string(rtsp), .string(iface), .string(ip), .string(current)]))
}

func signedInUrl(_ base: String, _ user: String, _ pass: String) async -> String {
    await wizardCall(WIZARD_SIGN_IN, [.string(base), .string(user), .string(pass)])?["result"].stringOrNil ?? base
}

func applyCamera(_ rtsp: String, _ camera: String, _ iface: String, _ ip: String, _ transport: String) async {
    _ = await wizardCall(WIZARD_APPLY, [.string(rtsp), .string(camera), .string(iface), .string(ip), .string(transport)])
}

func restartWizard() async {
    _ = await wizardCall(WIZARD_RESTART, [])
}
