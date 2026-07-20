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
