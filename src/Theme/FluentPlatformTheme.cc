/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "FluentPlatformTheme.h"

PlatformTheme::Tones FluentPlatformTheme::tones(Appearance appearance) const
{
    if (appearance == LightAppearance) {
        return Tones {
            .background      = QColor("#f3f3f3"),
            .surface         = QColor("#ffffff"),
            .surfaceSunken   = QColor("#ededed"),
            .surfaceRaised   = QColor("#f9f9f9"),
            .scrim           = QColor("#e6f3f3f3"),
            .glass           = QColor("#d9f3f3f3"),
            .outline         = QColor("#d1d1d1"),
            .outlineWeak     = QColor("#e5e5e5"),
            .fill            = QColor("#e0e0e0"),
            .ink             = QColor("#1b1b1b"),
            .inkMuted        = QColor("#5d5d5d"),
            .accent          = QColor("#0067c0"),
            .accentMuted     = QColor("#bfbfbf"),
            .accentContainer = QColor("#dcecfb"),
            .onAccent        = QColor("#ffffff"),
            .green           = QColor("#0f7b0f"),
            .yellowGreen     = QColor("#6a8a1f"),
            .yellow          = QColor("#9d5d00"),
            .orange          = QColor("#c4500a"),
            .red             = QColor("#c42b1c"),
            .grey            = QColor("#8a8a8a"),
        };
    }

    return Tones {
        .background      = QColor("#202020"),
        .surface         = QColor("#2b2b2b"),
        .surfaceSunken   = QColor("#1c1c1c"),
        .surfaceRaised   = QColor("#323232"),
        .scrim           = QColor("#d9202020"),
        .glass           = QColor("#73181818"),
        .outline         = QColor("#414141"),
        .outlineWeak     = QColor("#303030"),
        .fill            = QColor("#4a4a4a"),
        .ink             = QColor("#ffffff"),
        .inkMuted        = QColor("#9a9a9a"),
        .accent          = QColor("#4cc2ff"),
        .accentMuted     = QColor("#434343"),
        .accentContainer = QColor("#143a52"),
        .onAccent        = QColor("#003a58"),
        .green           = QColor("#6ccb5f"),
        .yellowGreen     = QColor("#a5c93a"),
        .yellow          = QColor("#fce100"),
        .orange          = QColor("#ff8c00"),
        .red             = QColor("#ff99a4"),
        .grey            = QColor("#9a9a9a"),
    };
}

PlatformTheme::Shape FluentPlatformTheme::shape() const
{
    return Shape { .controlRadiusRatio = 0.35, .capsuleControls = false };
}

QString FluentPlatformTheme::fontFamily() const
{
#ifdef Q_OS_WIN
    return PlatformTheme::fontFamily();
#else
    return QStringLiteral("Open Sans");
#endif
}
