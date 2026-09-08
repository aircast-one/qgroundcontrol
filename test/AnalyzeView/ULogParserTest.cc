#include "ULogParserTest.h"
#include "ULogParser.h"
#include "GeoTagWorker.h"

#include <QtTest/QTest>

void ULogParserTest::_getTagsFromLogTest()
{
    QFile file(":/unittest/SampleULog.ulg");
    QVERIFY(file.open(QIODevice::ReadOnly));

    const QByteArray logBuffer = file.readAll();
    file.close();

    QList<GeoTagWorker::CameraFeedbackPacket> cameraFeedback;
    QString errorMessage;
    QVERIFY(ULogParser::getTagsFromLog(logBuffer, cameraFeedback, errorMessage));
    QVERIFY(errorMessage.isEmpty());
    QVERIFY(!cameraFeedback.isEmpty());

    const GeoTagWorker::CameraFeedbackPacket firstCameraFeedback = cameraFeedback.constFirst();
    QVERIFY(firstCameraFeedback.imageSequence != 0);
}
