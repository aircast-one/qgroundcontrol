#include "PlatformThemeTest.h"
#include "ApplePlatformTheme.h"
#include "FluentPlatformTheme.h"
#include "MaterialPlatformTheme.h"
#include "PlatformTheme.h"
#include "QGCPalette.h"

#include <QtTest/QTest>

namespace {

qreal luminance(const QColor &color)
{
    const auto channel = [](qreal v) {
        return v <= 0.03928 ? v / 12.92 : qPow((v + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * channel(color.redF()) + 0.7152 * channel(color.greenF()) + 0.0722 * channel(color.blueF());
}

qreal contrastRatio(const QColor &a, const QColor &b)
{
    const qreal la = luminance(a);
    const qreal lb = luminance(b);
    return (qMax(la, lb) + 0.05) / (qMin(la, lb) + 0.05);
}

constexpr qreal kBodyTextContrast = 4.5;
constexpr qreal kControlContrast  = 3.0;

QList<const PlatformTheme *> everyTheme()
{
    static ApplePlatformTheme    apple;
    static MaterialPlatformTheme material;
    static FluentPlatformTheme   fluent;
    return { &apple, &material, &fluent };
}

QList<QColor> everyTone(const PlatformTheme::Tones &t)
{
    return { t.background, t.surface, t.surfaceSunken, t.surfaceRaised, t.scrim, t.glass, t.outline, t.outlineWeak,
             t.fill, t.ink, t.inkMuted, t.accent, t.accentMuted, t.accentContainer, t.onAccent,
             t.green, t.yellowGreen, t.yellow, t.orange, t.red, t.grey };
}

QString label(const PlatformTheme *theme, PlatformTheme::Appearance appearance)
{
    return QStringLiteral("%1 %2").arg(theme->name(), appearance == PlatformTheme::LightAppearance ? "light" : "dark");
}

} // namespace

void PlatformThemeTest::_everyThemeSuppliesValidTonesInBothAppearances()
{
    for (const PlatformTheme *theme : everyTheme()) {
        for (const PlatformTheme::Appearance appearance : { PlatformTheme::LightAppearance, PlatformTheme::DarkAppearance }) {
            const QList<QColor> tones = everyTone(theme->tones(appearance));
            QCOMPARE(tones.size(), 21);
            for (const QColor &tone : tones) {
                QVERIFY2(tone.isValid(), qPrintable(label(theme, appearance)));
            }
        }
    }
}

void PlatformThemeTest::_inkReadsOnEverySurfaceInEveryTheme()
{
    for (const PlatformTheme *theme : everyTheme()) {
        for (const PlatformTheme::Appearance appearance : { PlatformTheme::LightAppearance, PlatformTheme::DarkAppearance }) {
            const PlatformTheme::Tones t = theme->tones(appearance);
            const QByteArray whereBytes = label(theme, appearance).toUtf8();
            const char *const where = whereBytes.constData();
            QVERIFY2(contrastRatio(t.ink, t.background) >= kBodyTextContrast, where);
            QVERIFY2(contrastRatio(t.ink, t.surface)    >= kBodyTextContrast, where);
            QVERIFY2(contrastRatio(t.ink, t.surfaceRaised) >= kBodyTextContrast, where);
            QVERIFY2(contrastRatio(t.onAccent, t.accent) >= kControlContrast, where);
        }
    }
}

void PlatformThemeTest::_paletteRolesDeriveFromTheHostTones()
{
    const PlatformTheme *host = PlatformTheme::instance();

    const struct { QGCPalette::Theme theme; PlatformTheme::Appearance appearance; } cases[] = {
        { QGCPalette::Light, PlatformTheme::LightAppearance },
        { QGCPalette::Dark,  PlatformTheme::DarkAppearance },
    };

    for (const auto &c : cases) {
        QGCPalette::setGlobalTheme(c.theme);
        const PlatformTheme::Tones t = host->tones(c.appearance);

        QGCPalette enabled;
        enabled.setColorGroupEnabled(true);
        QCOMPARE(enabled.window(),          t.background);
        QCOMPARE(enabled.toolbarBackground(), t.background);
        QCOMPARE(enabled.windowShade(),     t.surface);
        QCOMPARE(enabled.windowShadeDark(), t.surfaceSunken);
        QCOMPARE(enabled.text(),            t.ink);
        QCOMPARE(enabled.buttonText(),      t.ink);
        QCOMPARE(enabled.buttonHighlight(), t.accent);
        QCOMPARE(enabled.primaryButton(),   t.accent);
        QCOMPARE(enabled.colorBlue(),       t.accent);
        QCOMPARE(enabled.colorRed(),        t.red);
        QCOMPARE(enabled.warningText(),     t.red);
        QCOMPARE(enabled.groupBorder(),     t.outlineWeak);
        QCOMPARE(enabled.overlayGlass(),    t.glass);
        QCOMPARE(enabled.overlayBackground(), t.scrim);
        QCOMPARE(enabled.overlayBorder().alpha(), 0x26);
        QCOMPARE(enabled.overlayCard().alpha(),   0x1c);

        QGCPalette disabled;
        disabled.setColorGroupEnabled(false);
        QCOMPARE(disabled.text(),            t.inkMuted);
        QCOMPARE(disabled.buttonHighlight(), t.accentMuted);
        QCOMPARE(disabled.primaryButtonText(), t.inkMuted);
    }
}
