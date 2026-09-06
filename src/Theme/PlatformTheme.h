/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QString>
#include <QtGui/QColor>

class PlatformTheme
{
public:
    enum Appearance {
        LightAppearance,
        DarkAppearance
    };

    struct Tones {
        QColor background;
        QColor surface;
        QColor surfaceSunken;
        QColor surfaceRaised;
        QColor scrim;
        QColor glass;
        QColor outline;
        QColor outlineWeak;
        QColor fill;
        QColor ink;
        QColor inkMuted;
        QColor accent;
        QColor accentMuted;
        QColor accentContainer;
        QColor onAccent;
        QColor green;
        QColor yellowGreen;
        QColor yellow;
        QColor orange;
        QColor red;
        QColor grey;
    };

    struct Shape {
        qreal controlRadiusRatio;
        bool capsuleControls;
    };

    virtual ~PlatformTheme() = default;

    static PlatformTheme *instance();

    virtual QString name() const = 0;
    virtual Tones tones(Appearance appearance) const = 0;
    virtual Shape shape() const = 0;
    virtual QString fontFamily() const;
    virtual QString controlStyle() const;
    virtual void applySystemChrome(Appearance appearance, const Tones &tones);
};
