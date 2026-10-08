#include "VideoCameraSwitchTest.h"
#include "Fact.h"
#include "SettingsManager.h"
#include "VideoManager.h"
#include "VideoReceiver.h"
#include "VideoSettings.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QScopeGuard>
#include <QtTest/QSignalSpy>
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
        (void) _settings->setDroneCameras(QJsonArray{});
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

void VideoCameraSwitchTest::_onlyTheActiveAndPipCamerasArePlayed()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    const StubVideoReceiver main(QStringLiteral("videoContent"));
    const StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    const StubVideoReceiver extra1(QStringLiteral("extraVideo1"));
    const StubVideoReceiver extra2(QStringLiteral("extraVideo2"));
    const auto played = [&]() {
        return QList<int>{vm->_cameraIndexForReceiver(&main), vm->_cameraIndexForReceiver(&extra0), vm->_cameraIndexForReceiver(&extra1), vm->_cameraIndexForReceiver(&extra2)};
    };

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(played(), (QList<int>{0, 1, -1, -1}));
    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(played(), (QList<int>{-1, 1, 2, -1}));
    fixture.settings()->activeVideoSource()->setRawValue(2);
    QCOMPARE(played(), (QList<int>{0, -1, 2, -1}));
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
    QQuickItem pipItem;

    const auto savedMain = vm->_mainWidget;
    const auto savedPip = vm->_pipWidget;
    vm->_mainWidget = &mainItem;
    vm->_pipWidget = &pipItem;

    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(vm->_widgetForCamera(1), &mainItem);
    QCOMPARE(vm->_widgetForCamera(2), &pipItem);
    QCOMPARE(vm->_widgetForCamera(0), nullptr);
    QCOMPARE(vm->_widgetForCamera(-1), nullptr);

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(vm->_widgetForCamera(0), &mainItem);
    QCOMPARE(vm->_widgetForCamera(1), &pipItem);
    QCOMPARE(vm->_widgetForCamera(2), nullptr);

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(vm->_widgetForCamera(0), &mainItem);
    QCOMPARE(vm->_widgetForCamera(1), nullptr);

    vm->_mainWidget = savedMain;
    vm->_pipWidget = savedPip;
}

void VideoCameraSwitchTest::_pipCameraNumbers()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(vm->pipCameraNumber(), 3);

    fixture.settings()->activeVideoSource()->setRawValue(2);
    QCOMPARE(vm->pipCameraNumber(), 1);

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(vm->pipCameraNumber(), 2);

    vm->promotePip();
    QCOMPARE(vm->activeVideoSource(), 1);

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(vm->pipCameraNumber(), 0);
    vm->promotePip();
    QCOMPARE(vm->activeVideoSource(), 1);
}

void VideoCameraSwitchTest::_nativeChannelsFollowTheActiveAndPipCameras()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();

    const StubVideoReceiver main(QStringLiteral("videoContent"));
    const StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    const StubVideoReceiver extra1(QStringLiteral("extraVideo1"));
    const auto channels = [&]() {
        return QList<int>{vm->_nativeChannelForReceiver(&main), vm->_nativeChannelForReceiver(&extra0), vm->_nativeChannelForReceiver(&extra1)};
    };

    fixture.settings()->activeVideoSource()->setRawValue(0);
    QCOMPARE(channels(), (QList<int>{0, 1, -1}));
    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(channels(), (QList<int>{-1, 0, 1}));
    fixture.settings()->activeVideoSource()->setRawValue(2);
    QCOMPARE(channels(), (QList<int>{1, -1, 0}));

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(channels(), (QList<int>{-1, -1, 0}));
}

void VideoCameraSwitchTest::_pipIsTheNextUsableCameraTheSwitchGoesTo()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    VideoManager *vm = VideoManager::instance();
    const QString rtsp = QString::fromUtf8(VideoSettings::videoSourceRTSP);

    settings->storeCameras(QJsonArray{
        VideoSettings::camera(QStringLiteral("a"), rtsp, QStringLiteral("rtsp://a")),
        VideoSettings::camera(QStringLiteral("b"), rtsp, QString()),
        VideoSettings::camera(QStringLiteral("c"), rtsp, QStringLiteral("rtsp://c")),
    }, 0);
    QCOMPARE(settings->pipCameraIndex(), 2);
    QCOMPARE(vm->pipCameraNumber(), 3);

    vm->switchActiveVideoSource();
    QCOMPARE(vm->activeVideoSource(), 2);
    QCOMPARE(settings->pipCameraIndex(), 0);

    settings->storeCameras(QJsonArray{VideoSettings::camera(QStringLiteral("a"), rtsp, QStringLiteral("rtsp://a"))}, 0);
    QCOMPARE(settings->pipCameraIndex(), -1);
    QCOMPARE(vm->pipCameraNumber(), 0);
    vm->switchActiveVideoSource();
    QCOMPARE(vm->activeVideoSource(), 0);
}

