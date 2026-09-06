/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "TerrainProgressTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtCore/QCoreApplication>

void TerrainProgressTest::_bannerIsRigFurnitureWhileItExists()
{
    QQuickView view;
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/TerrainProgressTest.qml")));

    QQuickItem *const column = findItemByName(view.rootObject(), QStringLiteral("topRightColumn"));
    QVERIFY(column);
    QTRY_COMPARE(view.rootObject()->property("statics").toInt(), 1);

    delete column;
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);

    QTRY_COMPARE(view.rootObject()->property("statics").toInt(), 0);
}
