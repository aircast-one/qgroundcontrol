#include "OtlpLogExporterTest.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

#include "OtlpLogExporter.h"

namespace {

LogEntry entryAt(LogEntry::Level level, const QString& message)
{
    LogEntry entry;
    entry.timestamp = QDateTime::fromMSecsSinceEpoch(1790085600123LL, QTimeZone::UTC);
    entry.level = level;
    entry.category = QStringLiteral("VideoManager.GstVideoReceiver");
    entry.message = message;
    entry.file = QStringLiteral("GstVideoReceiver.cc");
    entry.function = QStringLiteral("start");
    entry.line = 42;
    return entry;
}

}  // namespace

// What an operator types is a host, not a URL. The collector serves logs at a
// fixed path, and only the plaintext OTLP ports mean "no TLS".
void OtlpLogExporterTest::_logsURL_data()
{
    QTest::addColumn<QString>("endpoint");
    QTest::addColumn<QString>("expected");

    QTest::newRow("bare host:port") << "otel.aircast.one:443" << "https://otel.aircast.one:443/v1/logs";
    QTest::newRow("bare host") << "otel.aircast.one" << "https://otel.aircast.one/v1/logs";
    QTest::newRow("plaintext grpc port") << "127.0.0.1:4317" << "http://127.0.0.1:4317/v1/logs";
    QTest::newRow("plaintext http port") << "127.0.0.1:4318" << "http://127.0.0.1:4318/v1/logs";
    QTest::newRow("explicit scheme") << "http://collector:8080" << "http://collector:8080/v1/logs";
    QTest::newRow("explicit path kept") << "https://collector/ingest/v1/logs" << "https://collector/ingest/v1/logs";
    QTest::newRow("whitespace") << "  otel.aircast.one:443  " << "https://otel.aircast.one:443/v1/logs";
    QTest::newRow("empty") << "" << "";
}

void OtlpLogExporterTest::_logsURL()
{
    QFETCH(QString, endpoint);
    QFETCH(QString, expected);
    QCOMPARE(OtlpLogExporter::logsURL(endpoint), expected);
}

void OtlpLogExporterTest::_severityMapping()
{
    QCOMPARE(OtlpLogExporter::severityNumber(LogEntry::Debug), 5);
    QCOMPARE(OtlpLogExporter::severityNumber(LogEntry::Info), 9);
    QCOMPARE(OtlpLogExporter::severityNumber(LogEntry::Warning), 13);
    QCOMPARE(OtlpLogExporter::severityNumber(LogEntry::Critical), 17);
    QCOMPARE(OtlpLogExporter::severityNumber(LogEntry::Fatal), 21);
    QCOMPARE(OtlpLogExporter::severityText(LogEntry::Critical), QStringLiteral("ERROR"));
}

