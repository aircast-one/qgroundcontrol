/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.Controls

Item {
    id:     root
    width:  100
    height: 100

    readonly property var resolvedInsideContent: OverlayBackdrop.forItem(insideContent)
    readonly property var resolvedInsideFull:    OverlayBackdrop.forItem(insideFull)
    readonly property var resolvedOutside:       OverlayBackdrop.forItem(outside)
    readonly property var resolvedNothing:       OverlayBackdrop.forItem(null)

    readonly property var contentBackdropItem:   contentBackdrop
    readonly property var fullBackdropItem:      fullBackdrop

    Item {
        id: fullSource

        Item {
            id: contentSource

            Item { id: insideContent }
        }

        Item { id: insideFull }
    }

    Item { id: outside }

    Item { id: contentBackdrop }
    Item { id: fullBackdrop }

    Component.onCompleted: {
        OverlayBackdrop.contentSource   = contentSource
        OverlayBackdrop.contentBackdrop = contentBackdrop
        OverlayBackdrop.fullSource      = fullSource
        OverlayBackdrop.fullBackdrop    = fullBackdrop
    }

    function unregister() {
        OverlayBackdrop.contentSource   = null
        OverlayBackdrop.contentBackdrop = null
        OverlayBackdrop.fullSource      = null
        OverlayBackdrop.fullBackdrop    = null
    }
}
