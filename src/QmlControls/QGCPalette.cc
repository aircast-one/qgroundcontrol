#include "QGCPalette.h"
#include "QGCCorePlugin.h"
#include "AircastTheme.h"

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
    const AircastTokens::Scheme &l = AircastTheme::scheme(AircastTheme::LightAppearance);
    const AircastTokens::Scheme &d = AircastTheme::scheme(AircastTheme::DarkAppearance);

    const auto rgb = [](std::uint32_t hex) { return QColor::fromRgb(hex); };
    const auto alpha = [&rgb](std::uint32_t hex, int a) { QColor c = rgb(hex); c.setAlpha(a); return c; };
    const auto between = [&rgb](std::uint32_t a, std::uint32_t b) {
        const QColor x = rgb(a);
        const QColor y = rgb(b);
        return QColor((x.red() + y.red()) / 2, (x.green() + y.green()) / 2, (x.blue() + y.blue()) / 2);
    };

    DECLARE_QGC_COLOR(window,               rgb(l.surface), rgb(l.surface), rgb(d.surface), rgb(d.surface))
    DECLARE_QGC_COLOR(toolbarBackground,    rgb(l.surface), rgb(l.surface), rgb(d.surface), rgb(d.surface))
    DECLARE_QGC_COLOR(windowShade,          rgb(l.surfaceContainer), rgb(l.surfaceContainer), rgb(d.surfaceContainer), rgb(d.surfaceContainer))
    DECLARE_QGC_COLOR(windowShadeDark,      rgb(l.surfaceContainerLowest), rgb(l.surfaceContainerLowest), rgb(d.surfaceContainerLowest), rgb(d.surfaceContainerLowest))
    DECLARE_QGC_COLOR(windowShadeLight,     rgb(l.outlineVariant), rgb(l.outline), rgb(d.outlineVariant), rgb(d.outline))

    DECLARE_QGC_COLOR(text,                 rgb(l.onSurfaceVariant), rgb(l.onSurface), rgb(d.onSurfaceVariant), rgb(d.onSurface))
    DECLARE_QGC_COLOR(toolbarText,          rgb(l.onSurfaceVariant), rgb(l.onSurface), rgb(d.onSurfaceVariant), rgb(d.onSurface))
    DECLARE_QGC_COLOR(buttonText,           rgb(l.onSurfaceVariant), rgb(l.onSecondaryContainer), rgb(d.onSurfaceVariant), rgb(d.onSecondaryContainer))
    DECLARE_QGC_COLOR(textFieldText,        rgb(l.onSurfaceVariant), rgb(l.onSurface), rgb(d.onSurfaceVariant), rgb(d.onSurface))
    DECLARE_QGC_COLOR(statusFailedText,     rgb(l.onSurfaceVariant), rgb(l.onSurface), rgb(d.onSurfaceVariant), rgb(d.onSurface))
    DECLARE_QGC_COLOR(statusPassedText,     rgb(l.onSurfaceVariant), rgb(l.onSurface), rgb(d.onSurfaceVariant), rgb(d.onSurface))
    DECLARE_QGC_COLOR(statusPendingText,    rgb(l.onSurfaceVariant), rgb(l.onSurface), rgb(d.onSurfaceVariant), rgb(d.onSurface))
    DECLARE_QGC_COLOR(warningText,          rgb(l.error), rgb(l.error), rgb(d.error), rgb(d.error))

    DECLARE_QGC_COLOR(button,               rgb(l.secondaryContainer), rgb(l.secondaryContainer), rgb(d.secondaryContainer), rgb(d.secondaryContainer))
    DECLARE_QGC_COLOR(buttonBorder,         rgb(l.secondaryContainer), rgb(l.outlineVariant), rgb(d.secondaryContainer), rgb(d.outlineVariant))
    DECLARE_QGC_COLOR(buttonHighlight,      rgb(l.surfaceVariant), rgb(l.primary), rgb(d.surfaceVariant), rgb(d.primary))
    DECLARE_QGC_COLOR(buttonHighlightText,  rgb(l.onSurfaceVariant), rgb(l.onPrimary), rgb(d.onSurfaceVariant), rgb(d.onPrimary))
    DECLARE_QGC_COLOR(primaryButton,        rgb(l.surfaceVariant), rgb(l.primary), rgb(d.surfaceVariant), rgb(d.primary))
    DECLARE_QGC_COLOR(primaryButtonText,    rgb(l.onSurfaceVariant), rgb(l.onPrimary), rgb(d.onSurfaceVariant), rgb(d.onPrimary))
    DECLARE_QGC_COLOR(textField,            rgb(l.surface), rgb(l.surfaceContainerLow), rgb(d.surface), rgb(d.surfaceContainerLow))
    DECLARE_QGC_COLOR(groupBorder,          rgb(l.outlineVariant), rgb(l.outlineVariant), rgb(d.outlineVariant), rgb(d.outlineVariant))
    DECLARE_QGC_COLOR(toolStripHoverColor,  rgb(l.surfaceContainerLowest), rgb(l.surfaceContainerHighest), rgb(d.surfaceContainerLowest), rgb(d.surfaceContainerHighest))
    DECLARE_QGC_COLOR(missionItemEditor,    rgb(l.surfaceContainerLowest), rgb(l.primaryContainer), rgb(d.surfaceContainerLowest), rgb(d.primaryContainer))

    DECLARE_QGC_COLOR(colorGreen,           rgb(l.success), rgb(l.success), rgb(d.success), rgb(d.success))
    DECLARE_QGC_COLOR(colorYellow,          rgb(l.warning), rgb(l.warning), rgb(d.warning), rgb(d.warning))
    DECLARE_QGC_COLOR(colorYellowGreen,     between(l.success, l.warning), between(l.success, l.warning), between(d.success, d.warning), between(d.success, d.warning))
    DECLARE_QGC_COLOR(colorOrange,          rgb(l.warning), rgb(l.warning), rgb(d.warning), rgb(d.warning))
    DECLARE_QGC_COLOR(colorRed,             rgb(l.error), rgb(l.error), rgb(d.error), rgb(d.error))
    DECLARE_QGC_COLOR(colorGrey,            rgb(l.secondary), rgb(l.secondary), rgb(d.secondary), rgb(d.secondary))
    DECLARE_QGC_COLOR(colorBlue,            rgb(l.primary), rgb(l.primary), rgb(d.primary), rgb(d.primary))
    DECLARE_QGC_COLOR(alertBackground,      rgb(l.warning), rgb(l.warning), rgb(d.warning), rgb(d.warning))
    DECLARE_QGC_COLOR(alertBorder,          rgb(l.outline), rgb(l.outline), rgb(d.outline), rgb(d.outline))
    DECLARE_QGC_COLOR(windowTransparent,    alpha(l.surface, 0xcc), alpha(l.surface, 0xcc), alpha(d.surface, 0xcc), alpha(d.surface, 0xcc))
    DECLARE_QGC_COLOR(modifiedParamValue,   rgb(l.warning), rgb(l.warning), rgb(d.warning), rgb(d.warning))

    DECLARE_QGC_COLOR(mapIndicator,         "#585858", rgb(l.primary), "#585858", rgb(d.primary))
    DECLARE_QGC_NONTHEMED_COLOR(mapButton,          "#585858", "#000000")
    DECLARE_QGC_NONTHEMED_COLOR(mapButtonHighlight, "#585858", "#be781c")
    DECLARE_QGC_NONTHEMED_COLOR(mapIndicatorChild,  "#585858", "#4b7ba8")
    DECLARE_QGC_NONTHEMED_COLOR(brandingPurple,     "#4A2C6D", "#4A2C6D")
    DECLARE_QGC_NONTHEMED_COLOR(brandingBlue,       "#48D6FF", "#6045c5")
    DECLARE_QGC_NONTHEMED_COLOR(toolStripFGColor,   "#707070", "#ffffff")
    DECLARE_QGC_NONTHEMED_COLOR(photoCaptureButtonColor, "#707070", "#ffffff")
    DECLARE_QGC_NONTHEMED_COLOR(videoCaptureButtonColor, "#f89a9e", "#f32836")
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
