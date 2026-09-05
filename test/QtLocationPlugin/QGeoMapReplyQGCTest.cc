/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGeoMapReplyQGCTest.h"
#include "QGeoMapReplyQGC.h"
#include "QGCMapTasks.h"

#include <QtLocation/private/qgeotilespec_p.h>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>
#include <QtTest/QSignalSpy>

// The fetch task backing a tile reply runs asynchronously and can complete after the
// QNetworkAccessManager it was given is gone - torn down between unit tests, or during app
// shutdown, while a fetch was still in flight. _cacheError used to dereference that manager
// unconditionally and crash (SIGSEGV, confirmed via a full unit test suite run). It must instead
// fail the reply with an error and return.
void QGeoMapReplyQGCTest::_cacheErrorSurvivesADestroyedNetworkManager()
{
    QNetworkAccessManager* const manager = new QNetworkAccessManager;
    const QGeoTileSpec spec(QStringLiteral("qgeomapreplyqgctest"), 999999, 1, 0, 0);
    const QNetworkRequest request(QUrl(QStringLiteral("http://127.0.0.1.invalid/tile")));

    QGeoTiledMapReplyQGC* const reply = new QGeoTiledMapReplyQGC(manager, request, spec);
    QSignalSpy errorSpy(reply, &QGeoTiledMapReply::errorOccurred);

    delete manager;

    QVERIFY(QMetaObject::invokeMethod(reply, "_cacheError", Qt::DirectConnection,
                                       Q_ARG(QGCMapTask::TaskType, QGCMapTask::taskFetchTile),
                                       Q_ARG(QStringView, QStringView())));

    QCOMPARE(errorSpy.count(), 1);

    delete reply;
}

// Qt parents a reply to the manager that created it, so the reply cannot outlive the manager.
// Reparenting it onto the tile reply broke that: the manager was destroyed first and the
// orphaned reply then emitted QNetworkAccessManager::finished() through freed memory (SIGSEGV,
// reproduced 3/3 full suite runs). Nothing may take ownership of the reply away from the manager.
void QGeoMapReplyQGCTest::_networkReplyStaysOwnedByItsManager()
{
    QNetworkAccessManager manager;
    const QGeoTileSpec spec(QStringLiteral("qgeomapreplyqgctest"), 999999, 1, 0, 0);
    const QNetworkRequest request(QUrl(QStringLiteral("http://127.0.0.1.invalid/tile")));

    QGeoTiledMapReplyQGC* const reply = new QGeoTiledMapReplyQGC(&manager, request, spec);

    QVERIFY(QMetaObject::invokeMethod(reply, "_cacheError", Qt::DirectConnection,
                                      Q_ARG(QGCMapTask::TaskType, QGCMapTask::taskFetchTile),
                                      Q_ARG(QStringView, QStringView())));

    // Holds whether or not the request was issued - _cacheError declines to fetch when the
    // device reports no internet, and the tile reply must own no network reply either way.
    QVERIFY(reply->findChildren<QNetworkReply*>().isEmpty());

    const QList<QNetworkReply*> owned = manager.findChildren<QNetworkReply*>();
    if (!owned.isEmpty()) {
        QCOMPARE(owned.constFirst()->parent(), &manager);
    }

    delete reply;
}
