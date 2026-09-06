/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "CameraControlTest.h"

#include <algorithm>
#include "QuickInteractionTestHelpers.h"
#include "FlyViewSettings.h"
#include "VideoSettings.h"
#include "SettingsManager.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>
#include <QtQuick/QQuickItem>
static bool hasVisibleItemWithText(QQuickItem* item, const QString& text)
{
    if (!item->isVisible()) {
        return false;
    }
    if (item->property("text").toString() == text) {
        return true;
    }
    const QList<QQuickItem*> children = item->childItems();
    for (QQuickItem* child : children) {
        if (hasVisibleItemWithText(child, text)) {
            return true;
        }
    }
    return false;
}

static QQuickItem* findVisibleItemWithProperty(QQuickItem* item, const char* name, const QVariant& value)
{
    if (item->isVisible() && item->property(name).isValid() && item->property(name) == value) {
        return item;
    }
    const QList<QQuickItem*> children = item->childItems();
    for (QQuickItem* child : children) {
        if (QQuickItem* found = findVisibleItemWithProperty(child, name, value)) {
            return found;
        }
    }
    return nullptr;
}

class ChannelMappingScope
{
public:
    ChannelMappingScope(int tilt, int pan, int zoom, int light, int record = 0)
        : _settings(SettingsManager::instance()->flyViewSettings())
        , _tilt(_settings->gimbalTiltChannel()->rawValue())
        , _pan(_settings->gimbalPanChannel()->rawValue())
        , _zoom(_settings->cameraZoomChannel()->rawValue())
        , _light(_settings->cameraLightChannel()->rawValue())
        , _record(_settings->cameraRecordChannel()->rawValue())
    {
        _settings->gimbalTiltChannel()->setRawValue(tilt);
        _settings->gimbalPanChannel()->setRawValue(pan);
        _settings->cameraZoomChannel()->setRawValue(zoom);
        _settings->cameraLightChannel()->setRawValue(light);
        _settings->cameraRecordChannel()->setRawValue(record);
    }

    ~ChannelMappingScope()
    {
        _settings->gimbalTiltChannel()->setRawValue(_tilt);
        _settings->gimbalPanChannel()->setRawValue(_pan);
        _settings->cameraZoomChannel()->setRawValue(_zoom);
        _settings->cameraLightChannel()->setRawValue(_light);
        _settings->cameraRecordChannel()->setRawValue(_record);
    }

private:
    FlyViewSettings* const _settings;
    const QVariant _tilt;
    const QVariant _pan;
    const QVariant _zoom;
    const QVariant _light;
    const QVariant _record;
};

static bool loadLayer(QQuickView& view, QQmlPropertyMap& globals)
{
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    return loadTestView(view, QStringLiteral("qrc:/unittest/CameraControlLayerTest.qml"));
}
static void restoreRcControls(FlyViewSettings* const settings, const QVariant& saved)
{
    settings->rcControls()->setRawValue(saved);
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
}

