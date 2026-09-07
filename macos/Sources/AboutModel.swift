import Foundation

struct HelpLink: Identifiable, Equatable {
    let name: String
    let url: String

    var id: String { url }

    var host: String {
        let stripped = url
            .replacingOccurrences(of: "https://", with: "")
            .replacingOccurrences(of: "http://", with: "")
        return stripped.split(separator: "/").first.map(String.init) ?? stripped
    }

    static let all: [HelpLink] = [
        HelpLink(name: "QGroundControl User Guide", url: "https://docs.qgroundcontrol.com"),
        HelpLink(name: "PX4 Users Discussion Forum", url: "http://discuss.px4.io/c/qgroundcontrol"),
        HelpLink(name: "ArduPilot Users Discussion Forum",
                 url: "https://discuss.ardupilot.org/c/ground-control-software/qgroundcontrol"),
        HelpLink(name: "QGroundControl Discord Channel",
                 url: "https://discord.com/channels/1022170275984457759/1022185820683255908"),
    ]
}

struct AboutInfo: Equatable {
    let name: String
    let shortVersion: String
    let build: String

    static func version(short: String, build: String) -> String {
        if short.isEmpty && build.isEmpty { return "unknown" }
        if build.isEmpty || build == short { return short }
        if short.isEmpty { return build }
        return "\(short) (\(build))"
    }

    var versionText: String { AboutInfo.version(short: shortVersion, build: build) }

    static func read(_ info: [String: Any]?) -> AboutInfo {
        AboutInfo(
            name: (info?["CFBundleName"] as? String) ?? "Aircast QGC",
            shortVersion: (info?["CFBundleShortVersionString"] as? String) ?? "",
            build: (info?["CFBundleVersion"] as? String) ?? "")
    }
}
