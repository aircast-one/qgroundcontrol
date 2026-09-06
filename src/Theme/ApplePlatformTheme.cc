/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "ApplePlatformTheme.h"

PlatformTheme::Tones ApplePlatformTheme::tones(Appearance appearance) const
{
    if (appearance == LightAppearance) {
        return Tones {
            .background      = QColor("#f2f2f7"),
            .surface         = QColor("#ffffff"),
            .surfaceSunken   = QColor("#e5e5ea"),
            .surfaceRaised   = QColor("#e9e9eb"),
            .scrim           = QColor("#e6f2f2f7"),
            .glass           = QColor("#d9f2f2f7"),
            .outline         = QColor("#d1d1d6"),
            .outlineWeak     = QColor("#d1d1d6"),
            .fill            = QColor("#c7c7cc"),
            .ink             = QColor("#000000"),
            .inkMuted        = QColor("#8e8e93"),
            .accent          = QColor("#007aff"),
            .accentMuted     = QColor("#d1d1d6"),
            .accentContainer = QColor("#e5f1ff"),
            .onAccent        = QColor("#ffffff"),
            .green           = QColor("#34c759"),
            .yellowGreen     = QColor("#799f26"),
            .yellow          = QColor("#ffcc00"),
            .orange          = QColor("#ff9500"),
            .red             = QColor("#ff3b30"),
            .grey            = QColor("#8e8e93"),
        };
    }

    return Tones {
        .background      = QColor("#1c1c1e"),
        .surface         = QColor("#2c2c2e"),
        .surfaceSunken   = QColor("#242426"),
        .surfaceRaised   = QColor("#3a3a3c"),
        .scrim           = QColor("#d91c1c1e"),
        .glass           = QColor("#73151515"),
        .outline         = QColor("#48484a"),
        .outlineWeak     = QColor("#38383a"),
        .fill            = QColor("#48484a"),
        .ink             = QColor("#ffffff"),
        .inkMuted        = QColor("#8e8e93"),
        .accent          = QColor("#0a84ff"),
        .accentMuted     = QColor("#3a3a3c"),
        .accentContainer = QColor("#2c2c2e"),
        .onAccent        = QColor("#ffffff"),
        .green           = QColor("#30d158"),
        .yellowGreen     = QColor("#9dbe2f"),
        .yellow          = QColor("#ffd60a"),
        .orange          = QColor("#ff9f0a"),
        .red             = QColor("#ff453a"),
        .grey            = QColor("#8e8e93"),
    };
}

PlatformTheme::Shape ApplePlatformTheme::shape() const
{
    return Shape { .controlRadiusRatio = 0.4, .capsuleControls = true };
}
