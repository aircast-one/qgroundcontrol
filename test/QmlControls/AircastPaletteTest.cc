#include "AircastPaletteTest.h"

#include <QtTest/QTest>

#include "AircastTheme.h"
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
        QCOMPARE(enabled.window(), QColor::fromRgb(s.surface));
        QCOMPARE(enabled.windowShade(), QColor::fromRgb(s.surfaceContainer));
        QCOMPARE(enabled.text(), QColor::fromRgb(s.onSurface));
        QCOMPARE(enabled.button(), QColor::fromRgb(s.secondaryContainer));
        QCOMPARE(enabled.buttonHighlight(), QColor::fromRgb(s.primary));
        QCOMPARE(enabled.colorBlue(), QColor::fromRgb(s.primary));
        QCOMPARE(enabled.colorRed(), QColor::fromRgb(s.error));
        QCOMPARE(enabled.colorGreen(), QColor::fromRgb(s.success));
        QCOMPARE(enabled.colorOrange(), QColor::fromRgb(s.warning));
        QCOMPARE(enabled.groupBorder(), QColor::fromRgb(s.outlineVariant));

        QGCPalette disabled;
        disabled.setColorGroupEnabled(false);
        QCOMPARE(disabled.text(), QColor::fromRgb(s.onSurfaceVariant));
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
