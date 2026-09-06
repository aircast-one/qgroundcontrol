/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCTileCacheWorkerTest.h"

#include "QGCMapTasks.h"
#include "QGCTileCacheWorker.h"

#include <QtCore/QDeadlineTimer>
#include <QtTest/QTest>

void QGCTileCacheWorkerTest::_stopIsIdempotentAndEndsTheThread()
{
    QGCCacheWorker worker;

    for (int i = 0; i < 200; i++) {
        QVERIFY(worker.enqueueTask(new QGCMapTask(QGCMapTask::taskInit)));
    }

    worker.stop();
    worker.stop();

    QVERIFY(worker.wait(QDeadlineTimer(3000)));
    QVERIFY(worker.isFinished());
}

void QGCTileCacheWorkerTest::_stopRefusesFurtherTasks()
{
    QGCCacheWorker worker;

    worker.stop();
    QVERIFY(worker.wait(QDeadlineTimer(3000)));

    QCOMPARE(worker.enqueueTask(new QGCMapTask(QGCMapTask::taskInit)), false);
    QVERIFY(!worker.isRunning());
}
