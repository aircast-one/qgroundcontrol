/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "DebugApiServerTest.h"
#include "DebugApiServer.h"

#include "QuickInteractionTestHelpers.h"

#include <QtCore/QDateTime>
#include <QtCore/QJsonArray>
#include <QtCore/QSet>
#include <QtNetwork/QTcpSocket>
#include <QtQuick/QQuickView>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>
#include <QtTest/QTest>

namespace {

QNetworkReply *_get(QNetworkAccessManager &network, quint16 port, const QString &path, bool authHeader)
{
    QNetworkRequest request(QUrl(QStringLiteral("http://127.0.0.1:%1%2").arg(port).arg(path)));
    if (authHeader) {
        request.setRawHeader("X-QGC-Debug-Api", "1");
    }
    return network.get(request);
}

} // namespace

void DebugApiServerTest::_missingAuthHeaderRejected()
{
    DebugApiServer server(0);
    QVERIFY(server.serverPort() != 0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/status"), false);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 403);
    reply->deleteLater();
}

void DebugApiServerTest::_statusEndpoint()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/status"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 200);

    const QJsonObject status = QJsonDocument::fromJson(reply->readAll()).object();
    QVERIFY(status.contains(QStringLiteral("cameras")));
    QVERIFY(status.contains(QStringLiteral("activeVideoSource")));
    reply->deleteLater();
}

void DebugApiServerTest::_unknownPathReturns404()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/no-such-endpoint"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 404);
    reply->deleteLater();
}

void DebugApiServerTest::_handlerErrorReturns400()
{
    DebugApiServer server(0);

    // No vehicle is connected in the unit test environment, so the handler fails.
    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/vehicle/params"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);

    const QJsonObject body = QJsonDocument::fromJson(reply->readAll()).object();
    QCOMPARE(body.value(QStringLiteral("error")).toString(), QStringLiteral("no vehicle connected"));
    reply->deleteLater();
}

void DebugApiServerTest::_uiClickRejectsUnknownButton()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/ui/click?x=1&y=1&button=middle"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);

    const QJsonObject body = QJsonDocument::fromJson(reply->readAll()).object();
    QVERIFY(body.value(QStringLiteral("error")).toString().contains(QStringLiteral("button must be left or right")));
    reply->deleteLater();
}

void DebugApiServerTest::_uiClickAcceptsLeftAndRightButton()
{
    DebugApiServer server(0);
    QNetworkAccessManager network;

    // No main window exists in the unit test environment, so a valid button gets past
    // validation and fails on the window lookup instead.
    for (const QString &button : {QStringLiteral("left"), QStringLiteral("right"), QString()}) {
        const QString path = button.isEmpty() ? QStringLiteral("/ui/click?x=1&y=1")
                                              : QStringLiteral("/ui/click?x=1&y=1&button=%1").arg(button);
        QNetworkReply *reply = _get(network, server.serverPort(), path, true);
        QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);

        const QJsonObject body = QJsonDocument::fromJson(reply->readAll()).object();
        QCOMPARE(body.value(QStringLiteral("error")).toString(), QStringLiteral("no main window"));
        reply->deleteLater();
    }
}

void DebugApiServerTest::_motorTestRefusedWithoutActuatorGate()
{
    qunsetenv("QGC_DEBUG_API_ALLOW_ACTUATORS");
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/vehicle/motortest?motor=1&percent=10"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);

    const QJsonObject body = QJsonDocument::fromJson(reply->readAll()).object();
    QVERIFY(body.value(QStringLiteral("error")).toString().contains(QStringLiteral("disabled")));
    reply->deleteLater();
}

// The parameter-validation paths for the endpoints added for animation work. A main window is not
// available here, so these pin routing and argument handling rather than values read off live QML.
void DebugApiServerTest::_uiPropRequiresNameAndProperty()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/ui/prop"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);
    QVERIFY(QJsonDocument::fromJson(reply->readAll()).object().contains(QStringLiteral("error")));
    reply->deleteLater();
}

void DebugApiServerTest::_uiSetPropRequiresValue()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(),
                                QStringLiteral("/ui/setprop?name=someItem&property=width"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);
    QVERIFY(QJsonDocument::fromJson(reply->readAll()).object().contains(QStringLiteral("error")));
    reply->deleteLater();
}

void DebugApiServerTest::_uiAtRequiresCoordinates()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/ui/at"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);
    QVERIFY(QJsonDocument::fromJson(reply->readAll()).object().contains(QStringLiteral("error")));
    reply->deleteLater();
}

// /ui/watch keeps the socket and writes its own response, bypassing the normal routing, so its
// failure path is worth pinning separately: a bad request there must still answer and hang up
// rather than leave the caller waiting on a stream that will never arrive.
void DebugApiServerTest::_uiWatchRejectsMissingArguments()
{
    DebugApiServer server(0);

    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, server.serverPort(), QStringLiteral("/ui/watch"), true);
    QTRY_VERIFY_WITH_TIMEOUT(reply->isFinished(), 5000);
    QCOMPARE(reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt(), 400);
    QVERIFY(QJsonDocument::fromJson(reply->readAll()).object().contains(QStringLiteral("error")));
    reply->deleteLater();
}

