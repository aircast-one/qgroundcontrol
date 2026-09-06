#include "PlanViewLayoutTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

namespace {

struct Layout {
    QQuickItem *planView  = nullptr;
    QQuickItem *dock      = nullptr;
    QQuickItem *inspector = nullptr;
    qreal       margin    = 0;
};

QQuickItem *findItemByName(QQuickItem *item, const QString &name)
{
    if (item->objectName() == name) {
        return item;
    }
    const QList<QQuickItem*> children = item->childItems();
    for (QQuickItem *const child : children) {
        if (QQuickItem *const found = findItemByName(child, name)) {
            return found;
        }
    }
    return nullptr;
}

bool load(QQuickView &view, QQmlPropertyMap &globals, Layout &layout)
{
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/PlanViewLayoutTest.qml"))) {
        return false;
    }
    layout.planView  = findItemByName(view.rootObject(), QStringLiteral("planView"));
    layout.dock      = findItemByName(view.rootObject(), QStringLiteral("planDock"));
    layout.inspector = findItemByName(view.rootObject(), QStringLiteral("planInspector"));
    if (!layout.planView || !layout.dock || !layout.inspector) {
        return false;
    }
    layout.margin = layout.planView->property("_panelMargin").toReal();
    return QTest::qWaitFor([&layout] { return layout.inspector->height() > 0 && layout.dock->height() > 0; });
}

void resize(QQuickView &view, int width, int height)
{
    view.resize(width, height);
    view.rootObject()->setSize(QSizeF(width, height));
}

bool settled(const Layout &layout, std::function<bool()> condition)
{
    return QTest::qWaitFor([&layout, condition] { return layout.inspector->width() > 0 && condition(); });
}

}

void PlanViewLayoutTest::_narrowWindowStacksTheInspectorUnderTheDock()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));

    const qreal rootWidth  = layout.planView->width();
    const qreal rootHeight = layout.planView->height();
    QVERIFY(rootHeight > rootWidth);

    QVERIFY(settled(layout, [&] { return layout.inspector->y() >= layout.dock->y() + layout.dock->height(); }));
    QCOMPARE(layout.inspector->width(), rootWidth - layout.margin * 2);
    QCOMPARE(layout.inspector->x(), layout.margin);
    QCOMPARE(layout.inspector->y() + layout.inspector->height(), rootHeight - layout.margin);
    QVERIFY(layout.dock->y() > 0);
}

void PlanViewLayoutTest::_wideWindowKeepsTheInspectorOnTheRight()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));

    resize(view, 903, 428);

    QVERIFY(settled(layout, [&] { return layout.inspector->x() + layout.inspector->width() == 903 - layout.margin; }));
    QVERIFY(layout.inspector->width() < 903 / 2);
    QVERIFY(layout.inspector->y() + layout.inspector->height() <= 428 - layout.margin);
    QCOMPARE(layout.dock->y(), (428 - layout.dock->height()) / 2);
}

void PlanViewLayoutTest::_rotatingBackRestoresTheSideInspector()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));

    resize(view, 903, 428);
    QVERIFY(settled(layout, [&] { return layout.inspector->x() + layout.inspector->width() == 903 - layout.margin; }));

    resize(view, 428, 903);
    QVERIFY(settled(layout, [&] { return layout.inspector->y() >= layout.dock->y() + layout.dock->height(); }));
    QCOMPARE(layout.inspector->width(), 428 - layout.margin * 2);

    resize(view, 903, 428);
    QVERIFY(settled(layout, [&] { return layout.inspector->x() + layout.inspector->width() == 903 - layout.margin; }));
    QCOMPARE(layout.dock->y(), (428 - layout.dock->height()) / 2);
}
