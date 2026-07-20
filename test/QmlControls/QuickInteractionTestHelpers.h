/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "QGroundControlQmlGlobal.h"

#include <QtCore/QSettings>
#include <QtGui/QGuiApplication>
#include <QtGui/QStyleHints>
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickView>
#include <QtTest/QTest>

// MouseArea drag anchors at the first move past the drag threshold, swallowing that offset.
// Leading with an explicit anchor move and overshooting the end point by the same amount keeps
// the target displacement exactly to - from. The nudge must exceed the platform drag threshold,
// so derive it from styleHints instead of hard-coding. Drags without that anchor behavior (raw
// mouse tracking, DragHandler which preserves the press offset) skip the compensation.
inline QPoint dragAnchorNudge()
{
    const int distance = QGuiApplication::styleHints()->startDragDistance() * 2;
    return QPoint(distance, distance);
}

inline void clearQmlGlobalSettings(std::initializer_list<const char*> keys)
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    for (const char* key : keys) {
        settings.remove(QString::fromLatin1(key));
    }
    settings.endGroup();
}

inline bool loadTestView(QQuickView& view, const QString& qmlResource)
{
    view.engine()->addImportPath(QStringLiteral("qrc:/qml"));
    view.setSource(QUrl(qmlResource));
    if (view.status() != QQuickView::Ready) {
        return false;
    }
    view.show();
    return QTest::qWaitForWindowExposed(&view);
}

inline void dragMouse(QQuickView& view, const QPoint& from, const QPoint& to, bool compensateDragThresholdAnchor = true)
{
    const QPoint nudge = compensateDragThresholdAnchor ? dragAnchorNudge() : QPoint(0, 0);
    const QPoint anchor = from + nudge;
    const QPoint end = to + nudge;
    QTest::mousePress(&view, Qt::LeftButton, Qt::NoModifier, from);
    const int steps = 10;
    for (int i = 0; i <= steps; i++) {
        QTest::mouseMove(&view, anchor + (end - anchor) * i / steps);
    }
    QTest::mouseRelease(&view, Qt::LeftButton, Qt::NoModifier, end);
}

inline QPoint itemCenter(QQuickItem* item)
{
    return item->mapToScene(QPointF(item->width() / 2, item->height() / 2)).toPoint();
}
