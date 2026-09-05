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
import QtQuick.Window

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.Palette

Item {
    id:         _root
    width:      widthOverride > 0 ? widthOverride : naturalWidth
    height:     width * (9/16)
    visible:    item2 && item2.pipState.state !== item2.pipState.windowState && show

    property var    item1:                  null
    property var    item2:                  null
    property string item1IsFullSettingsKey
    property Item   fullContentItem:        _root.parent
    property bool   show:                   true
    property real   widthOverride:          0
    readonly property real naturalWidth:    parent ? Math.min(Math.max(_pipSize, parent.width * _minSize), parent.width * _maxSize) : _pipSize
    property real   margin:                 0

    property bool   showActionButton:       false
    property string actionButtonText:       ""
    signal          actionButtonClicked()

    readonly property alias hasCustomPosition: dragPosition.hasCustomPosition
    readonly property alias dragToPosition:    dragPosition
    readonly property bool  expanded:          _isExpanded

    property var overlayRig: null

    readonly property bool _editMode:    overlayRig ? overlayRig.editMode : false
    readonly property bool _arrangeable: overlayRig ? overlayRig.editMode : true

    readonly property string _pipExpandedSettingsKey: "IsPIPVisible"
    readonly property string _pipSizeSettingsKey:     "PIPSize"

    property var    _fullItem
    property var    _pipOrWindowItem
    property alias  _windowContentItem: window.contentItem
    property alias  _pipContentItem:    pipContent
    property bool   _isExpanded:        true
    property real   _pipSize:           parent ? parent.width * 0.2 : 0
    property real   _maxSize:           0.75
    property real   _minSize:           0.10

    DragToPosition {
        id:                 dragPosition
        objectName:         "dragPosition"
        target:             _root
        settingsKeyPrefix:  "PIP"
        defaultX:           _root.margin
        defaultY:           _root.parent ? _root.parent.height - _root.height - _root.margin : 0
    }

    Component.onCompleted: {
        var savedSize = parseFloat(QGroundControl.loadGlobalSetting(_pipSizeSettingsKey, "0"))
        if (savedSize > 0) {
            _pipSize = savedSize
        }
        _initForItems()
    }

    onItem2Changed: _initForItems()

    function setExpanded(isExpanded) {
        _setPipIsExpanded(isExpanded)
    }

    // The stored size is the pip's own width; naturalWidth clamps it again on read so a size
    // saved on a wide screen cannot swallow a narrow one. The video tiles are sized from this,
    // so the grip resizes the whole video cluster, not just the pip.
    function _resizeTo(newWidth) {
        if (!parent) {
            return
        }
        _pipSize = Math.max(parent.width * _minSize, Math.min(newWidth, parent.width * _maxSize))
    }

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

    readonly property real _previewRadius: ScreenTools.defaultFontPixelHeight * 0.75

    readonly property int  _revealDuration: 320
    readonly property int  _revealEasing:   Easing.OutCubic
    readonly property real _collapsedScale: 0.88

    Rectangle {
        anchors.fill:       pipContent
        radius:             _previewRadius
        color:              "black"
        visible:            opacity > 0
        opacity:            _isExpanded ? 1 : 0
        scale:              _isExpanded ? 1 : _collapsedScale
        transformOrigin:    Item.BottomLeft
        layer.enabled:      true
        layer.effect:       OverlayShadowEffect { elevated: pipMouseArea.drag.active }

        Behavior on opacity { NumberAnimation { duration: _revealDuration; easing.type: _revealEasing } }
        Behavior on scale   { NumberAnimation { duration: _revealDuration; easing.type: _revealEasing } }
    }

    Item {
        id:                 pipContent
        objectName:         "pipContent"
        anchors.fill:       parent
        visible:            opacity > 0
        opacity:            _isExpanded ? 1 : 0
        scale:              _isExpanded ? 1 : _collapsedScale
        transformOrigin:    Item.BottomLeft
        clip:               true
        layer.enabled:      visible
        layer.effect:   MultiEffect {
            maskEnabled:        true
            maskSource:         pipContentMask
            maskThresholdMin:   0.5
            maskSpreadAtMin:    1.0
        }

        Behavior on opacity { NumberAnimation { duration: _revealDuration; easing.type: _revealEasing } }
        Behavior on scale   { NumberAnimation { duration: _revealDuration; easing.type: _revealEasing } }
    }

    Item {
        id:             pipContentMask
        anchors.fill:   pipContent
        layer.enabled:  true
        visible:        false

        Rectangle {
            anchors.fill:   parent
            radius:         _previewRadius
            color:          "black"
        }
    }

    OverlayEditBadge {
        rig:     _root.overlayRig
        editKey: "pipView"
    }

    JiggleAnimation {
        target:  _root
        running: _root._editMode && _root.visible && _isExpanded
        lifted:  pipMouseArea.drag.active
    }

    // Hover reveal reads this, not the MouseArea below. The grip and the popup button carry
    // handlers of their own, and a child handler taking the hover point drops the MouseArea's
    // containsMouse - which hid the control the cursor had just arrived at, which handed hover
    // back, forever. A HoverHandler on the root stays hovered over its own children.
    HoverHandler { id: pipHover }

    MouseArea {
        id:             pipMouseArea
        anchors.fill:   parent
        enabled:        _isExpanded
        preventStealing: true
        hoverEnabled:   true
        cursorShape:    drag.active ? Qt.ClosedHandCursor : Qt.ArrowCursor
        drag.target:    _root._arrangeable ? _root : null
        drag.minimumX:  0
        drag.minimumY:  0
        drag.maximumX:  _root.parent ? _root.parent.width - _root.width : 0
        drag.maximumY:  _root.parent ? _root.parent.height - _root.height : 0

        property bool dragged:      false
        // The resize grip's DragHandler doesn't stop this MouseArea from also seeing the same
        // press/release: outside edit mode drag.target is null, so `dragged` above can never go
        // true, and every release would otherwise read as a plain click and swap map/video out
        // from under a resize. Captured at press time so it isn't racing the grip's own active
        // state, which has already gone false again by the time this MouseArea's click fires.
        // Any future DragHandler-based control added inside the pip (popupPIP/pipToggle are
        // fine - they use their own nested MouseArea, which already wins over this one) needs
        // the same treatment here, or it will reproduce this exact swap-on-drag bug.
        property bool pressedOnGrip: false

        onPressed: (mouse) => {
            dragged = false
            pressedOnGrip = resizeGrip.visible && resizeGrip.contains(mapToItem(resizeGrip, mouse.x, mouse.y))
        }
        onPositionChanged:  { if (drag.active) dragged = true }
        onReleased: {
            if (dragged) {
                dragPosition.commit()
                if (_root.overlayRig) {
                    _root.overlayRig.requestReflow()
                }
            }
        }
        onCanceled:         { if (dragged) dragPosition.commit() }
        onClicked:          { if (!dragged && !pressedOnGrip && !_root._editMode) _swapPip() }

        onPressAndHold: {
            if (_root.overlayRig) {
                _root.overlayRig.editMode = true
            }
        }
    }

    Rectangle {
        id:                 popupPIP
        anchors.left:       parent.left
        anchors.top:        parent.top
        anchors.margins:    ScreenTools.defaultFontPixelHeight / 3
        width:              ScreenTools.defaultFontPixelHeight * 2.2
        height:             width
        radius:             width / 2
        color:              "transparent"
        visible:            _isExpanded && !ScreenTools.isMobile && pipHover.hovered && !_root._editMode

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.radius
        }

        QGCColoredImage {
            source:             "/InstrumentValueIcons/browser-window-new.svg"
            color:              QGroundControl.globalPalette.text
            mipmap:             true
            fillMode:           Image.PreserveAspectFit
            anchors.centerIn:   parent
            height:             parent.height * 0.5
            width:              height
            sourceSize.height:  height
        }

        MouseArea {
            anchors.fill:   parent
            onClicked:      _pipOrWindowItem.pipState.state = _pipOrWindowItem.pipState.windowState
        }
    }

    // Top-right, because that is the corner that moves: the pip is pinned to the bottom of the
    // screen, so growing it travels up and to the right. A grip on the bottom edge would sit
    // still while the box grew away from it, and would share that corner with the camera switch.
    // Hover-reveal, like a macOS PiP window, so it doesn't cost an accidental resize of the live
    // feed. Move-by-drag stays edit-mode-gated below; only this small targeted handle is live.
    Rectangle {
        id:                 resizeGrip
        objectName:         "pipResizeGrip"
        anchors.right:      parent.right
        anchors.top:        parent.top
        anchors.margins:    ScreenTools.defaultFontPixelHeight / 3
        width:              ScreenTools.defaultFontPixelHeight * 1.8
        height:             width
        radius:             width / 2
        color:              "transparent"
        scale:              resizeHandler.active ? 1.1 : 1

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.radius
        }
        // Hidden while the video rail's grid mode is driving the width: the grip would fight
        // an override it cannot change.
        visible:            _isExpanded && _root.widthOverride === 0 && (_root._editMode || pipHover.hovered || resizeHandler.active)

        Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }

        HoverHandler { cursorShape: Qt.SizeBDiagCursor }

        QGCColoredImage {
            source:             "/InstrumentValueIcons/arrow-simple-right.svg"
            color:              QGroundControl.globalPalette.text
            mipmap:             true
            fillMode:           Image.PreserveAspectFit
            anchors.centerIn:   parent
            height:             parent.height * 0.55
            width:              height
            sourceSize.height:  height
            rotation:           -45
        }

        // Measured against the scene, not the handler's own translation: the grip is anchored to
        // an edge that resizing moves, so translation would shrink as fast as the width grew and
        // the pip would oscillate under the finger.
        DragHandler {
            id:     resizeHandler
            target: null

            property real _startWidth:  0
            property real _startSceneX: 0

            onActiveChanged: {
                if (active) {
                    _startWidth  = _root.width
                    _startSceneX = centroid.scenePosition.x
                } else {
                    QGroundControl.saveGlobalSetting(_root._pipSizeSettingsKey, _root._pipSize.toString())
                }
            }

            onCentroidChanged: {
                if (active) {
                    _root._resizeTo(_startWidth + (centroid.scenePosition.x - _startSceneX))
                }
            }
        }
    }

    Rectangle {
        id:                 pipToggle
        objectName:         "pipToggle"
        anchors.left:       parent.left
        anchors.bottom:     parent.bottom
        anchors.margins:    ScreenTools.defaultFontPixelHeight / 3
        width:              ScreenTools.defaultFontPixelHeight * 2.2
        height:             width
        radius:             width / 2
        color:              "transparent"
        visible:            opacity > 0
        opacity:            _root._editMode ? 0
                                : !_isExpanded ? 1
                                : (ScreenTools.isMobile || pipHover.hovered) ? 1 : 0
        scale:              pipToggleMouseArea.pressed ? 0.92 : 1

        Behavior on opacity { NumberAnimation { duration: _revealDuration; easing.type: _revealEasing } }
        Behavior on scale   { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.radius
        }

        QGCColoredImage {
            objectName:         "pipToggleChevron"
            source:             "/InstrumentValueIcons/cheveron-left.svg"
            color:              QGroundControl.globalPalette.text
            mipmap:             true
            fillMode:           Image.PreserveAspectFit
            anchors.centerIn:   parent
            height:             parent.height * 0.6
            width:              height
            sourceSize.height:  height
            rotation:           _isExpanded ? 0 : 180

            Behavior on rotation { NumberAnimation { duration: _revealDuration; easing.type: _revealEasing } }
        }

        MouseArea {
            id:             pipToggleMouseArea
            anchors.fill:   parent
            onClicked:      _root._setPipIsExpanded(!_isExpanded)
        }
    }

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