namespace {

// Everything below drives the endpoints against a real scene. Without a window they can only be
// tested on their failure paths, which leaves the part that actually reads QML uncovered.
class ProbeScene
{
public:
    ProbeScene()
    {
        _ok = loadTestView(_view, QStringLiteral("qrc:/unittest/DebugApiTest.qml"));
        DebugApiServer::setWindowForTesting(&_view);
    }
    ~ProbeScene() { DebugApiServer::setWindowForTesting(nullptr); }

    bool ready() const { return _ok; }

private:
    QQuickView _view;
    bool       _ok = false;
};

QJsonObject _getJson(quint16 port, const QString &path, int *statusOut = nullptr)
{
    QNetworkAccessManager network;
    QNetworkReply *reply = _get(network, port, path, true);
    while (!reply->isFinished()) {
        QTest::qWait(10);
    }
    if (statusOut) {
        *statusOut = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
    }
    const QJsonObject object = QJsonDocument::fromJson(reply->readAll()).object();
    reply->deleteLater();
    return object;
}

} // namespace

void DebugApiServerTest::_uiPropReadsSeveralPropertiesInOneSample()
{
    ProbeScene scene;
    QVERIFY(scene.ready());
    DebugApiServer server(0);

    const QJsonObject result = _getJson(server.serverPort(),
        QStringLiteral("/ui/prop?name=probeTarget&property=width,height,caption"));

    // One sample, one timestamp: the point of the batch form is that the values cannot drift
    // apart the way they do when each is fetched over its own request.
    QVERIFY(result.contains(QStringLiteral("t")));
    const QJsonObject values = result.value(QStringLiteral("values")).toObject();
    QCOMPARE(values.value(QStringLiteral("width")).toDouble(), 120.0);
    QCOMPARE(values.value(QStringLiteral("height")).toDouble(), 80.0);
    QCOMPARE(values.value(QStringLiteral("caption")).toString(), QStringLiteral("hello"));

    // A single property keeps the original shape so existing callers are unaffected.
    const QJsonObject single = _getJson(server.serverPort(),
        QStringLiteral("/ui/prop?name=probeTarget&property=width"));
    QCOMPARE(single.value(QStringLiteral("property")).toString(), QStringLiteral("width"));
    QCOMPARE(single.value(QStringLiteral("value")).toDouble(), 120.0);
}

void DebugApiServerTest::_uiSetPropWritesAndCoerces()
{
    ProbeScene scene;
    QVERIFY(scene.ready());
    DebugApiServer server(0);

    QCOMPARE(_getJson(server.serverPort(),
        QStringLiteral("/ui/setprop?name=probeTarget&property=caption&value=written"))
            .value(QStringLiteral("value")).toString(), QStringLiteral("written"));

    // The string off the query is coerced to whatever the property already holds.
    QCOMPARE(_getJson(server.serverPort(),
        QStringLiteral("/ui/setprop?name=probeTarget&property=flagged&value=true"))
            .value(QStringLiteral("value")).toBool(), true);
    QCOMPARE(_getJson(server.serverPort(),
        QStringLiteral("/ui/setprop?name=probeTarget&property=reading&value=2.25"))
            .value(QStringLiteral("value")).toDouble(), 2.25);

    int status = 0;
    _getJson(server.serverPort(),
        QStringLiteral("/ui/setprop?name=probeTarget&property=reading&value=notanumber"), &status);
    QCOMPARE(status, 400);
}

void DebugApiServerTest::_uiAtReportsTheStackUnderAPoint()
{
    ProbeScene scene;
    QVERIFY(scene.ready());
    DebugApiServer server(0);

    // probeTarget covers 40,50 -> 160,130; the unnamed rectangle sits inside it at 60,70.
    const QJsonArray hits = _getJson(server.serverPort(), QStringLiteral("/ui/at?x=70&y=80"))
                                .value(QStringLiteral("hits")).toArray();
    QStringList names;
    for (const QJsonValue &hit : hits) {
        names.append(hit.toObject().value(QStringLiteral("objectName")).toString());
    }
    QVERIFY(names.contains(QStringLiteral("probeTarget")));

    // A point outside every child still reports the root, and never the hidden item.
    const QJsonArray corner = _getJson(server.serverPort(), QStringLiteral("/ui/at?x=390&y=290"))
                                  .value(QStringLiteral("hits")).toArray();
    for (const QJsonValue &hit : corner) {
        QVERIFY(hit.toObject().value(QStringLiteral("objectName")).toString() != QStringLiteral("probeTarget"));
    }
}

