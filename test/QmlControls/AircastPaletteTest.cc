#include "AircastPaletteTest.h"

#include <QtTest/QTest>

#include "AircastTheme.h"
#include "QGCCorePlugin.h"
#include "QGCPalette.h"

namespace {

qreal luminance(const QColor& color)
{
    const auto channel = [](qreal v) { return v <= 0.03928 ? v / 12.92 : qPow((v + 0.055) / 1.055, 2.4); };
    return 0.2126 * channel(color.redF()) + 0.7152 * channel(color.greenF()) + 0.0722 * channel(color.blueF());
}

qreal contrastRatio(const QColor& a, const QColor& b)
{
    const qreal la = luminance(a);
    const qreal lb = luminance(b);
    return (qMax(la, lb) + 0.05) / (qMin(la, lb) + 0.05);
}

QColor themedRole(const char* name, QGCPalette::Theme theme, QGCPalette::ColorGroup group, QRgb token)
{
    const QColor color = QColor::fromRgb(token);
    QGCPalette::PaletteColorInfo_t info = {{color, color}, {color, color}};
    QGCCorePlugin::instance()->paletteOverride(QString::fromLatin1(name), info);
    return info[theme][group];
}

constexpr qreal kBodyTextContrast = 4.5;
constexpr qreal kControlContrast = 3.0;

const struct
{
    QGCPalette::Theme theme;
    AircastTheme::Appearance appearance;
} kThemes[] = {
    {QGCPalette::Light, AircastTheme::LightAppearance},
    {QGCPalette::Dark, AircastTheme::DarkAppearance},
};

}  // namespace

void AircastPaletteTest::_paletteRolesAreTheTokenRoles()
{
    for (const auto& t : kThemes) {
        QGCPalette::setGlobalTheme(t.theme);
        const AircastTokens::Scheme& s = AircastTheme::scheme(t.appearance);

        QGCPalette enabled;
        enabled.setColorGroupEnabled(true);
        QCOMPARE(enabled.window(), themedRole("window", t.theme, QGCPalette::ColorGroupEnabled, s.surface));
        QCOMPARE(enabled.windowShade(),
                 themedRole("windowShade", t.theme, QGCPalette::ColorGroupEnabled, s.surfaceContainer));
        QCOMPARE(enabled.text(), themedRole("text", t.theme, QGCPalette::ColorGroupEnabled, s.onSurface));
        QCOMPARE(enabled.button(), themedRole("button", t.theme, QGCPalette::ColorGroupEnabled, s.secondaryContainer));
        QCOMPARE(enabled.buttonHighlight(),
                 themedRole("buttonHighlight", t.theme, QGCPalette::ColorGroupEnabled, s.primary));
        QCOMPARE(enabled.colorBlue(), themedRole("colorBlue", t.theme, QGCPalette::ColorGroupEnabled, s.primary));
        QCOMPARE(enabled.colorRed(), themedRole("colorRed", t.theme, QGCPalette::ColorGroupEnabled, s.error));
        QCOMPARE(enabled.colorGreen(), themedRole("colorGreen", t.theme, QGCPalette::ColorGroupEnabled, s.success));
        QCOMPARE(enabled.colorOrange(), themedRole("colorOrange", t.theme, QGCPalette::ColorGroupEnabled, s.warning));
        QCOMPARE(enabled.groupBorder(),
                 themedRole("groupBorder", t.theme, QGCPalette::ColorGroupEnabled, s.outlineVariant));

        QGCPalette disabled;
        disabled.setColorGroupEnabled(false);
        QCOMPARE(disabled.text(), themedRole("text", t.theme, QGCPalette::ColorGroupDisabled, s.onSurfaceVariant));
    }
    QCOMPARE(AircastTokens::dark.surface, 0x0D0E11u);
    QCOMPARE(AircastTokens::dark.primary, 0x0088F2u);
}

void AircastPaletteTest::_textReadsOnEverySurfaceInBothThemes()
{
    for (const auto& t : kThemes) {
        QGCPalette::setGlobalTheme(t.theme);
        QGCPalette p;
        p.setColorGroupEnabled(true);
        const QByteArray where = QByteArray::number(t.theme);
        QVERIFY2(contrastRatio(p.text(), p.window()) >= kBodyTextContrast, where.constData());
        QVERIFY2(contrastRatio(p.text(), p.windowShade()) >= kBodyTextContrast, where.constData());
        QVERIFY2(contrastRatio(p.buttonText(), p.button()) >= kBodyTextContrast, where.constData());
        QVERIFY2(contrastRatio(p.buttonHighlightText(), p.buttonHighlight()) >= kControlContrast, where.constData());
    }
}

UT_REGISTER_TEST(AircastPaletteTest, TestLabel::Unit)
