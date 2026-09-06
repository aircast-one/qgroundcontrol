#include "PlanToolBarIndicatorsTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

#include <algorithm>

namespace {

constexpr qreal kTolerance = 0.5;

struct Row {
    QQuickItem *root  = nullptr;
    QQuickItem *row   = nullptr;
    QQuickItem *name  = nullptr;
    QQuickItem *stats = nullptr;

    qreal fontPixelWidth() const { return root->property("fontPixelWidth").toReal(); }

    bool everyVisibleChildInside() const
    {
        const QList<QQuickItem*> children = row->childItems();
        return std::all_of(children.cbegin(), children.cend(), [this](const QQuickItem *child) {
            return !child->isVisible() || child->x() + child->width() <= row->width() + kTolerance;
        });
    }
};

bool load(QQuickView &view, QQmlPropertyMap &globals, Row &row)
{
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/PlanToolBarIndicatorsTest.qml"))) {
        return false;
    }
    row.root  = view.rootObject();
    row.row   = findItemByName(row.root, QStringLiteral("planRow"));
    row.name  = findItemByName(row.root, QStringLiteral("planNameCapsule"));
    row.stats = findItemByName(row.root, QStringLiteral("planStatsCapsule"));
    if (!row.row || !row.name || !row.stats) {
        return false;
    }
    if (!QMetaObject::invokeMethod(row.root, "addWaypoint")) {
        return false;
    }
    return QTest::qWaitFor([&row] { return row.row->width() > 0 && row.name->width() > 0; });
}

void resize(QQuickView &view, int width)
{
    view.resize(width, 80);
    view.rootObject()->setSize(QSizeF(width, 80));
}

}

void PlanToolBarIndicatorsTest::_narrowRowKeepsEveryControlInside()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Row row;
    QVERIFY(load(view, globals, row));

    QTRY_VERIFY(!row.stats->isVisible());
    QTRY_VERIFY(row.everyVisibleChildInside());
    QVERIFY(row.name->width() >= row.fontPixelWidth() * 10 - kTolerance);
    QVERIFY(row.name->property("visible").toBool());
}

void PlanToolBarIndicatorsTest::_wideRowShowsTheFigures()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Row row;
    QVERIFY(load(view, globals, row));

    resize(view, 903);

    QTRY_VERIFY(row.stats->isVisible());
    QTRY_VERIFY(row.everyVisibleChildInside());
    QVERIFY(row.name->width() >= row.fontPixelWidth() * 10 - kTolerance);
}