void DebugApiServerTest::_uiTreeFindsUnnamedItemsOnlyWithAll()
{
    ProbeScene scene;
    QVERIFY(scene.ready());
    DebugApiServer server(0);

    const QJsonArray named = _getJson(server.serverPort(), QStringLiteral("/ui/tree"))
                                 .value(QStringLiteral("items")).toArray();
    for (const QJsonValue &item : named) {
        QVERIFY(!item.toObject().value(QStringLiteral("objectName")).toString().isEmpty());
    }

    // all=1 is the mode that finds something nobody thought to name - which is exactly the item
    // you are hunting for when a click lands on nothing.
    const QJsonArray all = _getJson(server.serverPort(), QStringLiteral("/ui/tree?all=1"))
                               .value(QStringLiteral("items")).toArray();
    QVERIFY(all.size() > named.size());
    bool sawUnnamed = false;
    for (const QJsonValue &item : all) {
        if (item.toObject().value(QStringLiteral("objectName")).toString().isEmpty()) {
            sawUnnamed = true;
            QVERIFY(!item.toObject().value(QStringLiteral("class")).toString().isEmpty());
        }
    }
    QVERIFY(sawUnnamed);
}

void DebugApiServerTest::_uiWatchStreamsSamplesWhileAValueChanges()
{
    ProbeScene scene;
    QVERIFY(scene.ready());
    DebugApiServer server(0);

    // A raw socket, because the response is an open NDJSON stream rather than a finished body.
    QTcpSocket socket;
    socket.connectToHost(QHostAddress::LocalHost, server.serverPort());
    QVERIFY(socket.waitForConnected(5000));
    socket.write("GET /ui/watch?name=probeTarget&property=counter,reading&frames=12&interval=10 "
                 "HTTP/1.1\r\nHost: localhost\r\nX-QGC-Debug-Api: 1\r\n\r\n");
    QVERIFY(socket.waitForBytesWritten(5000));

    QByteArray received;
    const qint64 deadline = QDateTime::currentMSecsSinceEpoch() + 8000;
    while ((QDateTime::currentMSecsSinceEpoch() < deadline) && (received.count('\n') < 13)) {
        QTest::qWait(20);
        received += socket.readAll();
    }
    socket.close();

    QVERIFY(received.startsWith("HTTP/1.1 200 OK"));
    QVERIFY(received.contains("application/x-ndjson"));

    const QByteArray body = received.mid(received.indexOf("\r\n\r\n") + 4);
    QList<QJsonObject> samples;
    const QList<QByteArray> lines = body.split('\n');
    for (const QByteArray &line : lines) {
        if (!line.trimmed().isEmpty()) {
            samples.append(QJsonDocument::fromJson(line).object());
        }
    }
    QVERIFY(samples.size() >= 5);

    // Every sample is stamped by the app and carries both properties from the same instant.
    QSet<int> counters;
    for (const QJsonObject &sample : samples) {
        QVERIFY(sample.contains(QStringLiteral("t")));
        const QJsonObject values = sample.value(QStringLiteral("values")).toObject();
        QVERIFY(values.contains(QStringLiteral("counter")));
        const int counter = values.value(QStringLiteral("counter")).toInt();
        const double reading = values.value(QStringLiteral("reading")).toDouble();
        // reading is derived from counter by the scene, so a sample that mixed two instants
        // would show them disagreeing.
        QVERIFY(qFuzzyCompare(reading + 1.0, (counter / 10.0) + 1.0));
        counters.insert(counter);
    }
    QVERIFY(counters.size() > 1);
}

// A watch holds its socket open for its whole life, which the single-request guard around normal
// routing does not cover. Two streams plus an ordinary request have to coexist: if a held socket
// blocked the server, everything after the first watch would hang.
void DebugApiServerTest::_uiWatchSurvivesConcurrentStreamsAndRequests()
{
    ProbeScene scene;
    QVERIFY(scene.ready());
    DebugApiServer server(0);

    const auto openWatch = [&](QTcpSocket &socket, const char *property) {
        socket.connectToHost(QHostAddress::LocalHost, server.serverPort());
        QVERIFY(socket.waitForConnected(5000));
        socket.write(QByteArray("GET /ui/watch?name=probeTarget&property=") + property +
                     "&frames=40&interval=10 HTTP/1.1\r\nHost: localhost\r\nX-QGC-Debug-Api: 1\r\n\r\n");
        QVERIFY(socket.waitForBytesWritten(5000));
    };

    QTcpSocket first;
    QTcpSocket second;
    openWatch(first, "counter");
    openWatch(second, "reading");

    // An ordinary request must still be answered while both streams are running.
    int status = 0;
    const QJsonObject plain = _getJson(server.serverPort(),
        QStringLiteral("/ui/prop?name=probeTarget&property=width"), &status);
    QCOMPARE(status, 200);
    QCOMPARE(plain.value(QStringLiteral("value")).toDouble(), 120.0);

    QByteArray firstBody;
    QByteArray secondBody;
    const qint64 deadline = QDateTime::currentMSecsSinceEpoch() + 6000;
    while ((QDateTime::currentMSecsSinceEpoch() < deadline) &&
           ((firstBody.count('\n') < 4) || (secondBody.count('\n') < 4))) {
        QTest::qWait(20);
        firstBody += first.readAll();
        secondBody += second.readAll();
    }
    first.close();
    second.close();

    QVERIFY(firstBody.contains("\"counter\""));
    QVERIFY(secondBody.contains("\"reading\""));
    QVERIFY(firstBody.count('\n') >= 3);
    QVERIFY(secondBody.count('\n') >= 3);
}
