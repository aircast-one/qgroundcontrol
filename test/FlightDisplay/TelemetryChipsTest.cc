/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "TelemetryChipsTest.h"

#include "QuickInteractionTestHelpers.h"
#include "FactValueGrid.h"
#include "QmlObjectListModel.h"

#include <QtQml/QQmlContext>
#include <QtQuick/QQuickItem>

static const QString kChipKeyPrefix     = QStringLiteral("TelemetryChip-");

// DragToPosition scopes every stored position by the size it was arranged at, so the keys
// carry the layer's dimensions. Seeding the unscoped key stores a position no size will ever
// look up, and the chip loads its default instead. Read off the view rather than written out,
// so resizing the test QML cannot silently divorce the keys from the app that reads them.
static QString chipSizeKey(const QQuickItem* root)
{
    return QStringLiteral("%1x%2-").arg(qRound(root->width())).arg(qRound(root->height()));
}
static const QString kGridSettingsGroup = QStringLiteral("TelemetryChips-");

static void clearChipPositionSettings()
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    const QStringList keys = settings.allKeys();
    for (const QString& key : keys) {
        if (key.startsWith(kChipKeyPrefix)) {
            settings.remove(key);
        }
    }
    settings.endGroup();
}

static void clearChipGridSettings()
{
    QSettings settings;
    const QStringList groups = settings.childGroups();
    for (const QString& group : groups) {
        if (group.startsWith(kGridSettingsGroup)) {
            settings.remove(group);
        }
    }
}

// The layer keeps its grid layout and its chip positions in global settings, so a test that
// leaves either behind changes what the next one loads.
class ChipSettingsScope
{
public:
    ChipSettingsScope()  { _clear(); }
    ~ChipSettingsScope() { _clear(); }

private:
    static void _clear()
    {
        clearChipPositionSettings();
        clearChipGridSettings();
    }
};

static QVariant chipSetting(const QString& sizeKey, const QString& uid, const QString& suffix)
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    return settings.value(kChipKeyPrefix + uid + sizeKey + suffix);
}

static QPointF storedChipPosition(const QString& sizeKey, const QString& uid)
{
    return QPointF(chipSetting(sizeKey, uid, QStringLiteral("PositionX")).toReal(),
                   chipSetting(sizeKey, uid, QStringLiteral("PositionY")).toReal());
}

// Chips animate from 0,0 to their default slot on load, so a position sampled right after the
// view appears is mid-flight. Seeding a stored position gives the chip one exact resting place
// to converge on instead.
static void seedChipPosition(const QString& sizeKey, const QString& uid, const QPointF& pos)
{
    QSettings settings;
    settings.beginGroup(QGroundControlQmlGlobal::kQmlGlobalKeyName);
    settings.setValue(kChipKeyPrefix + uid + sizeKey + QStringLiteral("CustomPosition"), true);
    settings.setValue(kChipKeyPrefix + uid + sizeKey + QStringLiteral("PositionX"), QString::number(pos.x()));
    settings.setValue(kChipKeyPrefix + uid + sizeKey + QStringLiteral("PositionY"), QString::number(pos.y()));
    // An arrangement is only found again if its size is on the roster; without this the
    // lookup falls through to the shipped defaults exactly as if nothing had been stored.
    settings.setValue(kChipKeyPrefix + uid + QStringLiteral("Sizes"),
                      QString(sizeKey).chopped(1));
}

static bool loadLayer(QQuickView& view, TelemetryChipsTestMainWindow& mainWindow)
{
    view.engine()->rootContext()->setContextProperty(QStringLiteral("mainWindow"), &mainWindow);
    return loadTestView(view, QStringLiteral("qrc:/unittest/TelemetryChipsTest.qml"));
}

// A chip owns the instrumentValueData property; the label and value inside it expose the same
// property, so the walk stops at the outermost match rather than counting each chip three times.
static void collectChips(QQuickItem* item, QList<QQuickItem*>& chips)
{
    if (!item->isVisible()) {
        return;
    }
    if (item->property("instrumentValueData").isValid()) {
        chips.append(item);
        return;
    }
    const QList<QQuickItem*> children = item->childItems();
    for (QQuickItem* child : children) {
        collectChips(child, chips);
    }
}

static QList<QQuickItem*> chipsOf(QQuickItem* root)
{
    QList<QQuickItem*> chips;
    collectChips(root, chips);
    return chips;
}

static QString chipUid(QQuickItem* chip)
{
    QObject* const data = chip->property("instrumentValueData").value<QObject*>();
    return data ? data->property("uid").toString() : QString();
}

static QQuickItem* chipWithUid(QQuickItem* root, const QString& uid)
{
    const QList<QQuickItem*> chips = chipsOf(root);
    for (QQuickItem* chip : chips) {
        if (chipUid(chip) == uid) {
            return chip;
        }
    }
    return nullptr;
}

