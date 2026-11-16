import SwiftUI

struct ControlMetrics: Equatable {
    let rowLarge: CGFloat
    let rowTall: CGFloat
    let row: CGFloat
    let rowCompact: CGFloat
    let rowDense: CGFloat
    let floatingButton: CGFloat
    let primaryButton: CGFloat
    let iconButton: CGFloat
    let iconButtonSmall: CGFloat
    let avatar: CGFloat
    let stepper: CGFloat
    let minTarget: CGFloat

    static let touch = ControlMetrics(
        rowLarge: 72, rowTall: 64, row: 56, rowCompact: 48, rowDense: 40,
        floatingButton: 56, primaryButton: 52, iconButton: 48, iconButtonSmall: 40, avatar: 40, stepper: 48, minTarget: 48
    )
}

enum Metrics {
    static let current = ControlMetrics.touch

    static var rowLarge: CGFloat { current.rowLarge }
    static var rowTall: CGFloat { current.rowTall }
    static var row: CGFloat { current.row }
    static var rowCompact: CGFloat { current.rowCompact }
    static var rowDense: CGFloat { current.rowDense }
    static var floatingButton: CGFloat { current.floatingButton }
    static var primaryButton: CGFloat { current.primaryButton }
    static var iconButton: CGFloat { current.iconButton }
    static var iconButtonSmall: CGFloat { current.iconButtonSmall }
    static var avatar: CGFloat { current.avatar }
    static var stepper: CGFloat { current.stepper }
    static var minTarget: CGFloat { current.minTarget }
}
