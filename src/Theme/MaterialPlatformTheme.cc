/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "MaterialPlatformTheme.h"

#include <optional>

#ifdef Q_OS_ANDROID
#include "AndroidInterface.h"
#endif

namespace {

QColor withAlpha(const QColor &color, int alpha)
{
    QColor result(color);
    result.setAlpha(alpha);
    return result;
}

} // namespace

PlatformTheme::Tones MaterialPlatformTheme::_baselineTones(Appearance appearance)
{
    if (appearance == LightAppearance) {
        return Tones {
            .background      = QColor("#fef7ff"),
            .surface         = QColor("#f7f2fa"),
            .surfaceSunken   = QColor("#f3edf7"),
            .surfaceRaised   = QColor("#ece6f0"),
            .scrim           = QColor("#e6fef7ff"),
            .glass           = QColor("#d9fef7ff"),
            .outline         = QColor("#79747e"),
            .outlineWeak     = QColor("#cac4d0"),
            .fill            = QColor("#e7e0ec"),
            .ink             = QColor("#1d1b20"),
            .inkMuted        = QColor("#79747e"),
            .accent          = QColor("#6750a4"),
            .accentMuted     = QColor("#e7e0ec"),
            .accentContainer = QColor("#eaddff"),
            .onAccent        = QColor("#ffffff"),
            .green           = QColor("#2e7d32"),
            .yellowGreen     = QColor("#7cb342"),
            .yellow          = QColor("#f9a825"),
            .orange          = QColor("#ef6c00"),
            .red             = QColor("#b3261e"),
            .grey            = QColor("#79747e"),
        };
    }

    return Tones {
        .background      = QColor("#141218"),
        .surface         = QColor("#211f26"),
        .surfaceSunken   = QColor("#1d1b20"),
        .surfaceRaised   = QColor("#2b2930"),
        .scrim           = QColor("#d9141218"),
        .glass           = QColor("#73141218"),
        .outline         = QColor("#938f99"),
        .outlineWeak     = QColor("#49454f"),
        .fill            = QColor("#49454f"),
        .ink             = QColor("#e6e0e9"),
        .inkMuted        = QColor("#938f99"),
        .accent          = QColor("#d0bcff"),
        .accentMuted     = QColor("#49454f"),
        .accentContainer = QColor("#4f378b"),
        .onAccent        = QColor("#381e72"),
        .green           = QColor("#81c784"),
        .yellowGreen     = QColor("#aed581"),
        .yellow          = QColor("#ffd54f"),
        .orange          = QColor("#ffb74d"),
        .red             = QColor("#f2b8b5"),
        .grey            = QColor("#938f99"),
    };
}

bool MaterialPlatformTheme::_applyDynamicColor(Appearance appearance, Tones &tones)
{
#ifdef Q_OS_ANDROID
    const QStringList names = appearance == LightAppearance
        ? QStringList{ "system_neutral1_10", "system_neutral1_50", "system_neutral1_100", "system_neutral2_100", "system_neutral2_500",
                       "system_neutral2_200", "system_neutral2_100", "system_neutral1_900", "system_neutral2_500", "system_accent1_600",
                       "system_neutral2_100", "system_accent1_100", "system_accent1_0" }
        : QStringList{ "system_neutral1_900", "system_neutral1_800", "system_neutral1_900", "system_neutral2_700", "system_neutral2_400",
                       "system_neutral2_700", "system_neutral2_700", "system_neutral1_100", "system_neutral2_400", "system_accent1_200",
                       "system_neutral2_700", "system_accent1_700", "system_accent1_800" };
    QList<QColor> colors;
    for (const QString &name : names) {
        const QColor color = AndroidInterface::systemColor(name);
        if (!color.isValid()) {
            qCWarning(AndroidInterfaceLog) << "Dynamic colour unavailable, keeping the Material baseline:" << name;
            return false;
        }
        colors.append(color);
    }

    tones.background      = colors[0];
    tones.surface         = colors[1];
    tones.surfaceSunken   = colors[2];
    tones.surfaceRaised   = colors[3];
    tones.outline         = colors[4];
    tones.outlineWeak     = colors[5];
    tones.fill            = colors[6];
    tones.ink             = colors[7];
    tones.inkMuted        = colors[8];
    tones.accent          = colors[9];
    tones.accentMuted     = colors[10];
    tones.accentContainer = colors[11];
    tones.onAccent        = colors[12];

    tones.scrim = withAlpha(tones.background, 0xe6);
    tones.glass = withAlpha(tones.background, appearance == LightAppearance ? 0xd9 : 0x73);
    tones.grey  = tones.inkMuted;

    return true;
#else
    Q_UNUSED(appearance);
    Q_UNUSED(tones);
    return false;
#endif
}

PlatformTheme::Tones MaterialPlatformTheme::tones(Appearance appearance) const
{
    std::optional<Tones> &cached = _cache[appearance == LightAppearance ? 0 : 1];
    if (!cached) {
        Tones tones = _baselineTones(appearance);
        (void) _applyDynamicColor(appearance, tones);
        cached = tones;
    }
    return *cached;
}

PlatformTheme::Shape MaterialPlatformTheme::shape() const
{
    return Shape { .controlRadiusRatio = 0.5, .capsuleControls = true };
}

QString MaterialPlatformTheme::controlStyle() const
{
    return QStringLiteral("Material");
}

void MaterialPlatformTheme::applySystemChrome(Appearance appearance, const Tones &tones)
{
    Q_UNUSED(tones);
#ifdef Q_OS_ANDROID
    AndroidInterface::setSystemBarAppearance(appearance == LightAppearance);
#else
    Q_UNUSED(appearance);
#endif
}
