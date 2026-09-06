/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

// A scene with the shapes the /ui endpoints are meant to describe: a named item to read and
// write, an unnamed one that only the tree's all=1 mode can see, two overlapping items so a hit
// test has a stack to report, and a value that changes on its own so a watch has something to
// stream.
Item {
    id:     root
    width:  400
    height: 300

    Rectangle {
        objectName: "probeTarget"
        x:          40
        y:          50
        width:      120
        height:     80
        color:      "steelblue"
        opacity:    0.5

        // Kept equal to counter/10 by the ticker below, from the very first sample: a watcher
        // asserting the two agree would otherwise trip over the initial state rather than over
        // anything the stream did.
        property real  reading:  0
        property int   counter:  0
        property bool  flagged:  false
        property string caption: "hello"
    }

    // No objectName on purpose: it must be invisible to a plain tree walk and visible to all=1.
    Rectangle {
        x:      60
        y:      70
        width:  40
        height: 30
        color:  "orange"
    }

    Rectangle {
        objectName: "probeHidden"
        x:          200
        y:          50
        width:      60
        height:     60
        visible:    false
        color:      "red"
    }

    Timer {
        objectName:  "probeTicker"
        running:     true
        repeat:      true
        interval:    10
        onTriggered: {
            const target = root.children[0]
            target.counter = target.counter + 1
            target.reading = target.counter / 10
        }
    }
}
