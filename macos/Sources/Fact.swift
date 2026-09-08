import Foundation

enum Fact {
    // Split on lower->upper only. Splitting every capital turns "RemoteID" into
    // "Remote I D" and "ADSBVehicleManager" into "Adsb Vehicle Manager", which
    // destroys the scent an operator scans the sidebar for.
    static func humanise(_ identifier: String) -> String {
        let characters = Array(identifier)
        var words: [String] = []
        var current = ""

        for (index, character) in characters.enumerated() {
            let previous = index > 0 ? characters[index - 1] : nil
            let next = index + 1 < characters.count ? characters[index + 1] : nil
            let startsWord: Bool
            if let previous {
                if character.isUppercase, previous.isLowercase {
                    // not previous.isNumber: "3D" and "H264" are one word, not two
                    startsWord = true
                } else if character.isUppercase, previous.isUppercase, let next, next.isLowercase {
                    // trailing capital of a run begins the next word: "ADSBVehicle" -> ADSB | Vehicle
                    startsWord = true
                } else if character.isNumber, previous.isLetter, !previous.isUppercase {
                    startsWord = true
                } else {
                    startsWord = false
                }
            } else {
                startsWord = false
            }

            if startsWord, !current.isEmpty {
                words.append(current)
                current = ""
            }
            current.append(character)
        }
        if !current.isEmpty { words.append(current) }

        return words
            .map { word -> String in
                let upper = word.uppercased()
                if acronyms.contains(upper) { return upper }
                return word.first!.isUppercase ? word : word.prefix(1).uppercased() + word.dropFirst()
            }
            .joined(separator: " ")
    }

    // Identifiers that start lowercase lose their acronym casing entirely
    // ("rtkSettings" -> "Rtk"), which no casing rule can recover.
    private static let acronyms: Set<String> = [
        "ADSB", "AGL", "AMSL", "APM", "ESC", "GCS", "GPS", "ID", "IMU", "NMEA",
        "PX4", "RC", "RTK", "RTSP", "TCP", "UDP", "UVC", "VTOL", "UTM", "SBS", "3D",
    ]
}
