#include "VideoCameraSwitchTest.h"
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

class ThreeCameraFixture
{
public:
    ThreeCameraFixture()
        : _settings(SettingsManager::instance()->videoSettings())
        , _savedCameras(_settings->cameras()->rawValue())
        , _savedActive(_settings->activeVideoSource()->rawValue())
        , _savedMultiView(_settings->multiViewEnabled()->rawValue())
    {
        QJsonArray cameras;
        cameras.append(QJsonObject{{"name", "cam1"}, {"source", VideoSettings::videoSourceRTSP}, {"url", "rtsp://zero"}});
        cameras.append(QJsonObject{{"name", "cam2"}, {"source", VideoSettings::videoSourceRTSP}, {"url", "rtsp://one"}});
        cameras.append(QJsonObject{{"name", "cam3"}, {"source", VideoSettings::videoSourceRTSP}, {"url", "rtsp://two"}});
        _settings->cameras()->setRawValue(QString::fromUtf8(QJsonDocument(cameras).toJson(QJsonDocument::Compact)));
        _settings->multiViewEnabled()->setRawValue(true);
        _settings->activeVideoSource()->setRawValue(0);
    }

    ~ThreeCameraFixture()
    {
        _settings->cameras()->setRawValue(_savedCameras);
        _settings->activeVideoSource()->setRawValue(_savedActive);
        _settings->multiViewEnabled()->setRawValue(_savedMultiView);
    }

    VideoSettings *settings() { return _settings; }

private:
    VideoSettings *_settings = nullptr;
    QVariant _savedCameras;
    QVariant _savedActive;
    QVariant _savedMultiView;
};

}

void VideoCameraSwitchTest::_cameraToReceiverPinning()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    const StubVideoReceiver main(QStringLiteral("videoContent"));
    const StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    const StubVideoReceiver extra1(QStringLiteral("extraVideo1"));
    const StubVideoReceiver extra2(QStringLiteral("extraVideo2"));

    for (int active = 0; active < 3; ++active) {
        fixture.settings()->activeVideoSource()->setRawValue(active);
        QCOMPARE(vm->_cameraIndexForReceiver(&main), 0);
        QCOMPARE(vm->_cameraIndexForReceiver(&extra0), 1);
        QCOMPARE(vm->_cameraIndexForReceiver(&extra1), 2);
        QCOMPARE(vm->_cameraIndexForReceiver(&extra2), -1);
    }
}

void VideoCameraSwitchTest::_multiViewOffGatesInactiveCameras()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    const StubVideoReceiver main(QStringLiteral("videoContent"));
    const StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    const StubVideoReceiver extra1(QStringLiteral("extraVideo1"));

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    fixture.settings()->activeVideoSource()->setRawValue(1);

    QCOMPARE(vm->_cameraIndexForReceiver(&main), -1);
    QCOMPARE(vm->_cameraIndexForReceiver(&extra0), 1);
    QCOMPARE(vm->_cameraIndexForReceiver(&extra1), -1);
}

void VideoCameraSwitchTest::_widgetRoles()
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

    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(vm->_widgetForCamera(1), &mainItem);
    QCOMPARE(vm->_widgetForCamera(0), &tile0);
    QCOMPARE(vm->_widgetForCamera(2), &tile1);
    QCOMPARE(vm->_widgetForCamera(-1), nullptr);

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(vm->_widgetForCamera(0), &mainItem);
    QCOMPARE(vm->_widgetForCamera(1), &tile0);
    QCOMPARE(vm->_widgetForCamera(2), &tile1);

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(vm->_widgetForCamera(0), &mainItem);
    QCOMPARE(vm->_widgetForCamera(1), nullptr);

    vm->_mainWidget = savedMain;
    vm->_tileWidgets = savedTiles;
}

void VideoCameraSwitchTest::_tileCameraNumbers()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

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

void VideoCameraSwitchTest::_urlWhitespaceIsTrimmed()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();

    QJsonArray cameras;
    cameras.append(QJsonObject{{"name", "front"}, {"source", VideoSettings::videoSourceRTSP}, {"url", " rtsp://192.168.0.10:8554/H264Video "}});
    cameras.append(QJsonObject{{"name", "hdmi"}, {"source", VideoSettings::videoSourceRTSP}, {"url", " rtsp://192.168.0.10:8554/H264Video1"}});
    settings->cameras()->setRawValue(QString::fromUtf8(QJsonDocument(cameras).toJson(QJsonDocument::Compact)));

    QCOMPARE(settings->videoUrlAt(0), QStringLiteral("rtsp://192.168.0.10:8554/H264Video"));
    QCOMPARE(settings->videoUrlAt(1), QStringLiteral("rtsp://192.168.0.10:8554/H264Video1"));
}

