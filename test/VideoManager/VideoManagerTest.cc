#include "VideoManagerTest.h"
#include "SettingsManager.h"
#include "VideoManager.h"
#include "VideoReceiver.h"
#include "VideoSettings.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtQuick/QQuickItem>
#include <QtTest/QTest>

namespace {

class StubVideoReceiver : public VideoReceiver
{
public:
    explicit StubVideoReceiver(const QString &name)
    {
        setName(name);
    }

    void start(uint32_t) final {}
    void stop() final {}
    void startDecoding(void *) final {}
    void stopDecoding() final {}
    void startRecording(const QString &, FILE_FORMAT) final {}
    void stopRecording() final {}
    void takeScreenshot(const QString &) final {}
};

// Two rtsp extras on top of the primary camera -> three switchable cameras.
class ThreeCameraFixture
{
public:
    ThreeCameraFixture()
        : _settings(SettingsManager::instance()->videoSettings())
        , _savedExtras(_settings->extraVideoSources()->rawValue())
        , _savedActive(_settings->activeVideoSource()->rawValue())
        , _savedMultiView(_settings->multiViewEnabled()->rawValue())
    {
        QJsonArray extras;
        extras.append(QJsonObject{{"name", "cam2"}, {"source", VideoSettings::videoSourceRTSP}, {"url", "rtsp://one"}});
        extras.append(QJsonObject{{"name", "cam3"}, {"source", VideoSettings::videoSourceRTSP}, {"url", "rtsp://two"}});
        _settings->extraVideoSources()->setRawValue(QString::fromUtf8(QJsonDocument(extras).toJson(QJsonDocument::Compact)));
        _settings->multiViewEnabled()->setRawValue(true);
        _settings->activeVideoSource()->setRawValue(0);
    }

    ~ThreeCameraFixture()
    {
        _settings->extraVideoSources()->setRawValue(_savedExtras);
        _settings->activeVideoSource()->setRawValue(_savedActive);
        _settings->multiViewEnabled()->setRawValue(_savedMultiView);
    }

    VideoSettings *settings() { return _settings; }

private:
    VideoSettings *_settings = nullptr;
    QVariant _savedExtras;
    QVariant _savedActive;
    QVariant _savedMultiView;
};

} // namespace

void VideoManagerTest::_cameraToReceiverPinning()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    const StubVideoReceiver main(QStringLiteral("videoContent"));
    const StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    const StubVideoReceiver extra1(QStringLiteral("extraVideo1"));
    const StubVideoReceiver extra2(QStringLiteral("extraVideo2"));

    // The pinning must not depend on which camera is active.
    for (int active = 0; active < 3; ++active) {
        fixture.settings()->activeVideoSource()->setRawValue(active);
        QCOMPARE(vm->_cameraIndexForReceiver(&main), 0);
        QCOMPARE(vm->_cameraIndexForReceiver(&extra0), 1);
        QCOMPARE(vm->_cameraIndexForReceiver(&extra1), 2);
        QCOMPARE(vm->_cameraIndexForReceiver(&extra2), -1); // no fourth camera
    }
}

void VideoManagerTest::_multiViewOffGatesInactiveCameras()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    const StubVideoReceiver main(QStringLiteral("videoContent"));
    const StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    const StubVideoReceiver extra1(QStringLiteral("extraVideo1"));

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    fixture.settings()->activeVideoSource()->setRawValue(1);

    // Only the active camera keeps a stream; the others are gated off.
    QCOMPARE(vm->_cameraIndexForReceiver(&main), -1);
    QCOMPARE(vm->_cameraIndexForReceiver(&extra0), 1);
    QCOMPARE(vm->_cameraIndexForReceiver(&extra1), -1);
}

void VideoManagerTest::_widgetRoles()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    QQuickItem mainItem;
    QQuickItem tile0;
    QQuickItem tile1;

    const auto savedMain = vm->_mainWidget;
    const auto savedTiles = vm->_tileWidgets;
    vm->_mainWidget = &mainItem;
    vm->_tileWidgets.clear();
    vm->_tileWidgets.insert(0, &tile0);
    vm->_tileWidgets.insert(1, &tile1);

    // Active camera renders in the main widget, the others fill tiles in order.
    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(vm->_widgetForCamera(1), &mainItem);
    QCOMPARE(vm->_widgetForCamera(0), &tile0);
    QCOMPARE(vm->_widgetForCamera(2), &tile1);
    QCOMPARE(vm->_widgetForCamera(-1), nullptr);

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(vm->_widgetForCamera(0), &mainItem);
    QCOMPARE(vm->_widgetForCamera(1), &tile0);
    QCOMPARE(vm->_widgetForCamera(2), &tile1);

    // Multi-view off: the active camera keeps the main widget, others get none.
    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(vm->_widgetForCamera(0), &mainItem);
    QCOMPARE(vm->_widgetForCamera(1), nullptr);

    vm->_mainWidget = savedMain;
    vm->_tileWidgets = savedTiles;
}

void VideoManagerTest::_tileCameraNumbers()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    // Tiles list every camera except the active one, in camera order (1-based numbers).
    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(vm->tileCameraNumber(0), 1);
    QCOMPARE(vm->tileCameraNumber(1), 3);
    QCOMPARE(vm->tileCameraNumber(2), 0);

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(vm->tileCameraNumber(0), 2);
    QCOMPARE(vm->tileCameraNumber(1), 3);

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(vm->tileCameraNumber(0), 0);
}