static QObject* gridValue(FactValueGrid* grid, int columnIndex)
{
    QmlObjectListModel* const column = grid->columns()->value<QmlObjectListModel*>(columnIndex);
    return column ? column->get(0) : nullptr;
}

static QStringList gridUids(FactValueGrid* grid)
{
    QStringList uids;
    for (int i = 0; i < grid->columns()->count(); i++) {
        QObject* const value = gridValue(grid, i);
        uids.append(value ? value->property("uid").toString() : QString());
    }
    return uids;
}

static QStringList gridFactNames(FactValueGrid* grid)
{
    QStringList names;
    for (int i = 0; i < grid->columns()->count(); i++) {
        QObject* const value = gridValue(grid, i);
        names.append(value ? value->property("factName").toString() : QString());
    }
    return names;
}

void TelemetryChipsTest::_chipPerValueIsRendered()
{
    ChipSettingsScope settingsScope;

    TelemetryChipsTestMainWindow mainWindow;
    QQuickView view;
    QVERIFY(loadLayer(view, mainWindow));

    FactValueGrid* const grid = view.rootObject()->findChild<FactValueGrid*>();
    QVERIFY(grid);
    QCOMPARE(grid->property("rowCount").toInt(), 1);
    QVERIFY(grid->columns()->count() > 1);

    QTRY_COMPARE(chipsOf(view.rootObject()).count(), grid->columns()->count());

    const QList<QQuickItem*> chips = chipsOf(view.rootObject());
    QStringList uids;
    for (QQuickItem* chip : chips) {
        QVERIFY(chip->width() > 0);
        QVERIFY(chip->height() > 0);
        uids.append(chipUid(chip));
    }

    // Every chip carries its own value, so no uid may repeat and none may be blank.
    QCOMPARE(uids, gridUids(grid));
    QVERIFY(!uids.contains(QString()));
    QCOMPARE(QSet<QString>(uids.begin(), uids.end()).count(), uids.count());
}

// Two chips may legitimately show the same fact. Keying their stored position by the fact made
// them share one settings key, so dragging one moved the other on the next load.
void TelemetryChipsTest::_duplicateFactChipsKeepIndependentPositions()
{
    ChipSettingsScope settingsScope;

    TelemetryChipsTestMainWindow mainWindow;

    QString uidA;
    QString uidB;
    QString sizeKey;

    {
        QQuickView view;
        QVERIFY(loadLayer(view, mainWindow));

        FactValueGrid* const grid = view.rootObject()->findChild<FactValueGrid*>();
        QVERIFY(grid);
        while (grid->columns()->count() > 2) {
            grid->deleteColumn(grid->columns()->count() - 1);
        }
        QCOMPARE(grid->columns()->count(), 2);

        for (int i = 0; i < 2; i++) {
            QObject* const value = gridValue(grid, i);
            QVERIFY(value);
            QVERIFY(QMetaObject::invokeMethod(value, "setFact",
                                              Q_ARG(QString, QStringLiteral("Vehicle")),
                                              Q_ARG(QString, QStringLiteral("AltitudeRelative"))));
        }
        QCOMPARE(gridFactNames(grid), QStringList({ QStringLiteral("AltitudeRelative"), QStringLiteral("AltitudeRelative") }));

        QTRY_COMPARE(chipsOf(view.rootObject()).count(), 2);
        const QList<QQuickItem*> chips = chipsOf(view.rootObject());
        uidA = chipUid(chips[0]);
        uidB = chipUid(chips[1]);
        sizeKey = chipSizeKey(view.rootObject());
        QVERIFY(!uidA.isEmpty());
        QVERIFY(uidA != uidB);
    }

    const QPointF seededA(120, 300);
    const QPointF seededB(500, 300);
    seedChipPosition(sizeKey, uidA, seededA);
    seedChipPosition(sizeKey, uidB, seededB);

    QPointF droppedA;

    {
        QQuickView view;
        QVERIFY(loadLayer(view, mainWindow));
        view.rootObject()->setProperty("editMode", true);

        QTRY_COMPARE(chipsOf(view.rootObject()).count(), 2);
        QQuickItem* const chipA = chipWithUid(view.rootObject(), uidA);
        QQuickItem* const chipB = chipWithUid(view.rootObject(), uidB);
        QVERIFY(chipA);
        QVERIFY(chipB);

        // Each chip found its own stored position, so the two never shared one key on load.
        QTRY_COMPARE(QPointF(chipA->x(), chipA->y()), seededA);
        QTRY_COMPARE(QPointF(chipB->x(), chipB->y()), seededB);

        const QPoint start = itemCenter(chipA);
        dragMouse(view, start, start + QPoint(200, -150), false);

        QCOMPARE(QPointF(chipB->x(), chipB->y()), seededB);

        // The write is debounced, so the settings still hold the seeded position for a moment
        // after the drop. Reading straight away compares the seed against itself.
        QTRY_VERIFY(storedChipPosition(sizeKey, uidA) != seededA);
        droppedA = storedChipPosition(sizeKey, uidA);
        QVERIFY(chipSetting(sizeKey, uidA, QStringLiteral("CustomPosition")).toBool());

        // The drop wrote only the dragged chip's key. The pre-fix key both chips shared must
        // not exist at all.
        QCOMPARE(storedChipPosition(sizeKey, uidB), seededB);
        QVERIFY(!chipSetting(sizeKey, QStringLiteral("Vehicle-AltitudeRelative"),
                             QStringLiteral("CustomPosition")).isValid());

        QTRY_COMPARE(QPointF(chipA->x(), chipA->y()), droppedA);
        QCOMPARE(QPointF(chipB->x(), chipB->y()), seededB);
    }

    QQuickView view2;
    QVERIFY(loadLayer(view2, mainWindow));

    QTRY_COMPARE(chipsOf(view2.rootObject()).count(), 2);
    QQuickItem* const restoredA = chipWithUid(view2.rootObject(), uidA);
    QQuickItem* const restoredB = chipWithUid(view2.rootObject(), uidB);
    QVERIFY(restoredA);
    QVERIFY(restoredB);

    QTRY_COMPARE(QPointF(restoredA->x(), restoredA->y()), droppedA);
    QTRY_COMPARE(QPointF(restoredB->x(), restoredB->y()), seededB);
    QVERIFY(QPointF(restoredA->x(), restoredA->y()) != QPointF(restoredB->x(), restoredB->y()));
}