// The body is the contract with the collector: a shape that drifts is rejected
// at the far end, where nobody is watching.
void OtlpLogExporterTest::_payloadShape()
{
    OtlpLogExporter exporter;
    exporter.setIdentity(QStringLiteral("aircast-qgc"), QStringLiteral("5.5.0"), QStringLiteral("session-1"));

    const QByteArray body = exporter.buildPayload({entryAt(LogEntry::Warning, QStringLiteral("video stalled"))});
    const QJsonObject root = QJsonDocument::fromJson(body).object();

    const QJsonObject resourceLogs = root.value(QStringLiteral("resourceLogs")).toArray().at(0).toObject();
    const QJsonArray resourceAttrs =
        resourceLogs.value(QStringLiteral("resource")).toObject().value(QStringLiteral("attributes")).toArray();
    QVariantMap resource;
    for (const QJsonValue& kv : resourceAttrs) {
        resource.insert(
            kv.toObject().value(QStringLiteral("key")).toString(),
            kv.toObject().value(QStringLiteral("value")).toObject().value(QStringLiteral("stringValue")).toVariant());
    }
    QCOMPARE(resource.value(QStringLiteral("service.name")).toString(), QStringLiteral("aircast-qgc"));
    QCOMPARE(resource.value(QStringLiteral("service.version")).toString(), QStringLiteral("5.5.0"));
    QCOMPARE(resource.value(QStringLiteral("session.id")).toString(), QStringLiteral("session-1"));

    const QJsonObject record = resourceLogs.value(QStringLiteral("scopeLogs"))
                                   .toArray()
                                   .at(0)
                                   .toObject()
                                   .value(QStringLiteral("logRecords"))
                                   .toArray()
                                   .at(0)
                                   .toObject();
    QCOMPARE(record.value(QStringLiteral("severityNumber")).toInt(), 13);
    QCOMPARE(record.value(QStringLiteral("severityText")).toString(), QStringLiteral("WARN"));
    QCOMPARE(record.value(QStringLiteral("body")).toObject().value(QStringLiteral("stringValue")).toString(),
             QStringLiteral("video stalled"));
    QCOMPARE(record.value(QStringLiteral("timeUnixNano")).toString(), QStringLiteral("1790085600123000000"));

    QVariantMap attrs;
    for (const QJsonValue& kv : record.value(QStringLiteral("attributes")).toArray()) {
        const QJsonObject value = kv.toObject().value(QStringLiteral("value")).toObject();
        attrs.insert(kv.toObject().value(QStringLiteral("key")).toString(),
                     value.contains(QStringLiteral("intValue"))
                         ? value.value(QStringLiteral("intValue")).toVariant()
                         : value.value(QStringLiteral("stringValue")).toVariant());
    }
    QCOMPARE(attrs.value(QStringLiteral("log.category")).toString(), QStringLiteral("VideoManager.GstVideoReceiver"));
    QCOMPARE(attrs.value(QStringLiteral("code.lineno")).toString(), QStringLiteral("42"));
}

// A collector that cannot be reached must not grow the queue until the
// application dies of it; the oldest lines go first and are counted.
void OtlpLogExporterTest::_queueIsBoundedAndDropsTheOldest()
{
    OtlpLogExporter exporter;
    exporter.configure(QStringLiteral("127.0.0.1:65000"), QString(), true);

    for (int i = 0; i < 2100; ++i) {
        exporter.enqueue(entryAt(LogEntry::Info, QStringLiteral("line %1").arg(i)));
    }
    QCOMPARE(exporter.queued(), 2000);
    QVERIFY(exporter.dropped() > 0);
}

void OtlpLogExporterTest::_nothingIsQueuedWhileDisabled()
{
    OtlpLogExporter exporter;
    exporter.configure(QStringLiteral("otel.aircast.one:443"), QStringLiteral("t"), false);
    exporter.enqueue(entryAt(LogEntry::Critical, QStringLiteral("this stays home")));
    QCOMPARE(exporter.queued(), 0);

    // Debug chatter is the bulk of the log and the least of what support needs.
    exporter.configure(QStringLiteral("otel.aircast.one:443"), QStringLiteral("t"), true);
    exporter.enqueue(entryAt(LogEntry::Debug, QStringLiteral("chatter")));
    QCOMPARE(exporter.queued(), 0);
    exporter.enqueue(entryAt(LogEntry::Info, QStringLiteral("worth sending")));
    QCOMPARE(exporter.queued(), 1);
}

// Nothing secret ships in this application: with no paired device and no
// token of its own, there is nothing to sign telemetry with — which is the
// point. An extracted build yields an address, not a credential.
void OtlpLogExporterTest::_noKeyShipsWithTheApplication()
{
    OtlpLogExporter exporter;
    exporter.configure(QStringLiteral("otel.aircast.one:443"), QString(), true);
    QVERIFY(!exporter.hasCredential());

    // A collector of your own is still reached with a key of your own.
    exporter.configure(QStringLiteral("otel.example.com:443"), QStringLiteral("my-key"), true);
    QVERIFY(exporter.hasCredential());

    // Paired with an aircraft, the ground station has somewhere to ask.
    OtlpLogExporter paired;
    paired.configure(QStringLiteral("otel.aircast.one:443"), QString(), true);
    paired.setDeviceHost(QStringLiteral("guta-test.tail.ts.net"));
    QVERIFY(paired.hasCredential());
}

UT_REGISTER_TEST_LIGHTWEIGHT(OtlpLogExporterTest, TestLabel::Unit, TestLabel::Utilities)