void VideoCameraSwitchTest::_currentCameraFallsBackToTheFirstUsable()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    settings->storeCameras(QJsonArray{
        VideoSettings::camera(QStringLiteral("solo"), QString::fromUtf8(VideoSettings::videoSource3DRSolo), QString()),
        VideoSettings::camera(QStringLiteral("odd"), QStringLiteral("Not A Source"), QString()),
        VideoSettings::camera(QStringLiteral("rtsp"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://two")),
    }, 0);

    QVERIFY(!settings->sourceUsable(0));
    QVERIFY(!settings->sourceUsable(1));
    QCOMPARE(settings->currentIndex(), 2);
    QCOMPARE(settings->switchableIndices(), QList<int>{2});
    QCOMPARE(settings->storedActiveSourceName(), QString::fromUtf8(VideoSettings::videoSource3DRSolo));

    settings->storeCameras(QJsonArray{VideoSettings::camera(QStringLiteral("solo"), QString::fromUtf8(VideoSettings::videoSource3DRSolo), QString())}, 0);
    QCOMPARE(settings->currentIndex(), 0);
    QVERIFY(!settings->streamConfigured());
}

void VideoCameraSwitchTest::_adoptingReplacesTheSameCamera()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();

    settings->adoptCamera(QStringLiteral("cam2"), QString::fromUtf8(VideoSettings::videoSourceWebRTC), QStringLiteral("http://x/whep"));
    QCOMPARE(settings->videoSourceCount(), 3);
    QCOMPARE(settings->videoSourceNameAt(1), QString::fromUtf8(VideoSettings::videoSourceWebRTC));
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 1);

    settings->adoptCamera(QString(), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://new"));
    settings->adoptCamera(QString(), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://new"));
    QCOMPARE(settings->videoSourceCount(), 4);
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 3);
}

void VideoCameraSwitchTest::_deviceSetupReplacesOnlyThatHost()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    settings->storeCameras(QJsonArray{
        VideoSettings::camera(QStringLiteral("old"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://10.0.0.5:8554/front")),
        VideoSettings::camera(QStringLiteral("other"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://10.0.0.9:8554/cam")),
    }, 0);

    settings->adoptDeviceCameras(QStringLiteral("10.0.0.5"), QJsonArray{
        VideoSettings::camera(QStringLiteral("front"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://10.0.0.5:8554/front")),
        VideoSettings::camera(QStringLiteral("belly"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://10.0.0.5:8554/belly")),
    });
    QCOMPARE(settings->videoSourceCount(), 3);
    QCOMPARE(settings->cameraName(0), QStringLiteral("other"));
    QCOMPARE(settings->cameraName(1), QStringLiteral("front"));
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 1);
}

void VideoCameraSwitchTest::_listEditsKeepTheCameraOnScreen()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();

    settings->activeVideoSource()->setRawValue(2);
    settings->removeCamera(1);
    QCOMPARE(settings->videoSourceCount(), 2);
    QCOMPARE(settings->cameraName(settings->currentIndex()), QStringLiteral("cam3"));

    settings->moveCamera(1, 0);
    QCOMPARE(settings->cameraName(settings->currentIndex()), QStringLiteral("cam3"));

    settings->removeCamera(0);
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 0);

    settings->activeVideoSource()->setRawValue(3);
    const int added = settings->addCamera(QStringLiteral("new"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://new"));
    QCOMPARE(added, 1);
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 4);
    QCOMPARE(settings->cameraName(settings->currentIndex()), QStringLiteral("cam1"));

    settings->storeCameras(QJsonArray{}, 0);
    VideoManager::instance()->setActiveVideoSource(2);
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 0);

    settings->storeCameras(QJsonArray{}, 3);
    settings->addCamera(QStringLiteral("first"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://first"));
    QCOMPARE(settings->activeVideoSource()->rawValue().toInt(), 0);
}

void VideoCameraSwitchTest::_droneCameraIsLiveOnly()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    const QVariant stored = settings->cameras()->rawValue();

    QVERIFY(settings->setDroneCameras(QJsonArray{VideoSettings::camera(QStringLiteral("SIYI"), QString::fromUtf8(VideoSettings::videoSourceUDPH264), QStringLiteral("0.0.0.0:5600"))}));
    QCOMPARE(settings->videoSourceCount(), 4);
    QVERIFY(settings->cameraFromDrone(3));
    QVERIFY(!settings->cameraFromDrone(2));
    QCOMPARE(VideoManager::instance()->cameraFromDrone().at(3).toBool(), true);
    QCOMPARE(settings->currentIndex(), 0);
    QCOMPARE(settings->cameras()->rawValue(), stored);
    QVERIFY(settings->switchableIndices().contains(3));

    settings->activeVideoSource()->setRawValue(3);
    QCOMPARE(settings->currentIndex(), 3);
    QVERIFY(VideoManager::instance()->autoStreamConfigured());

    QVERIFY(settings->setDroneCameras(QJsonArray{}));
    QCOMPARE(settings->videoSourceCount(), 3);
    QCOMPARE(settings->currentIndex(), 0);
}

void VideoCameraSwitchTest::_unreadableListIsLeftAlone()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    settings->cameras()->setRawValue(QStringLiteral("{not a list"));

    QVERIFY(!settings->camerasReadable());
    settings->adoptCamera(QStringLiteral("x"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://x"));
    settings->adoptDeviceCameras(QStringLiteral("10.0.0.5"), QJsonArray{VideoSettings::camera(QStringLiteral("y"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://10.0.0.5/y"))});
    QCOMPARE(settings->addCamera(QStringLiteral("z"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://z")), -1);
    settings->removeCamera(0);
    QCOMPARE(settings->cameras()->rawValue().toString(), QStringLiteral("{not a list"));
}

void VideoCameraSwitchTest::_videoStatsReadLikeTheWatchPage()
{
    QCOMPARE(VideoManager::formatVideoStats(279, 25, 1080), QStringLiteral("279 ms · 25 fps · 1080p"));
    QCOMPARE(VideoManager::formatVideoStats(-1, 30, 720), QStringLiteral("30 fps · 720p"));
    QCOMPARE(VideoManager::formatVideoStats(80, 0, 0), QStringLiteral("80 ms · 0 fps"));
}

UT_REGISTER_TEST(VideoCameraSwitchTest, TestLabel::Unit)
