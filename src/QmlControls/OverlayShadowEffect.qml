/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Effects

// The one elevation used by floating overlay chrome: a soft downward shadow that separates
// capsules and discs from the picture. Apply with `layer.enabled: true; layer.effect:
// OverlayShadowEffect { }`.
MultiEffect {
    property bool elevated: false

    shadowEnabled:          true
    shadowColor:            "#000000"
    shadowOpacity:          elevated ? 0.6 : 0.45
    shadowBlur:             elevated ? 1.0 : 0.7
    shadowHorizontalOffset: 0
    shadowVerticalOffset:   elevated ? 9 : 3

    Behavior on shadowOpacity        { NumberAnimation { duration: 150 } }
    Behavior on shadowBlur           { NumberAnimation { duration: 150 } }
    Behavior on shadowVerticalOffset { NumberAnimation { duration: 150 } }
}
