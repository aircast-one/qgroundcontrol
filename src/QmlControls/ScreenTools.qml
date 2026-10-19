pragma Singleton

import QtQuick
import QtQuick.Controls
import QtQuick.Window

import QGroundControl

Item {
    id: _screenTools

    property real defaultFontPointSize:     10
    property real platformFontPointSize:    10

    readonly property real smallFontPointRatio:      ScreenToolsController.isMobile ? 0.86 : 0.75
    readonly property real mediumFontPointRatio:     1.25
    readonly property real largeFontPointRatio:      1.5

    property real defaultFontPixelHeight:   10
    property real largeFontPixelHeight:     defaultFontPixelHeight * largeFontPointRatio
    property real mediumFontPixelHeight:    defaultFontPixelHeight * mediumFontPointRatio
    property real smallFontPixelHeight:     defaultFontPixelHeight * smallFontPointRatio

    property real defaultFontPixelWidth:    10
    property real largeFontPixelWidth:      defaultFontPixelWidth * largeFontPointRatio
    property real mediumFontPixelWidth:     defaultFontPixelWidth * mediumFontPointRatio
    property real smallFontPixelWidth:      defaultFontPixelWidth * smallFontPointRatio

    readonly property real safeAreaLeft:    ScreenToolsController.safeAreaLeft
    readonly property real safeAreaTop:     ScreenToolsController.safeAreaTop
    readonly property real safeAreaRight:   ScreenToolsController.safeAreaRight
    readonly property real safeAreaBottom:  ScreenToolsController.safeAreaBottom

    property real defaultFontDescent:       0

    property real defaultDialogControlSpacing: defaultFontPixelHeight / 2

    property real smallFontPointSize:       10
    property real mediumFontPointSize:      10
    property real largeFontPointSize:       10

    property real toolbarHeight:            0

    property real realPixelDensity: {
        if(QGroundControl.corePlugin.options.devicePixelDensity != 0) {
            return QGroundControl.corePlugin.options.devicePixelDensity
        }
        if(isAndroid) {
            if((Screen.width / Screen.pixelDensity) > 300) {
                return Screen.pixelDensity * 2
            }
        }
        return Screen.pixelDensity
    }

    property real screenWidth:  ScreenToolsController.fakeMobile ? 731 : Screen.width
    property real screenHeight: ScreenToolsController.fakeMobile ? 411 : Screen.height

    property bool isAndroid:                        ScreenToolsController.isAndroid
    property bool isiOS:                            ScreenToolsController.isiOS
    property bool isMobile:                         ScreenToolsController.isMobile
    property bool isFakeMobile:                     ScreenToolsController.fakeMobile
    property bool isWindows:                        ScreenToolsController.isWindows
    property bool isDebug:                          ScreenToolsController.isDebug
    property bool isMac:                            ScreenToolsController.isMacOS
    property bool isLinux:                          ScreenToolsController.isLinux
    readonly property real tinyScreenMillimeters:   120
    readonly property real smallFontScreenMillimeters: 60

    property bool isTinyScreen:                     (Screen.width / realPixelDensity) < tinyScreenMillimeters
    property bool isShortScreen:                    ((Screen.height / realPixelDensity) < tinyScreenMillimeters) || (ScreenToolsController.isMobile && ((Screen.height / Screen.width) < 0.6))
    property bool isHugeScreen:                     (Screen.width / realPixelDensity) >= (23.5 * 25.4) // 27" monitor
    property bool isSerialAvailable:                ScreenToolsController.isSerialAvailable

    readonly property real minTouchMillimeters:     5   ///< Minimum touch size in millimeters
    readonly property real minTouchMobilePixels:    48
    readonly property real minMobileSmallPointSize: 12
    property real minTouchPixels:                   0   ///< Minimum touch size in pixels (calculated from minTouchMillimeters and realPixelDensity)

    property real implicitButtonWidth:              Math.round(defaultFontPixelWidth *  (isMobile ? 7.0 : 5.0))
    property real implicitButtonHeight:             Math.round(defaultFontPixelHeight * (isMobile ? 2.0 : 1.6))
    property real implicitCheckBoxHeight:           Math.round(defaultFontPixelHeight * (isMobile ? 1.2 : 1.0))
    property real implicitRadioButtonHeight:        implicitCheckBoxHeight
    property real implicitTextFieldWidth:           defaultFontPixelWidth * 13
    property real settingsRowHeight:                Math.round(defaultFontPixelHeight * (isMobile ? 2.6 : 2.2))
    property real implicitTextFieldHeight:          Math.round(defaultFontPixelHeight * (isMobile ? 2.0 : 1.6))
    property real implicitComboBoxHeight:           Math.round(defaultFontPixelHeight * (isMobile ? 2.0 : 1.6))
    property real implicitComboBoxWidth:            Math.round(defaultFontPixelWidth *  (isMobile ? 7.0 : 5.0))
    property real comboBoxPadding:                  defaultFontPixelWidth
    property real implicitSliderHeight:             isMobile ? Math.max(defaultFontPixelHeight, minTouchPixels) : defaultFontPixelHeight
    property real buttonBorderRadius:               Math.round(defaultFontPixelHeight * ScreenToolsController.controlRadiusRatio)
    property real defaultBorderRadius:              buttonBorderRadius

    property real checkBoxIndicatorSize:            2 * Math.floor(defaultFontPixelHeight * (isMobile ? 1.5 : 1.0) / 2) + 1
    property real radioButtonIndicatorSize:         checkBoxIndicatorSize

    readonly property string normalFontFamily:      ScreenToolsController.normalFontFamily
    readonly property string fixedFontFamily:       ScreenToolsController.fixedFontFamily
    Connections {
        target: QGroundControl.settingsManager.appSettings.appFontPointSize
        function onValueChanged() {
            _setBasePointSize(QGroundControl.settingsManager.appSettings.appFontPointSize.value)
        }
    }

    onRealPixelDensityChanged: {
        _setBasePointSize(_basePointSize)
    }

    Connections {
        target: ScreenToolsController
        function onSystemFontScaleChanged() {
            _setBasePointSize(_basePointSize)
        }
    }

    function printScreenStats() {
        console.log('ScreenTools: Screen.width: ' + Screen.width + ' Screen.height: ' + Screen.height + ' Screen.pixelDensity: ' + Screen.pixelDensity)
    }

    function mouseX() {
        return ScreenToolsController.mouseX()
    }

    function mouseY() {
        return ScreenToolsController.mouseY()
    }

    property real _basePointSize: 10

    function _setBasePointSize(pointSize) {
        _basePointSize          = pointSize
        const scaledPointSize   = pointSize * ScreenToolsController.systemFontScale
        _textMeasure.font.pointSize = scaledPointSize
        defaultFontPointSize    = scaledPointSize
        defaultFontPixelHeight  = Math.round(_textMeasure.fontHeight/2.0)*2
        defaultFontPixelWidth   = Math.round(_textMeasure.fontWidth/2.0)*2
        defaultFontDescent      = ScreenToolsController.defaultFontDescent(defaultFontPointSize)
        smallFontPointSize      = ScreenToolsController.isMobile ? Math.max(defaultFontPointSize * _screenTools.smallFontPointRatio, minMobileSmallPointSize)
                                                                 : defaultFontPointSize * _screenTools.smallFontPointRatio
        mediumFontPointSize     = defaultFontPointSize  * _screenTools.mediumFontPointRatio
        largeFontPointSize      = defaultFontPointSize  * _screenTools.largeFontPointRatio
        minTouchPixels          = Math.round(minTouchMillimeters * realPixelDensity)
        if (minTouchPixels / Screen.height > 0.15) {
            minTouchPixels      = defaultFontPixelHeight * 3
        }
        if (ScreenToolsController.isMobile) {
            minTouchPixels      = Math.max(minTouchPixels, minTouchMobilePixels)
        }
        toolbarHeight           = defaultFontPixelHeight * 3
        toolbarHeight           = toolbarHeight * QGroundControl.corePlugin.options.toolbarHeightMultiplier
    }

    Text {
        id:     _defaultFont
        text:   "X"
    }

    Text {
        id:     _textMeasure
        text:   "X"
        font.family:    normalFontFamily
        property real   fontWidth:    contentWidth
        property real   fontHeight:   contentHeight
        Component.onCompleted: {
            if(ScreenToolsController.isMobile) {
                if(ScreenToolsController.isiOS) {
                    if(ScreenToolsController.isiOS && Screen.width < 570) {
                        platformFontPointSize = 12;  // This will be lowered in a future pull
                    } else {
                        platformFontPointSize = 14;
                    }
                } else if((Screen.width / realPixelDensity) < smallFontScreenMillimeters) {
                    platformFontPointSize = 11;
                } else {
                    platformFontPointSize = 14;
                }
            } else {
                platformFontPointSize = _defaultFont.font.pointSize;
            }
            var _appFontPointSizeFact = QGroundControl.settingsManager.appSettings.appFontPointSize
            var baseSize = _appFontPointSizeFact.value
            if(baseSize < _appFontPointSizeFact.min || baseSize > _appFontPointSizeFact.max) {
                baseSize = platformFontPointSize;
                _appFontPointSizeFact.value = baseSize
            }
            _screenTools._setBasePointSize(baseSize);
        }
    }
}
