import Foundation

struct DeviceSetupStep: Equatable {
    let key: String
    let label: String
    let title: String
    let lead: String
    let status: DeviceBadge?
}

struct SetupCheckRow: Equatable {
    let label: String
    let status: DeviceBadge
    let step: Int
}

struct SetupInternet: Equatable {
    let online: DeviceBadge
    let joinOpen: Bool
    let reopen: WifiReopen
    let connection: DevicePage
    let hint: String
}

struct SetupCamera: Equatable {
    let problem: String
    let wizard: CameraWizardView
    let watch: String
    let cameras: DevicePage
    let hint: String
}

struct SetupTelemetry: Equatable {
    let waiting: DeviceBadge?
    let endpoints: [DeviceEndpoint]
    let hint: String
}

struct DeviceSetupView: Equatable {
    let start: Int
    let steps: [DeviceSetupStep]
    let internet: SetupInternet
    let remote: TailscaleCard
    let camera: SetupCamera
    let telemetry: SetupTelemetry
    let checks: [SetupCheckRow]
}

enum SetupNext: Equatable { case Next, Skip }

struct SetupNav: Equatable {
    let back: Bool
    let later: Bool
    let next: SetupNext?
    let finish: Bool
}

private func parseInternet(_ json: JSON?) -> SetupInternet {
    let reopen = json?["reopen"]
    return SetupInternet(
        online: parseBadge(json?["online"].objectOrNil),
        joinOpen: json?["joinOpen"].bool ?? false,
        reopen: WifiReopen(host: reopen?["host"].string ?? "", hostname: reopen?["hostname"].string ?? ""),
        connection: parseDevicePage(json?["connection"].objectOrNil),
        hint: json?["hint"].string ?? ""
    )
}

func parseDeviceSetup(_ json: JSON?) -> DeviceSetupView? {
    guard let json, let camera = json["camera"].objectOrNil, let wizard = parseCameraWizard(camera["wizard"].objectOrNil) else { return nil }
    let telemetry = json["telemetry"].objectOrNil
    let steps = json["steps"].objects.map {
        DeviceSetupStep(key: $0["key"].string, label: $0["label"].string, title: $0["title"].string, lead: $0["lead"].string, status: optDeviceBadge($0, "status"))
    }
    guard !steps.isEmpty else { return nil }
    return DeviceSetupView(
        start: clampStep(json["start"].int(1), steps.count),
        steps: steps,
        internet: parseInternet(json["internet"].objectOrNil),
        remote: parseTailscale(json["remote"].objectOrNil),
        camera: SetupCamera(
            problem: camera["problem"].string,
            wizard: wizard,
            watch: camera["watch"].string,
            cameras: parseDevicePage(camera["cameras"].objectOrNil),
            hint: camera["hint"].string
        ),
        telemetry: SetupTelemetry(waiting: optDeviceBadge(telemetry, "waiting"), endpoints: parseEndpointList(telemetry, "endpoints"), hint: telemetry?["hint"].string ?? ""),
        checks: json["checks"].objects.map { SetupCheckRow(label: $0["label"].string, status: parseBadge($0["status"].objectOrNil), step: $0["step"].int(1)) }
    )
}

func clampStep(_ step: Int, _ total: Int) -> Int { min(max(step, 1), total) }

let CAMERA_APPLYING = 3

func cameraHoldsTheWizard(_ key: String, _ wizardShown: Bool, _ wizardStep: Int) -> Bool {
    key == "camera" && wizardShown && wizardStep != CAMERA_APPLYING
}

func setupNav(_ step: Int, _ total: Int, _ held: Bool) -> SetupNav {
    SetupNav(back: step > 1 && !held, later: step < total, next: step >= total ? nil : held ? SetupNext.Skip : SetupNext.Next, finish: step == total)
}
