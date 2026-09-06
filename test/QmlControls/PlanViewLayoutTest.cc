#include "PlanViewLayoutTest.h"
#include "QuickInteractionTestHelpers.h"
#include "SettingsManager.h"
#include "PlanViewSettings.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlPropertyMap>

namespace {

constexpr qreal kTolerance = 0.5;

bool near(qreal a, qreal b)
{
    return qAbs(a - b) <= kTolerance;
}

struct Layout {
    QQuickItem *root      = nullptr;
    QQuickItem *planView  = nullptr;
    QQuickItem *dock      = nullptr;
    QQuickItem *inspector = nullptr;
    QQuickItem *terrain   = nullptr;
    qreal       margin    = 0;

    qreal panelHeight() const { return inspector->parentItem()->height(); }
    qreal panelWidth() const { return inspector->parentItem()->width(); }
    qreal fontPixelWidth() const { return root->property("fontPixelWidth").toReal(); }
    qreal terrainProfileHeight() const { return root->property("terrainProfileHeight").toReal(); }
    qreal inspectorBottom() const { return inspector->y() + inspector->height(); }
    qreal inspectorRight() const { return inspector->x() + inspector->width(); }
    qreal dockBottom() const { return dock->y() + dock->height(); }
};

class MissionStatusScope
{
public:
    explicit MissionStatusScope(bool shown)
        : _fact(SettingsManager::instance()->planViewSettings()->showMissionItemStatus())
        , _saved(_fact->rawValue())
    {
        _fact->setRawValue(shown);
    }
    ~MissionStatusScope() { _fact->setRawValue(_saved); }

private:
    Fact *const    _fact;
    const QVariant _saved;
};

bool load(QQuickView &view, QQmlPropertyMap &globals, Layout &layout)
{
    globals.insert(QStringLiteral("activeVehicle"), QVariant());
    view.engine()->rootContext()->setContextProperty(QStringLiteral("globals"), &globals);
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/PlanViewLayoutTest.qml"))) {
        return false;
    }
    layout.root      = view.rootObject();
    layout.planView  = findItemByName(layout.root, QStringLiteral("planView"));
    layout.dock      = findItemByName(layout.root, QStringLiteral("planDock"));
    layout.inspector = findItemByName(layout.root, QStringLiteral("planInspector"));
    layout.terrain   = findItemByName(layout.root, QStringLiteral("planTerrainSheet"));
    if (!layout.planView || !layout.dock || !layout.inspector || !layout.terrain) {
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

bool inspectorOnTheRight(const Layout &layout)
{
    return near(layout.inspectorRight(), layout.panelWidth() - layout.margin);
}

bool inspectorUnderTheDock(const Layout &layout)
{
    return layout.inspector->y() >= layout.dockBottom() - kTolerance;
}

qreal expectedWideWidth(const Layout &layout)
{
    return qMin(layout.panelWidth() / 3, layout.fontPixelWidth() * 40);
}

}

void PlanViewLayoutTest::_narrowWindowStacksTheInspectorUnderTheDock()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));

    QVERIFY(layout.planView->height() > layout.planView->width());

    QVERIFY(settled(layout, [&] { return inspectorUnderTheDock(layout); }));
    QVERIFY(near(layout.inspector->width(), layout.panelWidth() - layout.margin * 2));
    QVERIFY(near(layout.inspector->x(), layout.margin));
    QVERIFY(near(layout.inspectorBottom(), layout.panelHeight() - layout.margin));
    QVERIFY(layout.dock->y() > 0);
}

void PlanViewLayoutTest::_narrowWindowGivesTheTerrainProfileItsOwnBand()
{
    MissionStatusScope status(true);
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));
    QVERIFY(settled(layout, [&] { return inspectorUnderTheDock(layout); }));
    const qreal heightWithoutProfile = layout.inspector->height();

    QVERIFY(QMetaObject::invokeMethod(layout.root, "addWaypoint"));

    QVERIFY(settled(layout, [&] { return near(layout.terrain->height(), layout.terrainProfileHeight()); }));
    QVERIFY(settled(layout, [&] { return near(layout.terrain->y() + layout.terrain->height() + layout.margin, layout.inspector->y()); }));
    QVERIFY(layout.terrain->y() >= layout.dock->y() - kTolerance);
    QVERIFY(near(layout.terrain->x(), layout.dock->x() + layout.dock->width() + layout.margin));
    QVERIFY(near(layout.inspectorBottom(), layout.panelHeight() - layout.margin));
    QVERIFY(layout.inspector->height() <= heightWithoutProfile - layout.terrainProfileHeight() + kTolerance);
}

void PlanViewLayoutTest::_wideWindowKeepsTheInspectorOnTheRight()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));

    resize(view, 903, 428);

    QVERIFY(settled(layout, [&] { return inspectorOnTheRight(layout); }));
    QVERIFY(near(layout.inspector->width(), expectedWideWidth(layout)));
    QVERIFY(layout.inspectorBottom() <= layout.panelHeight() - layout.margin + kTolerance);
    QVERIFY(near(layout.dock->y(), (layout.panelHeight() - layout.dock->height()) / 2));
}

void PlanViewLayoutTest::_rotatingBackRestoresTheSideInspector()
{
    QQmlPropertyMap globals;
    QQuickView view;
    Layout layout;
    QVERIFY(load(view, globals, layout));

    resize(view, 903, 428);
    QVERIFY(settled(layout, [&] { return inspectorOnTheRight(layout); }));

    resize(view, 428, 903);
    QVERIFY(settled(layout, [&] { return inspectorUnderTheDock(layout); }));
    QVERIFY(near(layout.inspector->width(), 428 - layout.margin * 2));

    resize(view, 903, 428);
    QVERIFY(settled(layout, [&] { return inspectorOnTheRight(layout); }));
    QVERIFY(near(layout.inspector->width(), expectedWideWidth(layout)));
    QVERIFY(near(layout.dock->y(), (layout.panelHeight() - layout.dock->height()) / 2));
}
