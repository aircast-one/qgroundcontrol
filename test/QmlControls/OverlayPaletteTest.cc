/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "OverlayPaletteTest.h"
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

constexpr qreal kMinContrast = 4.5;

}

void OverlayPaletteTest::_inkContrastsWithTheMaterialInBothThemes()
{
    for (const QGCPalette::Theme theme : { QGCPalette::Light, QGCPalette::Dark }) {
        QGCPalette::setGlobalTheme(theme);
        QGCPalette palette;
        palette.setColorGroupEnabled(true);
        QVERIFY2(contrastRatio(palette.overlayInk(), palette.overlayGlass()) >= kMinContrast,
                 qPrintable(QStringLiteral("overlayInk on overlayGlass, theme %1").arg(theme)));
    }
}

void OverlayPaletteTest::_invertedInkContrastsWithTheInkInBothThemes()
{
    for (const QGCPalette::Theme theme : { QGCPalette::Light, QGCPalette::Dark }) {
        QGCPalette::setGlobalTheme(theme);
        QGCPalette palette;
        palette.setColorGroupEnabled(true);
        QVERIFY2(contrastRatio(palette.overlayInkInverse(), palette.overlayInk()) >= kMinContrast,
                 qPrintable(QStringLiteral("overlayInkInverse on overlayInk, theme %1").arg(theme)));
    }
}

void OverlayPaletteTest::_materialFollowsTheThemeLikeTheWindow()
{
    QGCPalette::setGlobalTheme(QGCPalette::Light);
    QGCPalette light;
    light.setColorGroupEnabled(true);
    const QColor lightGlass = light.overlayGlass();
    const QColor lightWindow = light.window();

    QGCPalette::setGlobalTheme(QGCPalette::Dark);
    QGCPalette dark;
    dark.setColorGroupEnabled(true);

    QVERIFY(lightGlass != dark.overlayGlass());
    QVERIFY((luminance(lightGlass) > luminance(dark.overlayGlass()))
            == (luminance(lightWindow) > luminance(dark.window())));
}
