import Foundation

extension String {
    var isBlank: Bool { allSatisfy(\.isWhitespace) }

    func ifBlank(_ fallback: @autoclosure () -> String) -> String { isBlank ? fallback() : self }

    func ifEmpty(_ fallback: @autoclosure () -> String) -> String { isEmpty ? fallback() : self }

    func trimmingTrailing(_ characters: Character...) -> String {
        String(reversed().drop(while: characters.contains).reversed())
    }

    func trimmingLeading(_ characters: Character...) -> String {
        String(drop(while: characters.contains))
    }

    var trimmed: String { trimmingCharacters(in: .whitespacesAndNewlines) }

    func removingPrefix(_ prefix: String) -> String { hasPrefix(prefix) ? String(dropFirst(prefix.count)) : self }

    func removingSuffix(_ suffix: String) -> String { hasSuffix(suffix) ? String(dropLast(suffix.count)) : self }

    var capitalizedFirst: String { prefix(1).uppercased() + dropFirst() }

    var lowercasedFirst: String { prefix(1).lowercased() + dropFirst() }
}

private let properNouns: Set<String> = ["Android", "ArduPilot", "Pixhawk", "Herelink", "Trimble", "Septentrio", "Bing", "Esri", "Google", "Mapbox", "Tianditu", "VWorld", "OpenAIP", "QGroundControl"]

private let properPhrases = ["PX4 Pro", "3DR Solo", "Parrot Discovery", "Yuneec Mantis G", "Herelink AirUnit", "Herelink Hotspot"]

private func keepsCase(_ word: String) -> Bool {
    properNouns.contains(word) || word.dropFirst().contains(where: \.isUppercase) || !word.contains(where: \.isLowercase)
}

func sentenceCase(_ label: String) -> String {
    if let phrase = properPhrases.first(where: label.hasPrefix) {
        return phrase + sentenceCase(label.removingPrefix(phrase)).lowercasedFirst
    }
    return label.components(separatedBy: " ").enumerated().map { at, word in
        word.components(separatedBy: "-").enumerated().map { part, piece in
            let lead = String(piece.prefix(while: { !$0.isLetter && !$0.isNumber }))
            let body = String(piece.dropFirst(lead.count))
            return (at == 0 && part == 0) || keepsCase(body) ? piece : lead + body.lowercasedFirst
        }.joined(separator: "-")
    }.joined(separator: " ")
}
