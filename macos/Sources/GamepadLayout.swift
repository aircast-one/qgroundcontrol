enum GamepadLayout {
    static let axisScale: Float = 32767
    static let hatUp = 0x01
    static let hatRight = 0x02
    static let hatDown = 0x04
    static let hatLeft = 0x08
    static let hatThreshold: Float = 0.5
    static let axisCount = 6
    static let buttonCount = 13

    static func scaled(_ value: Float) -> Int {
        Int((min(max(value, -1), 1) * axisScale).rounded())
    }

    static func hatBits(x: Float, y: Float) -> Int {
        (y > hatThreshold ? hatUp : 0) | (y < -hatThreshold ? hatDown : 0)
            | (x < -hatThreshold ? hatLeft : 0) | (x > hatThreshold ? hatRight : 0)
    }
}
