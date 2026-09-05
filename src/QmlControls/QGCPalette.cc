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
    DECLARE_QGC_COLOR(window,               "#f2f2f7", "#f2f2f7", "#1c1c1e", "#1c1c1e")
    DECLARE_QGC_COLOR(overlayBackground,    "#e6f2f2f7", "#e6f2f2f7", "#d91c1c1e", "#d91c1c1e")
    DECLARE_QGC_COLOR(overlayBorder,        "#26000000", "#26000000", "#29ffffff", "#29ffffff")
    DECLARE_QGC_COLOR(overlayGlass,         "#8cf2f2f7", "#8cf2f2f7", "#73151515", "#73151515")
    DECLARE_QGC_COLOR(overlayGlassLight,    "#8cf2f2f7", "#8cf2f2f7", "#bfe9e9ee", "#bfe9e9ee")
    DECLARE_QGC_COLOR(toolbarText,          "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(windowShadeLight,     "#d1d1d6", "#c7c7cc", "#3a3a3c", "#48484a")
    DECLARE_QGC_COLOR(windowShade,          "#ffffff", "#ffffff", "#2c2c2e", "#2c2c2e")
    DECLARE_QGC_COLOR(windowShadeDark,      "#e5e5ea", "#e5e5ea", "#242426", "#242426")
    DECLARE_QGC_COLOR(text,                 "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(warningText,          "#ff3b30", "#ff3b30", "#ff453a", "#ff453a")
    DECLARE_QGC_COLOR(button,               "#e9e9eb", "#e9e9eb", "#2c2c2e", "#3a3a3c")
    DECLARE_QGC_COLOR(buttonBorder,         "#e9e9eb", "#d1d1d6", "#2c2c2e", "#48484a")
    DECLARE_QGC_COLOR(buttonText,           "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(buttonHighlight,      "#d1d1d6", "#007aff", "#3a3a3c", "#0a84ff")
    DECLARE_QGC_COLOR(buttonHighlightText,  "#8e8e93", "#ffffff", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(primaryButton,        "#d1d1d6", "#007aff", "#3a3a3c", "#0a84ff")
    DECLARE_QGC_COLOR(primaryButtonText,    "#ffffff", "#ffffff", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(textField,            "#f2f2f7", "#ffffff", "#2c2c2e", "#1c1c1e")
    DECLARE_QGC_COLOR(textFieldText,        "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(mapButton,            "#585858", "#000000", "#585858", "#000000")
    DECLARE_QGC_COLOR(mapButtonHighlight,   "#585858", "#be781c", "#585858", "#be781c")
    DECLARE_QGC_COLOR(mapIndicator,         "#585858", "#007aff", "#585858", "#0a84ff")
    DECLARE_QGC_COLOR(mapIndicatorChild,    "#585858", "#4b7ba8", "#585858", "#4b7ba8")
    DECLARE_QGC_COLOR(colorGreen,           "#34c759", "#34c759", "#30d158", "#30d158") 
    DECLARE_QGC_COLOR(colorYellow,          "#ffcc00", "#ffcc00", "#ffd60a", "#ffd60a")  
    DECLARE_QGC_COLOR(colorYellowGreen,     "#799f26", "#799f26", "#9dbe2f", "#9dbe2f")  
    DECLARE_QGC_COLOR(colorOrange,          "#ff9500", "#ff9500", "#ff9f0a", "#ff9f0a")  
    DECLARE_QGC_COLOR(colorRed,             "#ff3b30", "#ff3b30", "#ff453a", "#ff453a")
    DECLARE_QGC_COLOR(colorGrey,            "#8e8e93", "#8e8e93", "#8e8e93", "#8e8e93")
    DECLARE_QGC_COLOR(colorBlue,            "#007aff", "#007aff", "#0a84ff", "#0a84ff")
    DECLARE_QGC_COLOR(alertBackground,      "#ffcc00", "#ffcc00", "#ffd60a", "#ffd60a")
    DECLARE_QGC_COLOR(alertBorder,          "#8e8e93", "#8e8e93", "#8e8e93", "#8e8e93")
    DECLARE_QGC_COLOR(alertText,            "#000000", "#000000", "#000000", "#000000")
    DECLARE_QGC_COLOR(missionItemEditor,    "#e5e5ea", "#e5f1ff", "#2c2c2e", "#2c2c2e")
    DECLARE_QGC_COLOR(toolStripHoverColor,  "#e5e5ea", "#d1d1d6", "#2c2c2e", "#3a3a3c")
    DECLARE_QGC_COLOR(statusFailedText,     "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(statusPassedText,     "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(statusPendingText,    "#8e8e93", "#000000", "#8e8e93", "#ffffff")
    DECLARE_QGC_COLOR(toolbarBackground,    "#f2f2f7", "#f2f2f7", "#1c1c1e", "#1c1c1e")
    DECLARE_QGC_COLOR(groupBorder,          "#d1d1d6", "#d1d1d6", "#38383a", "#38383a")
    DECLARE_QGC_NONTHEMED_COLOR(brandingPurple,     "#4A2C6D", "#4A2C6D")
    DECLARE_QGC_NONTHEMED_COLOR(brandingBlue,       "#48D6FF", "#6045c5")
    DECLARE_QGC_NONTHEMED_COLOR(toolStripFGColor,   "#707070", "#ffffff")
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