void VideoCameraSwitchTest::_cameraSignalsFollowTheReceivers()
{
    ThreeCameraFixture fixture;
    VideoManager *vm = VideoManager::instance();
    Fact *const streamEnabled = fixture.settings()->streamEnabled();
    const QVariant savedStream = streamEnabled->rawValue();
    const auto savedReceivers = vm->_videoReceivers;
    const auto savedState = vm->_receiverState;
    const auto restore = qScopeGuard([&] {
        vm->_videoReceivers = savedReceivers;
        vm->_receiverState = savedState;
        streamEnabled->setRawValue(savedStream);
    });
    streamEnabled->setRawValue(true);

    StubVideoReceiver main(QStringLiteral("videoContent"));
    StubVideoReceiver extra0(QStringLiteral("extraVideo0"));
    StubVideoReceiver extra1(QStringLiteral("extraVideo1"));
    vm->_initVideoReceiver(&main, nullptr);
    vm->_initVideoReceiver(&extra0, nullptr);
    vm->_initVideoReceiver(&extra1, nullptr);

    emit main.onStartComplete(VideoReceiver::STATUS_OK);
    emit extra0.onStartComplete(VideoReceiver::STATUS_OK);
    QCOMPARE(vm->cameraSignals(), (QStringList{QStringLiteral("connecting"), QStringLiteral("connecting"), QStringLiteral("idle")}));

    emit main.decodingChanged(true);
    QCOMPARE(vm->cameraSignals().at(0), QStringLiteral("live"));

    emit extra0.onStopComplete(VideoReceiver::STATUS_OK);
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("noSignal"));
    emit extra0.onStartComplete(VideoReceiver::STATUS_OK);
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("noSignal"));
    emit extra0.streamingChanged(true);
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("noSignal"));
    emit extra0.decodingChanged(true);
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("live"));
    emit extra0.decodingChanged(false);
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("connecting"));

    emit extra0.streamingChanged(false);
    emit extra0.onStartComplete(VideoReceiver::STATUS_FAIL);
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("noSignal"));
    (void) vm->_updateVideoUri(&extra0, QStringLiteral("rtsp://elsewhere"));
    QCOMPARE(vm->cameraSignals().at(1), QStringLiteral("connecting"));

    vm->_restartVideo(&main);
    emit main.decodingChanged(false);
    emit main.onStopComplete(VideoReceiver::STATUS_OK);
    QCOMPARE(vm->cameraSignals().at(0), QStringLiteral("connecting"));

    fixture.settings()->activeVideoSource()->setRawValue(1);
    QCOMPARE(vm->cameraSignals(), (QStringList{QStringLiteral("idle"), QStringLiteral("connecting"), QStringLiteral("noSignal")}));

    QSignalSpy camerasChanged(vm, &VideoManager::camerasChanged);
    streamEnabled->setRawValue(false);
    QCOMPARE(camerasChanged.count(), 1);
    QCOMPARE(vm->cameraSignals(), (QStringList{QStringLiteral("idle"), QStringLiteral("idle"), QStringLiteral("idle")}));
    streamEnabled->setRawValue(true);

    fixture.settings()->multiViewEnabled()->setRawValue(false);
    QCOMPARE(vm->cameraSignals(), (QStringList{QStringLiteral("idle"), QStringLiteral("connecting"), QStringLiteral("idle")}));
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
    QCOMPARE(settings->addCamera(QStringLiteral("new"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://new")), QString());
    QCOMPARE(settings->storedCameraCount(), 2);
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
    QVERIFY(!settings->addCamera(QStringLiteral("z"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://z")).isEmpty());
    QVERIFY(!settings->updateCamera(0, QStringLiteral("z"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://z")).isEmpty());
    settings->removeCamera(0);
    QCOMPARE(settings->cameras()->rawValue().toString(), QStringLiteral("{not a list"));
}

void VideoCameraSwitchTest::_onlyPlayableKindsAreOffered()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    const QStringList offered = settings->offeredSources();
    QVERIFY(offered.contains(QString::fromUtf8(VideoSettings::videoSourceRTSP)));
    QVERIFY(offered.contains(QString::fromUtf8(VideoSettings::videoSourceUDPH264)));
    QVERIFY(!offered.contains(QString::fromUtf8(VideoSettings::videoSource3DRSolo)));
    QVERIFY(!offered.contains(QString::fromUtf8(VideoSettings::videoSourceParrotDiscovery)));
    QVERIFY(!offered.contains(QString::fromUtf8(VideoSettings::videoSourceYuneecMantisG)));
    QVERIFY(!offered.contains(QString::fromUtf8(VideoSettings::videoDisabled)));

    const QString unplayable = QStringLiteral("This kind of camera cannot show video in this app.");
    QCOMPARE(settings->addCamera(QStringLiteral("solo"), QString::fromUtf8(VideoSettings::videoSource3DRSolo), QString()), unplayable);
    QCOMPARE(settings->videoSourceCount(), 3);

    QCOMPARE(settings->updateCamera(0, QStringLiteral("cam1"), QString::fromUtf8(VideoSettings::videoSource3DRSolo), QString()), unplayable);
    QCOMPARE(settings->videoSourceNameAt(0), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(settings->addCamera(QStringLiteral("off"), QString::fromUtf8(VideoSettings::videoDisabled), QString()), QStringLiteral("Pick the kind of stream this camera sends."));
}

void VideoCameraSwitchTest::_addressesAreCheckedAndCleanedLikeTheCore()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    const QString rtsp = QString::fromUtf8(VideoSettings::videoSourceRTSP);
    const QString udp = QString::fromUtf8(VideoSettings::videoSourceUDPH264);
    const QString webrtc = QString::fromUtf8(VideoSettings::videoSourceWebRTC);

    QCOMPARE(VideoSettings::normalizedUrl(udp, QStringLiteral("UDP://0.0.0.0:5600")), QStringLiteral("0.0.0.0:5600"));
    QCOMPARE(VideoSettings::normalizedUrl(QString::fromUtf8(VideoSettings::videoSourceUDPH265), QStringLiteral("udp://0.0.0.0:5600")), QStringLiteral("0.0.0.0:5600"));
    QCOMPARE(VideoSettings::normalizedUrl(QString::fromUtf8(VideoSettings::videoSourceTCP), QStringLiteral("tcp://10.0.0.5:5600")), QStringLiteral("10.0.0.5:5600"));
    QCOMPARE(VideoSettings::normalizedUrl(rtsp, QStringLiteral("RTSP://10.0.0.5/live")), QStringLiteral("rtsp://10.0.0.5/live"));
    QCOMPARE(VideoSettings::normalizedUrl(webrtc, QStringLiteral("HTTP://cam/whep")), QStringLiteral("http://cam/whep"));

    QCOMPARE(settings->problem(rtsp, QString()), QStringLiteral("This kind of stream needs an address."));
    QCOMPARE(settings->problem(rtsp, QStringLiteral("rtsp:/10.0.0.5/live")), QStringLiteral("An RTSP address starts with rtsp://."));
    QCOMPARE(settings->problem(webrtc, QStringLiteral("rtsp://10.0.0.5/live")), QStringLiteral("A WebRTC address starts with http:// or https://."));
    QCOMPARE(settings->problem(webrtc, QStringLiteral("10.0.0.5:8889/cam/whep")), QString());
    QCOMPARE(settings->problem(udp, QStringLiteral("rtsp://x")), QStringLiteral("Leave the scheme off. The app adds it, and a doubled one fails to resolve."));

    QCOMPARE(settings->addCamera(QStringLiteral("radio"), udp, QStringLiteral("udp://0.0.0.0:5600")), QString());
    QCOMPARE(settings->videoUrlAt(3), QStringLiteral("0.0.0.0:5600"));
    QCOMPARE(settings->addCamera(QStringLiteral("empty"), rtsp, QString()), QStringLiteral("This kind of stream needs an address."));
    QCOMPARE(settings->storedCameraCount(), 4);
    QCOMPARE(settings->updateCamera(0, QStringLiteral("cam1"), rtsp, QStringLiteral("RTSP://10.0.0.5/one")), QString());
    QCOMPARE(settings->videoUrlAt(0), QStringLiteral("rtsp://10.0.0.5/one"));
    QCOMPARE(settings->updateCamera(9, QStringLiteral("x"), rtsp, QStringLiteral("rtsp://x")), QStringLiteral("There is no camera at that position."));

    QCOMPARE(VideoSettings::urlHost(QStringLiteral("rtsp://admin:pw@10.0.0.5/live")), QStringLiteral("10.0.0.5"));
    QCOMPARE(VideoSettings::urlHost(QStringLiteral("rtsp://admin:pw@10.0.0.5:554/live")), QStringLiteral("10.0.0.5"));
    QCOMPARE(VideoSettings::urlHost(QStringLiteral("10.0.0.5:5600")), QStringLiteral("10.0.0.5"));

    settings->storeCameras(QJsonArray{
        VideoSettings::camera(QStringLiteral("tcp"), QString::fromUtf8(VideoSettings::videoSourceTCP), QStringLiteral("10.0.0.5:5600")),
        VideoSettings::camera(QStringLiteral("other"), rtsp, QStringLiteral("rtsp://10.0.0.9/cam")),
    }, 0);
    settings->adoptDeviceCameras(QStringLiteral("10.0.0.5"), QJsonArray{VideoSettings::camera(QStringLiteral("front"), rtsp, QStringLiteral("rtsp://10.0.0.5:8554/front"))});
    QCOMPARE(settings->videoSourceCount(), 2);
    QCOMPARE(settings->cameraName(0), QStringLiteral("other"));
    QCOMPARE(settings->cameraName(1), QStringLiteral("front"));
}

void VideoCameraSwitchTest::_droneCameraNamesFollowTheCore()
{
    QCOMPARE(VideoManager::_droneCameraName(QStringLiteral("SIYI A8"), QStringLiteral("Stream 1-1"), true, 100), QStringLiteral("SIYI A8 · Stream 1-1"));
    QCOMPARE(VideoManager::_droneCameraName(QStringLiteral("SIYI A8"), QStringLiteral("Stream 1-1"), false, 100), QStringLiteral("SIYI A8"));
    QCOMPARE(VideoManager::_droneCameraName(QStringLiteral("SIYI A8"), QString(), true, 100), QStringLiteral("SIYI A8"));
    QCOMPARE(VideoManager::_droneCameraName(QString(), QStringLiteral("Front"), false, 100), QStringLiteral("Front"));
    QCOMPARE(VideoManager::_droneCameraName(QString(), QString(), false, 101), QStringLiteral("Drone camera 101"));
}

void VideoCameraSwitchTest::_storedListAndIndexAreSeenTogether()
{
    ThreeCameraFixture fixture;
    VideoSettings *settings = fixture.settings();
    QList<int> activeSeenWithList;
    QStringList listSeenWithActive;
    const QMetaObject::Connection onList = connect(settings->cameras(), &Fact::rawValueChanged, this, [settings, &activeSeenWithList]() {
        activeSeenWithList.append(settings->activeVideoSource()->rawValue().toInt());
    });
    const QMetaObject::Connection onActive = connect(settings->activeVideoSource(), &Fact::rawValueChanged, this, [settings, &listSeenWithActive]() {
        listSeenWithActive.append(settings->cameraName(0));
    });

    VideoManager::instance()->storeCameras(QStringLiteral(R"([{"name":"cam3","source":"RTSP Video Stream","url":"rtsp://two"},{"name":"cam1","source":"RTSP Video Stream","url":"rtsp://zero"}])"), 1);
    (void) disconnect(onList);
    (void) disconnect(onActive);

    QCOMPARE(activeSeenWithList, QList<int>{1});
    QCOMPARE(listSeenWithActive, QStringList{QStringLiteral("cam3")});
    QCOMPARE(settings->cameraName(settings->currentIndex()), QStringLiteral("cam1"));
}

void VideoCameraSwitchTest::_videoStatsReadLikeTheWatchPage()
{
    QCOMPARE(VideoManager::formatVideoStats(279, 25, 1080), QStringLiteral("279 ms · 25 fps · 1080p"));
    QCOMPARE(VideoManager::formatVideoStats(-1, 30, 720), QStringLiteral("30 fps · 720p"));
    QCOMPARE(VideoManager::formatVideoStats(80, 0, 0), QStringLiteral("80 ms · 0 fps"));
}

UT_REGISTER_TEST(VideoCameraSwitchTest, TestLabel::Unit)