// The edit badge deletes the column it sits on, which is not necessarily the last one.
void TelemetryChipsTest::_deleteColumnRemovesTargetedChip()
{
    ChipSettingsScope settingsScope;

    TelemetryChipsTestMainWindow mainWindow;
    QQuickView view;
    QVERIFY(loadLayer(view, mainWindow));

    FactValueGrid* const grid = view.rootObject()->findChild<FactValueGrid*>();
    QVERIFY(grid);
    QVERIFY(grid->columns()->count() >= 3);

    const QStringList uidsBefore      = gridUids(grid);
    const QStringList factNamesBefore = gridFactNames(grid);
    QTRY_COMPARE(chipsOf(view.rootObject()).count(), uidsBefore.count());

    grid->deleteColumn(0);

    QCOMPARE(gridUids(grid), uidsBefore.mid(1));
    QCOMPARE(gridFactNames(grid), factNamesBefore.mid(1));
    QVERIFY(!gridFactNames(grid).contains(factNamesBefore.first()));

    QTRY_COMPARE(chipsOf(view.rootObject()).count(), uidsBefore.count() - 1);

    const QList<QQuickItem*> chips = chipsOf(view.rootObject());
    QStringList uidsAfter;
    for (QQuickItem* chip : chips) {
        uidsAfter.append(chipUid(chip));
    }
    QCOMPARE(uidsAfter, uidsBefore.mid(1));
    QVERIFY(!chipWithUid(view.rootObject(), uidsBefore.first()));
}

static QQuickItem *resetPillOf(QQuickItem *root)
{
    return root->findChild<QQuickItem*>(QStringLiteral("editModeResetPill"));
}

// Reset Layout wipes every stored position and unhides everything, with no undo, from a pill
// the user reaches past on their way to Done. One tap must never be enough.
void TelemetryChipsTest::_resetLayoutNeedsTwoTaps()
{
    QQuickView view;
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/TelemetryChipsTest.qml")));
    view.rootObject()->setProperty("editMode", true);

    QQuickItem *const pill = resetPillOf(view.rootObject());
    QVERIFY(pill);
    QTRY_VERIFY(pill->isVisible() && pill->width() > 0);
    const QString idleText = pill->property("text").toString();

    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pill));
    QTRY_VERIFY(pill->property("text").toString() != idleText);
    QCOMPARE(view.rootObject()->property("resetCount").toInt(), 0);

    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pill));
    QTRY_COMPARE(view.rootObject()->property("resetCount").toInt(), 1);
    QCOMPARE(pill->property("text").toString(), idleText);
}

// An arm left standing put a single tap between a returning user and a wiped layout.
void TelemetryChipsTest::_leavingEditModeDisarmsReset()
{
    QQuickView view;
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/TelemetryChipsTest.qml")));
    view.rootObject()->setProperty("editMode", true);

    QQuickItem *const pill = resetPillOf(view.rootObject());
    QVERIFY(pill);
    QTRY_VERIFY(pill->isVisible() && pill->width() > 0);
    const QString idleText = pill->property("text").toString();

    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pill));
    QTRY_VERIFY(pill->property("text").toString() != idleText);

    view.rootObject()->setProperty("editMode", false);
    QTRY_VERIFY(!pill->isVisible());
    view.rootObject()->setProperty("editMode", true);
    QTRY_VERIFY(pill->isVisible());

    QCOMPARE(pill->property("text").toString(), idleText);
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pill));
    QCOMPARE(view.rootObject()->property("resetCount").toInt(), 0);
}
