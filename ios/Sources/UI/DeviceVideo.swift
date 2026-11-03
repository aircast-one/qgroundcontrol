import Foundation

struct RadioSettings: Equatable {
    let status: DeviceBadge
    let channel: String
    let region: String
    let txPower: String
    let details: String
}

enum SharingStage {
    static let Intro = "intro"
    static let Deploy = "deploy"
    static let Live = "live"
}

struct PublicSharing: Equatable {
    let status: DeviceBadge
    let stage: String
    let deployUrl: String
    let signKey: String
    let deviceToken: String
    let baseUrl: String
}

struct DevicePageLink: Equatable {
    let title: String
    let href: String
    let summary: String
}

struct VideoView: Equatable {
    let radio: RadioSettings?
    let sharing: PublicSharing
    let engineering: DevicePageLink
}

private func parseRadio(_ json: JSON?) -> RadioSettings? {
    json.map {
        RadioSettings(status: parseBadge($0["status"].objectOrNil), channel: $0["channel"].string, region: $0["region"].string, txPower: $0["txPower"].string, details: $0["details"].string)
    }
}

private func parseSharing(_ json: JSON) -> PublicSharing {
    PublicSharing(
        status: parseBadge(json["status"].objectOrNil),
        stage: json["stage"].string.ifEmpty(SharingStage.Intro),
        deployUrl: json["deployUrl"].string,
        signKey: json["signKey"].string,
        deviceToken: json["deviceToken"].string,
        baseUrl: json["baseUrl"].string
    )
}

private func parsePageLink(_ json: JSON?) -> DevicePageLink {
    DevicePageLink(title: json?["title"].string ?? "", href: json?["href"].string ?? "", summary: json?["summary"].string ?? "")
}

func parseVideo(_ json: JSON) -> VideoView? {
    guard let sharing = json["sharing"].objectOrNil else { return nil }
    return VideoView(radio: parseRadio(json["radio"].objectOrNil), sharing: parseSharing(sharing), engineering: parsePageLink(json["engineering"].objectOrNil))
}

let SHARING_OFF = ["baseUrl": "", "signKey": "", "deviceToken": ""]