void CameraControlTest::_unmappedChannelsShowNoControls()
{
    ChannelMappingScope mapping(0, 0, 0, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QVERIFY(!findVisibleItemWithProperty(view.rootObject(), "icon", QVariant(QStringLiteral("/qmlimages/CameraTilt.svg"))));
    QVERIFY(!findVisibleItemWithProperty(view.rootObject(), "icon", QVariant(QStringLiteral("/qmlimages/CameraZoom.svg"))));
    QVERIFY(!findVisibleItemWithProperty(view.rootObject(), "icon", QVariant(QStringLiteral("/qmlimages/CameraLight.svg"))));
}

void CameraControlTest::_mappedChannelsRevealTheirControls()
{
    ChannelMappingScope mapping(13, 14, 10, 9);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QVERIFY(findVisibleItemWithProperty(view.rootObject(), "icon", QVariant(QStringLiteral("/qmlimages/CameraTilt.svg"))));
    QVERIFY(findVisibleItemWithProperty(view.rootObject(), "icon", QVariant(QStringLiteral("/qmlimages/CameraZoom.svg"))));
    QVERIFY(findVisibleItemWithProperty(view.rootObject(), "icon", QVariant(QStringLiteral("/qmlimages/CameraLight.svg"))));
}
void CameraControlTest::_aimingIsIncrementalAndClamped()
{
    ChannelMappingScope mapping(13, 14, 0, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);

    QCOMPARE(layer->property("_tilt").toInt(), 1500);

    QVERIFY(QMetaObject::invokeMethod(layer, "nudgeTilt", Q_ARG(QVariant, QVariant(120))));
    QCOMPARE(layer->property("_tilt").toInt(), 1620);

    QVERIFY(QMetaObject::invokeMethod(layer, "nudgeTilt", Q_ARG(QVariant, QVariant(-40))));
    QCOMPARE(layer->property("_tilt").toInt(), 1580);

    QVERIFY(QMetaObject::invokeMethod(layer, "nudgeTilt", Q_ARG(QVariant, QVariant(9999))));
    QCOMPARE(layer->property("_tilt").toInt(), 2000);

    QVERIFY(QMetaObject::invokeMethod(layer, "nudgePan", Q_ARG(QVariant, QVariant(-9999))));
    QCOMPARE(layer->property("_pan").toInt(), 1000);

    QVERIFY(QMetaObject::invokeMethod(layer, "recenterGimbal"));
    QCOMPARE(layer->property("_tilt").toInt(), 1500);
    QCOMPARE(layer->property("_pan").toInt(), 1500);
}

void CameraControlTest::_settingsExposeTheChannelMapping()
{
    QQmlPropertyMap globals;
    globals.insert(QStringLiteral("activeVehicle"), QVariant());

    QQuickView view;
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/CameraControlSettingsTest.qml")));

    QVERIFY(hasVisibleItemWithText(view.rootObject(), QStringLiteral("Gimbal tilt channel")));
    QVERIFY(hasVisibleItemWithText(view.rootObject(), QStringLiteral("Camera zoom channel")));
    QVERIFY(hasVisibleItemWithText(view.rootObject(), QStringLiteral("Add Control")));
}

void CameraControlTest::_settingsFlagConflictingChannels()
{
    ChannelMappingScope mapping(0, 0, 10, 0);
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant saved = settings->rcControls()->rawValue();
    settings->rcControls()->setRawValue(QStringLiteral(
        R"([{"label":"Spray","channel":7,"type":"button"},{"label":"","channel":10,"type":"slider"}])"));

    QQmlPropertyMap globals;
    globals.insert(QStringLiteral("activeVehicle"), QVariant());

    QQuickView view;
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/CameraControlSettingsTest.qml")));

    QQuickItem* const clean = findItemByName(view.rootObject(), QStringLiteral("rcControlConflict0"));
    QVERIFY(clean);
    QVERIFY(!clean->isVisible());

    QQuickItem* const clash = findItemByName(view.rootObject(), QStringLiteral("rcControlConflict1"));
    QVERIFY(clash);
    QVERIFY(clash->isVisible());
    QVERIFY(clash->property("text").toString().contains(QStringLiteral("Camera zoom")));

    QQuickItem* const group = findItemByName(view.rootObject(), QStringLiteral("rcControlsGroup"));
    QVERIFY(group);
    QVariant firstFree;
    QVERIFY(QMetaObject::invokeMethod(group, "_firstFreeChannel", Q_RETURN_ARG(QVariant, firstFree)));
    QCOMPARE(firstFree.toInt(), 1);

    restoreRcControls(settings, saved);
}

void CameraControlTest::_customRcControlsAppearForTheirChannels()
{
    ChannelMappingScope mapping(0, 0, 10, 0);
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant saved = settings->rcControls()->rawValue();
    settings->rcControls()->setRawValue(QStringLiteral(
        R"([{"label":"Spray","channel":7,"type":"button"},)"
        R"({"label":"Focus","channel":8,"type":"slider"},)"
        R"({"label":"Unmapped","channel":0,"type":"slider"},)"
        R"({"label":"Duplicate","channel":8,"type":"button"},)"
        R"({"label":"Zoom clash","channel":10,"type":"button"}])"));

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const button = findItemByName(view.rootObject(), QStringLiteral("rcControlButton7"));
    QVERIFY(button);
    QVERIFY(button->isVisible());
    QVERIFY(hasVisibleItemWithText(button->parentItem(), QStringLiteral("Spray")));
    QVERIFY(!button->property("checked").toBool());
    QVERIFY(QMetaObject::invokeMethod(button, "clicked"));
    QVERIFY(button->property("checked").toBool());

    QQuickItem* const slider = findItemByName(view.rootObject(), QStringLiteral("rcControlSlider8"));
    QVERIFY(slider);
    QVERIFY(slider->isVisible());
    QCOMPARE(slider->property("readout").toString(), QStringLiteral("Focus"));
    QCOMPARE(slider->property("from").toReal(), 1000.0);
    QCOMPARE(slider->property("to").toReal(), 2000.0);
    QCOMPARE(slider->property("value").toReal(), 1500.0);

    QVERIFY(!findItemByName(view.rootObject(), QStringLiteral("rcControlSlider0")));
    QVERIFY(!findItemByName(view.rootObject(), QStringLiteral("rcControlButton8")));
    QVERIFY(!findItemByName(view.rootObject(), QStringLiteral("rcControlButton10")));

    restoreRcControls(settings, saved);
}

void CameraControlTest::_threePositionSwitchCyclesThroughPositions()
{
    ChannelMappingScope mapping(0, 0, 0, 0);
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant saved = settings->rcControls()->rawValue();
    settings->rcControls()->setRawValue(QStringLiteral(
        R"([{"label":"Mode","channel":5,"type":"switch3"}])"));

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const switch3 = findItemByName(view.rootObject(), QStringLiteral("rcControlSwitch35"));
    QVERIFY(switch3);
    QVERIFY(switch3->isVisible());
    QVERIFY(hasVisibleItemWithText(switch3->parentItem(), QStringLiteral("Mode")));
    QCOMPARE(switch3->property("currentIndex").toInt(), 1);
    QVERIFY(QMetaObject::invokeMethod(switch3, "activated", Q_ARG(int, 2)));
    QCOMPARE(switch3->property("currentIndex").toInt(), 2);

    restoreRcControls(settings, saved);
}

void CameraControlTest::_momentarySwitchTogglesCheckedOnPressAndRelease()
{
    ChannelMappingScope mapping(0, 0, 0, 0);
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant saved = settings->rcControls()->rawValue();
    settings->rcControls()->setRawValue(QStringLiteral(
        R"([{"label":"Arm","channel":6,"type":"momentary"}])"));

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const momentary = findItemByName(view.rootObject(), QStringLiteral("rcControlMomentary6"));
    QVERIFY(momentary);
    QVERIFY(momentary->isVisible());
    QVERIFY(hasVisibleItemWithText(momentary->parentItem(), QStringLiteral("Arm")));
    QVERIFY(!momentary->property("checked").toBool());
    QVERIFY(QMetaObject::invokeMethod(momentary, "pressed"));
    QVERIFY(momentary->property("checked").toBool());
    QVERIFY(QMetaObject::invokeMethod(momentary, "released"));
    QVERIFY(!momentary->property("checked").toBool());

    restoreRcControls(settings, saved);
}

void CameraControlTest::_sliderOrientationCanBeHorizontal()
{
    ChannelMappingScope mapping(0, 0, 0, 0);
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant saved = settings->rcControls()->rawValue();
    settings->rcControls()->setRawValue(QStringLiteral(
        R"([{"label":"Pan","channel":7,"type":"slider","orientation":"horizontal"}])"));

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const slider = findItemByName(view.rootObject(), QStringLiteral("rcControlSlider7"));
    QVERIFY(slider);
    QVERIFY(!slider->property("vertical").toBool());

    restoreRcControls(settings, saved);
}

void CameraControlTest::_momentarySwitchReleasesWhenEditModeInterruptsThePress()
{
    ChannelMappingScope mapping(0, 0, 0, 0);
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant saved = settings->rcControls()->rawValue();
    settings->rcControls()->setRawValue(QStringLiteral(
        R"([{"label":"Arm","channel":6,"type":"momentary"}])"));

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = findItemByName(view.rootObject(), QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QObject* const overlayRig = layer->property("overlayRig").value<QObject*>();
    QVERIFY(overlayRig);

    QQuickItem* const momentary = findItemByName(view.rootObject(), QStringLiteral("rcControlMomentary6"));
    QVERIFY(momentary);
    QVERIFY(QMetaObject::invokeMethod(momentary, "pressed"));
    QVERIFY(momentary->property("checked").toBool());

    // The same long-press that arms edit mode disables this button's own MouseArea mid-press,
    // so no onReleased signal ever fires. The channel must not stay latched at max PWM.
    overlayRig->setProperty("editMode", true);
    QVERIFY(!momentary->property("checked").toBool());

    overlayRig->setProperty("editMode", false);
    restoreRcControls(settings, saved);
}

void CameraControlTest::_overrideIndicatorLaysOutForTheToolbar()
{
    QQmlPropertyMap globals;
    globals.insert(QStringLiteral("activeVehicle"), QVariant());

    QQuickView view;
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/RcOverrideIndicatorTest.qml")));

    QQuickItem* const indicator = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("rcOverrideIndicator"));
    QVERIFY(indicator);
    QVERIFY(indicator->property("implicitWidth").toReal() > 0);
    QVERIFY(indicator->property("implicitHeight").toReal() > 0);
}
void CameraControlTest::_rcFallbackUsedWhenNoGimbalManager()
{
    ChannelMappingScope mapping(13, 14, 0, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QVERIFY(!layer->property("_hasGimbalManager").toBool());
    QVERIFY(layer->property("_hasGimbal").toBool());

    QQuickItem* const tilt = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraTiltSlider"));
    QVERIFY(tilt);
    QCOMPARE(tilt->property("readout").toString(), QString());
    QCOMPARE(tilt->property("from").toReal(), 1000.0);
    QCOMPARE(tilt->property("to").toReal(), 2000.0);
}
void CameraControlTest::_aimDragDrivesTheGimbal()
{
    ChannelMappingScope mapping(13, 14, 0, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QCOMPARE(layer->property("_tilt").toInt(), 1500);
    QCOMPARE(layer->property("_pan").toInt(), 1500);

    QQuickItem* const aimArea = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraAimArea"));
    QVERIFY(aimArea);
    QVERIFY(QMetaObject::invokeMethod(aimArea, "aimed", Q_ARG(double, 0.25), Q_ARG(double, 0.25)));
    QCOMPARE(layer->property("_pan").toInt(), 1750);
    QCOMPARE(layer->property("_tilt").toInt(), 1750);
    QVERIFY(QMetaObject::invokeMethod(aimArea, "aimed", Q_ARG(double, -0.5), Q_ARG(double, -0.5)));
    QCOMPARE(layer->property("_pan").toInt(), 1250);
    QCOMPARE(layer->property("_tilt").toInt(), 1250);
}
void CameraControlTest::_shutterAppearsForAMappedRecordChannel()
{
    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const QVariant savedSource = videoSettings->videoSource()->rawValue();
    videoSettings->videoSource()->setRawValue(videoSettings->disabledVideoSource());

    {
        ChannelMappingScope mapping(0, 0, 0, 0, 0);

        QQmlPropertyMap globals;
        QQuickView view;
        QVERIFY(loadLayer(view, globals));
        QQuickItem* const shutter = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraShutterButton"));
        QVERIFY(shutter);
        QVERIFY(!shutter->isVisible());
    }

    ChannelMappingScope mapping(0, 0, 0, 0, 11);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const shutter = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraShutterButton"));
    QVERIFY(shutter);
    QVERIFY(shutter->isVisible());
    QVERIFY(!shutter->property("recording").toBool());

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);

    QVERIFY(QMetaObject::invokeMethod(layer, "toggleRecording"));
    QVERIFY(layer->property("_recording").toBool());
    QVERIFY(shutter->property("recording").toBool());

    QVERIFY(QMetaObject::invokeMethod(layer, "toggleRecording"));
    QVERIFY(!layer->property("_recording").toBool());

    videoSettings->videoSource()->setRawValue(savedSource);
}
void CameraControlTest::_pinchAndSliderShareOneZoomValue()
{
    ChannelMappingScope mapping(0, 0, 10, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QQuickItem* const zoom = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraZoomSlider"));
    QVERIFY(zoom);

    QCOMPARE(layer->property("_zoom").toInt(), 1500);
    QCOMPARE(zoom->property("value").toReal(), 1500.0);

    QQuickItem* const aimArea = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraAimArea"));
    QVERIFY(aimArea);
    QVERIFY(QMetaObject::invokeMethod(aimArea, "zoomed", Q_ARG(double, 0.2)));
    QCOMPARE(layer->property("_zoom").toInt(), 1700);
    QCOMPARE(zoom->property("value").toReal(), 1700.0);
    QVERIFY(QMetaObject::invokeMethod(aimArea, "zoomed", Q_ARG(double, 5.0)));
    QCOMPARE(layer->property("_zoom").toInt(), 2000);

    QVERIFY(QMetaObject::invokeMethod(aimArea, "zoomed", Q_ARG(double, -5.0)));
    QCOMPARE(layer->property("_zoom").toInt(), 1000);
}
void CameraControlTest::_yawModeButtonOnlyWithAGimbalManager()
{
    ChannelMappingScope mapping(13, 14, 0, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QVERIFY(!layer->property("_hasGimbalManager").toBool());
    QVERIFY(!layer->property("_yawLocked").toBool());

    QQuickItem* const mode = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraYawModeButton"));
    QVERIFY(mode);
    QVERIFY(!mode->isVisible());
    QQuickItem* const recenter = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraRecenterButton"));
    QVERIFY(recenter);
    QVERIFY(recenter->isVisible());
}
void CameraControlTest::_shutterFollowsTheRecordChannelWhenTheStreamIsDead()
{
    VideoSettings* const videoSettings = SettingsManager::instance()->videoSettings();
    const QVariant savedSource = videoSettings->videoSource()->rawValue();
    videoSettings->videoSource()->setRawValue(videoSettings->rtspVideoSource());

    ChannelMappingScope mapping(0, 0, 0, 0, 11);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QVERIFY(!layer->property("_recording").toBool());

    QVERIFY(QMetaObject::invokeMethod(layer, "toggleRecording"));
    QVERIFY(layer->property("_recording").toBool());

    QQuickItem* const shutter = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraShutterButton"));
    QVERIFY(shutter);
    QVERIFY(shutter->property("recording").toBool());

    QVERIFY(QMetaObject::invokeMethod(layer, "toggleRecording"));
    QVERIFY(!layer->property("_recording").toBool());

    videoSettings->videoSource()->setRawValue(savedSource);
}
void CameraControlTest::_aimAreaMustNotHoldOntoTouchPoints()
{
    ChannelMappingScope mapping(13, 14, 10, 0);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const aimArea = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraAimArea"));
    QVERIFY(aimArea);
    QVERIFY(!aimArea->property("preventStealing").toBool());
    const QObjectList children = aimArea->children();
    const bool hasPinchHandler = std::any_of(children.begin(), children.end(), [](const QObject* child) {
        return QLatin1String(child->metaObject()->className()).startsWith(QLatin1String("QQuickPinchHandler"));
    });
    QVERIFY(hasPinchHandler);
}
void CameraControlTest::_joystickSharingOneChannelForBothAxesIsRejected()
{
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    const QVariant savedTilt = settings->gimbalTiltChannel()->rawValue();
    const QVariant savedPan  = settings->gimbalPanChannel()->rawValue();

    settings->gimbalTiltChannel()->setRawValue(13);
    settings->gimbalPanChannel()->setRawValue(13);

    QQmlPropertyMap globals;
    QQuickView view;
    QVERIFY(loadLayer(view, globals));

    QQuickItem* const layer = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("cameraControlLayer"));
    QVERIFY(layer);
    QCOMPARE(layer->property("_tilt").toInt(), 1500);
    QVERIFY(QMetaObject::invokeMethod(layer, "aim",
                                      Q_ARG(QVariant, QVariant(0.25)), Q_ARG(QVariant, QVariant(0.25))));
    QCOMPARE(layer->property("_panChannel").toInt(), 0);
    QCOMPARE(layer->property("_tiltChannel").toInt(), 13);
    QVERIFY(layer->property("_tilt").toInt() > 1500);

    settings->gimbalTiltChannel()->setRawValue(savedTilt);
    settings->gimbalPanChannel()->setRawValue(savedPan);
}
