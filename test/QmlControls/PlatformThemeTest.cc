#include "PlatformThemeTest.h"
#include "ApplePlatformTheme.h"
#include "FluentPlatformTheme.h"
#include "MaterialPlatformTheme.h"
#include "PlatformTheme.h"
#include "QGCCorePlugin.h"
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

// A core plugin may recolour palette roles (the custom example does), and those no longer derive
// from the host tones.
bool overriddenByCorePlugin(const char *role)
{
    const QColor sentinel(0x12, 0x34, 0x56);
    QGCPalette::PaletteColorInfo_t info;
    for (auto &theme : info) {
        for (QColor &color : theme) {
            color = sentinel;
        }
    }
    QGCCorePlugin::instance()->paletteOverride(QString::fromLatin1(role), info);
    for (const auto &theme : info) {
        for (const QColor &color : theme) {
            if (color != sentinel) {
                return true;
            }
        }
    }
    return false;
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

#define COMPARE_ROLE(palette, role, expected) \
    if (!overriddenByCorePlugin(#role)) { \
        QCOMPARE(palette.role(), expected); \
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
        COMPARE_ROLE(enabled, window,          t.background);
        COMPARE_ROLE(enabled, toolbarBackground, t.background);
        COMPARE_ROLE(enabled, windowShade,     t.surface);
        COMPARE_ROLE(enabled, windowShadeDark, t.surfaceSunken);
        COMPARE_ROLE(enabled, text,            t.ink);
        COMPARE_ROLE(enabled, buttonText,      t.ink);
        COMPARE_ROLE(enabled, buttonHighlight, t.accent);
        COMPARE_ROLE(enabled, primaryButton,   t.accent);
        COMPARE_ROLE(enabled, colorBlue,       t.accent);
        COMPARE_ROLE(enabled, colorRed,        t.red);
        COMPARE_ROLE(enabled, warningText,     t.red);
        COMPARE_ROLE(enabled, groupBorder,     t.outlineWeak);
        COMPARE_ROLE(enabled, overlayGlass,    t.glass);
        COMPARE_ROLE(enabled, overlayBackground, t.scrim);
        if (!overriddenByCorePlugin("overlayBorder")) {
            QCOMPARE(enabled.overlayBorder().alpha(), 0x26);
        }
        if (!overriddenByCorePlugin("overlayCard")) {
            QCOMPARE(enabled.overlayCard().alpha(), 0x1c);
        }

        QGCPalette disabled;
        disabled.setColorGroupEnabled(false);
        COMPARE_ROLE(disabled, text,            t.inkMuted);
        COMPARE_ROLE(disabled, buttonHighlight, t.accentMuted);
        COMPARE_ROLE(disabled, primaryButtonText, t.inkMuted);
    }
}

#undef COMPARE_ROLE

UT_REGISTER_TEST(PlatformThemeTest, TestLabel::Unit)
