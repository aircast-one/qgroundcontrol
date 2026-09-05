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

template <typename Keys>
inline void clearQmlGlobalSettings(const Keys& keys)
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    for (const auto& key : keys) {
        settings.remove(QString(key));
    }
    settings.endGroup();
}

// Braced call sites deduce nothing against a template parameter, so they need this overload.
inline void clearQmlGlobalSettings(std::initializer_list<const char*> keys)
{
    clearQmlGlobalSettings<std::initializer_list<const char*>>(keys);
}

// DragToPosition stores one arrangement per window size, so its keys carry the size they were
// made at. Naming them in a test means the cleanup silently stops matching the day the test
// view is resized; match the prefix instead.
inline void clearDragPositionSettings(std::initializer_list<const char*> prefixes)
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    const QStringList keys = settings.allKeys();
    for (const char* prefix : prefixes) {
        for (const QString& key : keys) {
            if (key.startsWith(QLatin1String(prefix))) {
                settings.remove(key);
            }
        }
    }
    settings.endGroup();
}

// The position write in DragToPosition is debounced by a quarter second so a window drag does
// not thrash the settings file. A test that tears its view down the instant the drop lands
// persists nothing, and the reloaded view legitimately shows the default.
inline bool storedPosition(const QString& key)
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    return settings.value(key).toBool();
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

// Dropped positions quantize to DragToPosition's snapGrid, so a drag of N pixels does not
// land N pixels away. Expectations have to be snapped the same way or they are off by up to
// half a grid step.
inline qreal snapToDropGrid(qreal value, qreal extent, qreal size, qreal grid, qreal margin)
{
    const qreal low = margin;
    const qreal high = qMax(low, size - margin - extent);
    if (grid <= 0) {
        return qBound(low, value, high);
    }
    const bool nearFarEdge = value + (extent / 2) > size / 2;
    const qreal snapped = nearFarEdge ? high - qRound((high - value) / grid) * grid
                                      : low + qRound((value - low) / grid) * grid;
    return qBound(low, snapped, high);
}

inline qreal dropGridOf(QQuickItem* item)
{
    QObject* const dragPosition = item->findChild<QObject*>(QStringLiteral("dragPosition"));
    return dragPosition ? dragPosition->property("snapGrid").toReal() : 0;
}

inline qreal dropMarginOf(QQuickItem* item)
{
    QObject* const dragPosition = item->findChild<QObject*>(QStringLiteral("dragPosition"));
    return dragPosition ? dragPosition->property("edgeMargin").toReal() : 0;
}

inline QPoint itemCenter(QQuickItem* item)
{
    return item->mapToScene(QPointF(item->width() / 2, item->height() / 2)).toPoint();
}
