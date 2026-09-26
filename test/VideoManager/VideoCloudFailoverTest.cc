#include "VideoCloudFailoverTest.h"

#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

#include "VideoCloudFailover.h"
#include "VideoReceiver.h"

namespace {

class StubVideoReceiver : public VideoReceiver
{
public:
    StubVideoReceiver() { setName(QStringLiteral("videoContent")); }

    void start(uint32_t) final {}

    void stop() final {}

    void startDecoding(void*) final {}

    void stopDecoding() final {}

    void startRecording(const QString&, FILE_FORMAT) final {}

    void stopRecording() final {}

    void takeScreenshot(const QString&) final {}
};

const VideoCloudFailover::Device kDevice{QStringLiteral("100.64.0.12"), QStringLiteral("https://sfu.dev.aircast.one"),
                                         QStringLiteral("d-1")};
const QString kDirect = QStringLiteral("rtsp://100.64.0.12:8554/front");
const QString kCloud = QStringLiteral("https://sfu.dev.aircast.one/api/v1/whep/d-1/front");

struct Harness
{
    QString token = QStringLiteral("view-token");
    bool deviceAnswers = false;
    int restarts = 0;
    int probes = 0;
    VideoCloudFailover failover;
    StubVideoReceiver receiver;

    explicit Harness(VideoCloudFailover::Timing timing = fastTiming())
        : failover([this](const QString&, QObject*, VideoCloudFailover::TokenCallback done) { done(token); },
                   [this](const QString&, QObject*, std::function<void(bool)> done) {
                       ++probes;
                       done(deviceAnswers);
                   },
                   [this](VideoReceiver*) { ++restarts; }, timing)
    {
        receiver.setUri(kDirect);
        failover.setDevice(kDevice);
        failover.watch(&receiver);
    }

    static VideoCloudFailover::Timing fastTiming()
    {
        VideoCloudFailover::Timing timing;
        timing.stallMs = 50;
        timing.probeMs = 20;
        timing.probeSuccesses = 2;
        timing.holdMinMs = 40;
        timing.holdMaxMs = 1000;
        timing.settleMs = 5000;
        return timing;
    }
};

}  // namespace

void VideoCloudFailoverTest::_cloudUrlForAnAircastCamera()
{
    QCOMPARE(VideoCloudFailover::cloudUrlFor(kDevice, kDirect), kCloud);
    QCOMPARE(VideoCloudFailover::cloudUrlFor(kDevice, QStringLiteral("http://100.64.0.12:8889/rear cam/whep")),
             QStringLiteral("https://sfu.dev.aircast.one/api/v1/whep/d-1/rear%20cam"));
    QCOMPARE(VideoCloudFailover::cloudUrlFor(kDevice, QStringLiteral("rtsp://10.0.0.9:8554/front")), QString());
    QCOMPARE(VideoCloudFailover::cloudUrlFor(kDevice, QStringLiteral("udp://0.0.0.0:5600")), QString());
    QCOMPARE(
        VideoCloudFailover::cloudUrlFor(VideoCloudFailover::Device{kDevice.host, QString(), kDevice.deviceId}, kDirect),
        QString());
}

void VideoCloudFailoverTest::_aStalledStreamSwitchesToTheCloudCopyWithAViewToken()
{
    Harness h;
    QSignalSpy switched(&h.failover, &VideoCloudFailover::switched);

    QTRY_COMPARE_WITH_TIMEOUT(h.receiver.uri(), kCloud, 2000);
    QCOMPARE(h.receiver.authToken(), QStringLiteral("view-token"));
    QVERIFY(h.failover.onCloud(&h.receiver));
    QCOMPARE(h.restarts, 1);
    QCOMPARE(switched.count(), 1);
    QCOMPARE(switched.first().at(1).toBool(), true);
}

void VideoCloudFailoverTest::_theDeviceAnsweringAgainSwitchesBack()
{
    Harness h;
    QTRY_VERIFY_WITH_TIMEOUT(h.failover.onCloud(&h.receiver), 2000);
    emit h.receiver.decodingChanged(true);
    QTest::qWait(150);
    QVERIFY2(h.failover.onCloud(&h.receiver), "a device that doesn't answer keeps the cloud copy playing");

    h.deviceAnswers = true;
    QTRY_VERIFY_WITH_TIMEOUT(!h.failover.onCloud(&h.receiver), 2000);
    QCOMPARE(h.receiver.uri(), kDirect);
    QVERIFY(h.receiver.authToken().isEmpty());
    QCOMPARE(h.restarts, 2);
}

void VideoCloudFailoverTest::_withoutAViewTokenItStaysOnTheDevice()
{
    Harness h;
    h.token.clear();
    QTest::qWait(200);
    QCOMPARE(h.receiver.uri(), kDirect);
    QCOMPARE(h.restarts, 0);
}

void VideoCloudFailoverTest::_aDecodingStreamNeverFailsOver()
{
    Harness h;
    emit h.receiver.decodingChanged(true);
    QTest::qWait(200);
    QCOMPARE(h.receiver.uri(), kDirect);
}

void VideoCloudFailoverTest::_choosingAnotherSourceDropsTheCloudCopy()
{
    Harness h;
    QTRY_VERIFY_WITH_TIMEOUT(h.failover.onCloud(&h.receiver), 2000);
    emit h.receiver.decodingChanged(true);

    h.receiver.setUri(QStringLiteral("rtsp://100.64.0.12:8554/rear"));
    QVERIFY(!h.failover.onCloud(&h.receiver));
    QVERIFY(h.receiver.authToken().isEmpty());
}

void VideoCloudFailoverTest::_failingOverSoonAfterReturningWaitsLongerBeforeTheNextTry()
{
    VideoCloudFailover::Timing timing = Harness::fastTiming();
    timing.holdMinMs = 300;
    Harness h(timing);
    QTRY_VERIFY_WITH_TIMEOUT(h.failover.onCloud(&h.receiver), 2000);
    h.deviceAnswers = true;
    QTRY_VERIFY_WITH_TIMEOUT(!h.failover.onCloud(&h.receiver), 2000);

    QTRY_VERIFY_WITH_TIMEOUT(h.failover.onCloud(&h.receiver), 2000);
    QElapsedTimer onCloud;
    onCloud.start();
    QTRY_VERIFY_WITH_TIMEOUT(!h.failover.onCloud(&h.receiver), 3000);
    QVERIFY2(onCloud.elapsed() >= 500, "the second hold doubles the first");
}

UT_REGISTER_TEST(VideoCloudFailoverTest, TestLabel::Unit)
