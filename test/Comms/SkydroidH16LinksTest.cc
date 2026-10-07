#include "SkydroidH16LinksTest.h"
#include "AutoConnectSettings.h"
#include "LinkManager.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "SkydroidH16Links.h"
#include "UDPLink.h"
#include "VideoSettings.h"

#include <QtTest/QTest>

namespace {

const UDPConfiguration *udpConfigOnPort(quint16 localPort)
{
    QmlObjectListModel *configs = LinkManager::instance()->linkConfigurations();
    for (int i = 0; i < configs->count(); i++) {
        const auto *config = qobject_cast<const LinkConfiguration *>(configs->get(i));
        if (!config || config->type() != LinkConfiguration::TypeUdp) {
            continue;
        }
        const auto *udp = static_cast<const UDPConfiguration *>(config);
        if (udp->localPort() == localPort) {
            return udp;
        }
    }
    return nullptr;
}

}

void SkydroidH16LinksTest::_ensureCreatesBothLinksOnce()
{
    AutoConnectSettings *autoConnect = SettingsManager::instance()->autoConnectSettings();
    VideoSettings *video = SettingsManager::instance()->videoSettings();
    const QVariant savedAutoConnectUdp = autoConnect->autoConnectUDP()->rawValue();
    const QVariant savedCameras = video->cameras()->rawValue();
    const QVariant savedActive = video->activeVideoSource()->rawValue();
    const QVariant savedMultiView = video->multiViewEnabled()->rawValue();
    video->multiViewEnabled()->setRawValue(false);
    autoConnect->autoConnectUDP()->setRawValue(true);
    video->cameras()->setRawValue(QStringLiteral("[]"));
    const int before = LinkManager::instance()->linkConfigurations()->count();

    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 3);
    QCOMPARE(LinkManager::instance()->linkConfigurations()->count(), before + 2);
    QVERIFY(!autoConnect->autoConnectUDP()->rawValue().toBool());
    QCOMPARE(video->videoSourceCount(), 2);
    QCOMPARE(video->videoSourceNameAt(0), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(video->videoUrlAt(0), SkydroidH16Links::cameraUrl(SkydroidH16Links::kCameraPaths.first()));
    QCOMPARE(video->cameraName(0), QStringLiteral("Camera 1"));
    QCOMPARE(video->cameraName(1), QStringLiteral("Camera 2"));
    QCOMPARE(video->videoUrlAt(1), SkydroidH16Links::cameraUrl(SkydroidH16Links::kCameraPaths.at(1)));
    QVERIFY(video->multiViewEnabled()->rawValue().toBool());

    const UDPConfiguration *telemetry = udpConfigOnPort(SkydroidH16Links::kTelemetryLocalPort);
    QVERIFY(telemetry);
    QVERIFY(telemetry->isAutoConnect());
    QVERIFY(telemetry->hostList().isEmpty());

    const UDPConfiguration *camera = udpConfigOnPort(SkydroidH16Links::kCameraLocalPort);
    QVERIFY(camera);
    QVERIFY(camera->isAutoConnect());
    QCOMPARE(camera->hostList(), QStringList{QStringLiteral("127.0.0.1:15552")});

    video->multiViewEnabled()->setRawValue(false);
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 0);
    QCOMPARE(LinkManager::instance()->linkConfigurations()->count(), before + 2);
    QVERIFY(!video->multiViewEnabled()->rawValue().toBool());

    video->storeCameras(QJsonArray{SkydroidH16Links::cameras().first()}, 0);
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 1);
    QCOMPARE(video->videoSourceCount(), 2);

    video->storeCameras(QJsonArray{VideoSettings::camera(QStringLiteral("Mine"), QString::fromUtf8(VideoSettings::videoSourceRTSP), QStringLiteral("rtsp://10.0.0.5/mine"))}, 0);
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 0);
    QCOMPARE(video->videoUrlAt(0), QStringLiteral("rtsp://10.0.0.5/mine"));
    QCOMPARE(video->videoSourceCount(), 1);

    LinkManager::instance()->removeConfiguration(const_cast<UDPConfiguration *>(telemetry));
    LinkManager::instance()->removeConfiguration(const_cast<UDPConfiguration *>(camera));
    autoConnect->autoConnectUDP()->setRawValue(savedAutoConnectUdp);
    video->cameras()->setRawValue(savedCameras);
    video->activeVideoSource()->setRawValue(savedActive);
    video->multiViewEnabled()->setRawValue(savedMultiView);
}

UT_REGISTER_TEST(SkydroidH16LinksTest, TestLabel::Unit)
