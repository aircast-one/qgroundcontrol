/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCPalette.h"
#include "QGCCorePlugin.h"
#include "PlatformTheme.h"

#include <QtCore/QDebug>

QList<QGCPalette*>   QGCPalette::_paletteObjects;

QGCPalette::Theme QGCPalette::_theme = QGCPalette::Dark;

QMap<int, QMap<int, QMap<QString, QColor>>> QGCPalette::_colorInfoMap;

QStringList QGCPalette::_colors;

QGCPalette::QGCPalette(QObject* parent) :
    QObject(parent),
    _colorGroupEnabled(true)
{
    if (_colorInfoMap.isEmpty()) {
        _buildMap();
    }
    _paletteObjects += this;
}

QGCPalette::~QGCPalette()
{
    bool fSuccess = _paletteObjects.removeOne(this);
    if (!fSuccess) {
        qWarning() << "Internal error";
    }
}

void QGCPalette::_buildMap()
{
    const PlatformTheme *platform = PlatformTheme::instance();
    const PlatformTheme::Tones l = platform->tones(PlatformTheme::LightAppearance);
    const PlatformTheme::Tones d = platform->tones(PlatformTheme::DarkAppearance);

    const auto alpha = [](const QColor &color, int a) { QColor c(color); c.setAlpha(a); return c; };

    DECLARE_QGC_COLOR(window,               l.background, l.background, d.background, d.background)
    DECLARE_QGC_COLOR(toolbarBackground,    l.background, l.background, d.background, d.background)
    DECLARE_QGC_COLOR(windowShade,          l.surface, l.surface, d.surface, d.surface)
    DECLARE_QGC_COLOR(windowShadeDark,      l.surfaceSunken, l.surfaceSunken, d.surfaceSunken, d.surfaceSunken)
    DECLARE_QGC_COLOR(windowShadeLight,     l.outline, l.fill, d.outline, d.fill)

    DECLARE_QGC_COLOR(overlayBackground,    l.scrim, l.scrim, d.scrim, d.scrim)
    DECLARE_QGC_COLOR(overlayGlass,         l.glass, l.glass, d.glass, d.glass)
    DECLARE_QGC_COLOR(overlayBorder,        alpha(l.ink, 0x26), alpha(l.ink, 0x26), alpha(d.ink, 0x26), alpha(d.ink, 0x26))
    DECLARE_QGC_COLOR(overlayCard,          alpha(l.ink, 0x1c), alpha(l.ink, 0x1c), alpha(d.ink, 0x1c), alpha(d.ink, 0x1c))
    DECLARE_QGC_COLOR(overlayInk,           l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(overlayInkInverse,    d.inkMuted, d.ink, l.inkMuted, l.ink)

    DECLARE_QGC_COLOR(text,                 l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(toolbarText,          l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(buttonText,           l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(textFieldText,        l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(statusFailedText,     l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(statusPassedText,     l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(statusPendingText,    l.inkMuted, l.ink, d.inkMuted, d.ink)
    DECLARE_QGC_COLOR(warningText,          l.red, l.red, d.red, d.red)

    DECLARE_QGC_COLOR(button,               l.surfaceRaised, l.surfaceRaised, d.surfaceRaised, d.surfaceRaised)
    DECLARE_QGC_COLOR(buttonBorder,         l.surfaceRaised, l.outline, d.surfaceRaised, d.outline)
    DECLARE_QGC_COLOR(buttonHighlight,      l.accentMuted, l.accent, d.accentMuted, d.accent)
    DECLARE_QGC_COLOR(buttonHighlightText,  l.inkMuted, l.onAccent, d.inkMuted, d.onAccent)
    DECLARE_QGC_COLOR(primaryButton,        l.accentMuted, l.accent, d.accentMuted, d.accent)
    DECLARE_QGC_COLOR(primaryButtonText,    l.inkMuted, l.onAccent, d.inkMuted, d.onAccent)
    DECLARE_QGC_COLOR(textField,            l.background, l.surface, d.background, d.surface)
    DECLARE_QGC_COLOR(groupBorder,          l.outlineWeak, l.outlineWeak, d.outlineWeak, d.outlineWeak)
    DECLARE_QGC_COLOR(toolStripHoverColor,  l.surfaceSunken, l.outline, d.surfaceSunken, d.outline)
    DECLARE_QGC_COLOR(missionItemEditor,    l.surfaceSunken, l.accentContainer, d.surfaceSunken, d.accentContainer)

    DECLARE_QGC_COLOR(colorGreen,           l.green, l.green, d.green, d.green)
    DECLARE_QGC_COLOR(colorYellow,          l.yellow, l.yellow, d.yellow, d.yellow)
    DECLARE_QGC_COLOR(colorYellowGreen,     l.yellowGreen, l.yellowGreen, d.yellowGreen, d.yellowGreen)
    DECLARE_QGC_COLOR(colorOrange,          l.orange, l.orange, d.orange, d.orange)
    DECLARE_QGC_COLOR(colorRed,             l.red, l.red, d.red, d.red)
    DECLARE_QGC_COLOR(colorGrey,            l.grey, l.grey, d.grey, d.grey)
    DECLARE_QGC_COLOR(colorBlue,            l.accent, l.accent, d.accent, d.accent)
    DECLARE_QGC_COLOR(alertBackground,      l.yellow, l.yellow, d.yellow, d.yellow)
    DECLARE_QGC_COLOR(alertBorder,          l.grey, l.grey, d.grey, d.grey)

    DECLARE_QGC_COLOR(mapIndicator,         "#585858", l.accent, "#585858", d.accent)
    DECLARE_QGC_NONTHEMED_COLOR(mapButton,          "#585858", "#000000")
    DECLARE_QGC_NONTHEMED_COLOR(mapButtonHighlight, "#585858", "#be781c")
    DECLARE_QGC_NONTHEMED_COLOR(mapIndicatorChild,  "#585858", "#4b7ba8")
    DECLARE_QGC_NONTHEMED_COLOR(brandingPurple,     "#4A2C6D", "#4A2C6D")
    DECLARE_QGC_NONTHEMED_COLOR(brandingBlue,       "#48D6FF", "#6045c5")
    DECLARE_QGC_NONTHEMED_COLOR(toolStripFGColor,   "#707070", "#ffffff")
    DECLARE_QGC_SINGLE_COLOR(alertText,                     "#000000")
    DECLARE_QGC_SINGLE_COLOR(mapWidgetBorderLight,          "#ffffff")
    DECLARE_QGC_SINGLE_COLOR(mapWidgetBorderDark,           "#000000")
    DECLARE_QGC_SINGLE_COLOR(mapMissionTrajectory,          "#0a84ff")
    DECLARE_QGC_SINGLE_COLOR(surveyPolygonInterior,         "green")
    DECLARE_QGC_SINGLE_COLOR(surveyPolygonTerrainCollision, "red")
#ifdef QGC_UTM_ADAPTER
    DECLARE_QGC_COLOR(switchUTMSP,        "#b0e0e6", "#b0e0e6", "#b0e0e6", "#b0e0e6");
    DECLARE_QGC_COLOR(sliderUTMSP,        "#9370db", "#9370db", "#9370db", "#9370db");
    DECLARE_QGC_COLOR(successNotifyUTMSP, "#3cb371", "#3cb371", "#3cb371", "#3cb371");
#endif
}

void QGCPalette::setColorGroupEnabled(bool enabled)
{
    _colorGroupEnabled = enabled;
    emit paletteChanged();
}

void QGCPalette::setGlobalTheme(Theme newTheme)
{
    if (_theme != newTheme) {
        _theme = newTheme;
        _signalPaletteChangeToAll();
    }
}

void QGCPalette::_signalPaletteChangeToAll()
{
    for (QGCPalette *palette : std::as_const(_paletteObjects)) {
        palette->_signalPaletteChanged();
    }
}

void QGCPalette::_signalPaletteChanged()
{
    emit paletteChanged();
}
