/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Window

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.Palette

Item {
    id:         _root
    width:      parent ? Math.min(Math.max(_pipSize, parent.width * _minSize), parent.width * _maxSize) : _pipSize
    height:     width * (9/16)
    visible:    item2 && item2.pipState.state !== item2.pipState.windowState && show

    property var    item1:                  null    // Required
    property var    item2:                  null    // Optional, may come and go
    property string item1IsFullSettingsKey          // Settings key to save whether item1 was saved in full mode
    property bool   show:                   true
    property real   margin:                 0

    // Optional action button rendered above the pip (e.g. switch video source). Click is
    // consumed so it does not trigger the pip swap.
    property bool   showActionButton:       false
    property string actionButtonText:       ""
    signal          actionButtonClicked()

    readonly property string _pipExpandedSettingsKey: "IsPIPVisible"
    readonly property string _pipSizeSettingsKey:     "PIPSize"

    readonly property alias hasCustomPosition: dragPosition.hasCustomPosition

    DragToPosition {
        id:                 dragPosition
        target:             _root
        settingsKeyPrefix:  "PIP"
        defaultX:           _root.margin
        defaultY:           _root.parent ? _root.parent.height - _root.height - _root.margin : 0
    }

    property var    _fullItem
    property var    _pipOrWindowItem
    property alias  _windowContentItem: window.contentItem
    property alias  _pipContentItem:    pipContent
    property bool   _isExpanded:        true
    property real   _pipSize:           parent ? parent.width * 0.2 : 0
    property real   _maxSize:           0.75                // Percentage of parent control size
    property real   _minSize:           0.10

    Component.onCompleted: {
        var savedSize = parseFloat(QGroundControl.loadGlobalSetting(_pipSizeSettingsKey, "0"))
        if (savedSize > 0) {
            _pipSize = savedSize
        }
        _initForItems()
    }

    onItem2Changed: _initForItems()

    function showWindow() {
        window.width = _root.width
        window.height = _root.height
        window.show()
    }

    function _initForItems() {
        var item1IsFull = QGroundControl.loadBoolGlobalSetting(item1IsFullSettingsKey, true)
        if (item1 && item2) {
            item1.pipState.state = item1IsFull ? item1.pipState.fullState : item1.pipState.pipState
            item2.pipState.state = item1IsFull ? item2.pipState.pipState : item2.pipState.fullState
            _fullItem = item1IsFull ? item1 : item2
            _pipOrWindowItem = item1IsFull ? item2 : item1
        } else {
            item1.pipState.state = item1.pipState.fullState
            _fullItem = item1
            _pipOrWindowItem = null
        }
        _setPipIsExpanded(QGroundControl.loadBoolGlobalSetting(_pipExpandedSettingsKey, true))
    }

    function _swapPip() {
        var item1IsFull = false
        if (item1.pipState.state === item1.pipState.fullState) {
            item1.pipState.state = item1.pipState.pipState
            item2.pipState.state = item2.pipState.fullState
            _fullItem = item2
            _pipOrWindowItem = item1
            item1IsFull = false
        } else {
            item1.pipState.state = item1.pipState.fullState
            item2.pipState.state = item2.pipState.pipState
            _fullItem = item1
            _pipOrWindowItem = item2
            item1IsFull = true
        }
        QGroundControl.saveBoolGlobalSetting(item1IsFullSettingsKey, item1IsFull)
    }

    function _setPipIsExpanded(isExpanded) {
        QGroundControl.saveBoolGlobalSetting(_pipExpandedSettingsKey, isExpanded)
        _isExpanded = isExpanded
    }

    Window {
        id:         window
        visible:    false
        onClosing: {
            var item = contentItem.children[0]
            if (item) {
                item.pipState.windowAboutToClose()
                item.pipState.state = item.pipState.pipState
            }
        }
    }

    Item {
        id:             pipContent
        anchors.fill:   parent
        visible:        _isExpanded
        clip:           true
    }

    MouseArea {
        id:             pipMouseArea
        anchors.fill:   parent
        enabled:        _isExpanded
        preventStealing: true
        hoverEnabled:   true
        cursorShape:    drag.active ? Qt.ClosedHandCursor : Qt.ArrowCursor
        drag.target:    _root
        drag.minimumX:  0
        drag.minimumY:  0
        drag.maximumX:  _root.parent ? _root.parent.width - _root.width : 0
        drag.maximumY:  _root.parent ? _root.parent.height - _root.height : 0

        property bool dragged: false

        onPressed:          dragged = false
        onPositionChanged:  { if (drag.active) dragged = true }
        onReleased:         { if (dragged) dragPosition.commit() }
        onCanceled:         { if (dragged) dragPosition.commit() }
        onClicked:          { if (!dragged) _swapPip() }
    }

    ResizeHandle {
        target:      _root
        enabled:     _isExpanded
        iconVisible: _isExpanded && (ScreenTools.isMobile || pipMouseArea.containsMouse)
        onResized:   (newWidth) => { _pipSize = newWidth }
        onCommitted: {
            _pipSize = _root.width
            QGroundControl.saveGlobalSetting(_root._pipSizeSettingsKey, _root.width.toString())
            // A resize alone must not turn the default position into a custom one; only
            // update the stored spot if the user already dragged.
            if (dragPosition.hasCustomPosition) {
                dragPosition.commit()
            } else {
                dragPosition.rebind()
            }
        }
    }

    // Pip to Window
    Image {
        id:             popupPIP
        source:         "/qmlimages/PiP.svg"
        mipmap:         true
        fillMode:       Image.PreserveAspectFit
        anchors.left:   parent.left
        anchors.top:    parent.top
        visible:        _isExpanded && !ScreenTools.isMobile && pipMouseArea.containsMouse
        height:         ScreenTools.defaultFontPixelHeight * 2.5
        width:          ScreenTools.defaultFontPixelHeight * 2.5
        sourceSize.height:  height

        MouseArea {
            anchors.fill:   parent
            onClicked:      _pipOrWindowItem.pipState.state = _pipOrWindowItem.pipState.windowState
        }
    }

    Image {
        id:             hidePIP
        source:         "/qmlimages/pipHide.svg"
        mipmap:         true
        fillMode:       Image.PreserveAspectFit
        anchors.left:   parent.left
        anchors.bottom: parent.bottom
        visible:        _isExpanded && (ScreenTools.isMobile || pipMouseArea.containsMouse)
        height:         ScreenTools.defaultFontPixelHeight * 2.5
        width:          ScreenTools.defaultFontPixelHeight * 2.5
        sourceSize.height:  height
        MouseArea {
            anchors.fill:   parent
            onClicked:      _root._setPipIsExpanded(false)
        }
    }

    Rectangle {
        id:                     showPip
        anchors.left :          parent.left
        anchors.bottom:         parent.bottom
        height:                 ScreenTools.defaultFontPixelHeight * 2
        width:                  ScreenTools.defaultFontPixelHeight * 2
        radius:                 ScreenTools.defaultFontPixelHeight / 3
        visible:                !_isExpanded
        color:                  _fullItem.pipState.isDark ? Qt.rgba(0,0,0,0.75) : Qt.rgba(0,0,0,0.5)
        Image {
            width:              parent.width  * 0.75
            height:             parent.height * 0.75
            sourceSize.height:  height
            source:             "/res/buttonRight.svg"
            mipmap:             true
            fillMode:           Image.PreserveAspectFit
            anchors.verticalCenter:     parent.verticalCenter
            anchors.horizontalCenter:   parent.horizontalCenter
        }
        MouseArea {
            anchors.fill:   parent
            onClicked:      _root._setPipIsExpanded(true)
        }
    }

    // Optional action button (e.g. switch video source). Sits above pipMouseArea so its
    // click is handled here instead of swapping the pip.
    CameraSwitchButton {
        id:                     actionButton
        anchors.right:          parent.right
        anchors.bottom:         parent.bottom
        anchors.margins:        ScreenTools.defaultFontPixelHeight / 3
        visible:                _isExpanded && _root.showActionButton
        text:                   _root.actionButtonText
        onClicked:              _root.actionButtonClicked()
    }
}
