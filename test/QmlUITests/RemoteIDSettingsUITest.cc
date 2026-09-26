#include "RemoteIDSettingsUITest.h"

#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "Fact.h"
#include "RemoteIDSettings.h"
#include "SettingsManager.h"

namespace {

constexpr const char* kValidFullOperatorID = "FIN87astrdge12k8-xyz";
constexpr const char* kValidPublicOperatorID = "FIN87astrdge12k8";
constexpr const char* kInvalidOperatorID = "DEADBEEFDEADBEEFDEAD";
constexpr const char* kEUFieldObjectName = "settingsTextField_operatorIDEU";
constexpr const char* kFAAFieldObjectName = "settingsTextField_operatorIDFAA";

}

UT_REGISTER_TEST(RemoteIDSettingsUITest, TestLabel::Integration)

void RemoteIDSettingsUITest::init()
{
    UnitTest::init();

    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();

    _savedRegion = settings->region()->rawValue();
    _savedOperatorIDType = settings->operatorIDType()->rawValue();
    _savedOperatorIDEU = settings->operatorIDEU()->rawValue();
    _savedOperatorIDFAA = settings->operatorIDFAA()->rawValue();
    _savedSendOperatorID = settings->sendOperatorID()->rawValue();
    _savedLocationType = settings->locationType()->rawValue();

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::EU));
    settings->operatorIDType()->setRawValue(0);
    settings->operatorIDEU()->setRawValue(QString());
    settings->operatorIDFAA()->setRawValue(QString());
}

void RemoteIDSettingsUITest::cleanup()
{
    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();

    settings->region()->setRawValue(_savedRegion);
    settings->operatorIDType()->setRawValue(_savedOperatorIDType);
    settings->operatorIDEU()->setRawValue(_savedOperatorIDEU);
    settings->operatorIDFAA()->setRawValue(_savedOperatorIDFAA);
    settings->sendOperatorID()->setRawValue(_savedSendOperatorID);
    settings->locationType()->setRawValue(_savedLocationType);

    QmlUITestBase::cleanup();
}

bool RemoteIDSettingsUITest::_navigateToRemoteIDPage()
{
    return openSettingsPage(QStringLiteral("Remote ID"));
}

QQuickItem* RemoteIDSettingsUITest::_operatorIDTextField(const QString& wrapperObjectName)
{
    QQuickItem* wrapper = findVisibleItem(_rootItem, wrapperObjectName);
    if (!wrapper) {
        QTest::qFail(qPrintable(QStringLiteral("Operator ID field wrapper not found: %1").arg(wrapperObjectName)), __FILE__, __LINE__);
        return nullptr;
    }

    QQuickItem* textField = qvariant_cast<QQuickItem*>(wrapper->property("textField"));
    if (!textField) {
        QTest::qFail("Inner FactTextField not found on operator ID wrapper", __FILE__, __LINE__);
        return nullptr;
    }
    return textField;
}

bool RemoteIDSettingsUITest::_typeIntoField(QQuickItem* field, const QString& text)
{
    scrollIntoView(field, QStringLiteral("settingsPageFlickable"));

    const QPointF center = field->mapToScene(QPointF(field->width() / 2, field->height() / 2));
    QTest::mouseClick(_window, Qt::LeftButton, Qt::NoModifier, center.toPoint());
    if (!QTest::qWaitFor([field]() { return field->hasActiveFocus(); }, 2000)) {
        QTest::qFail("Operator ID text field did not gain focus", __FILE__, __LINE__);
        return false;
    }
    QMetaObject::invokeMethod(field, "selectAll");

    for (const QChar& c : text) {
        QTest::keyClick(_window, c.toLatin1());
    }
    QTest::keyClick(_window, Qt::Key_Return);
    return true;
}

void RemoteIDSettingsUITest::_testInvalidOperatorIDShowsError()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    QQuickItem* field = _operatorIDTextField(QString::fromLatin1(kEUFieldObjectName));
    QVERIFY(field);

    QVERIFY2(!field->property("validationError").toBool(), "Validation error shown with empty operator ID");

    QVERIFY(_typeIntoField(field, QString::fromLatin1(kInvalidOperatorID)));

    QTRY_VERIFY2_WITH_TIMEOUT(field->property("validationError").toBool(),
                              "Validation error not shown for invalid operator ID", 2000);

    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();
    QVERIFY2(settings->operatorIDEU()->rawValue().toString().isEmpty(),
             "Invalid operator ID was stored in the fact");

    QVERIFY(_typeIntoField(field, QStringLiteral("STILLNOTVALIDEITHER1")));
    QTRY_VERIFY2_WITH_TIMEOUT(field->property("validationError").toBool(),
                              "Validation error lost after editing invalid ID into another invalid ID", 2000);
    QVERIFY2(settings->operatorIDEU()->rawValue().toString().isEmpty(),
             "Invalid operator ID was stored in the fact");

    stopUI();
}

void RemoteIDSettingsUITest::_testValidOperatorIDClearsErrorAndSanitizes()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    QQuickItem* field = _operatorIDTextField(QString::fromLatin1(kEUFieldObjectName));
    QVERIFY(field);

    QVERIFY(_typeIntoField(field, QString::fromLatin1(kInvalidOperatorID)));
    QTRY_VERIFY2_WITH_TIMEOUT(field->property("validationError").toBool(),
                              "Validation error not shown for invalid operator ID", 2000);

    QVERIFY(_typeIntoField(field, QString::fromLatin1(kValidFullOperatorID)));

    QTRY_VERIFY2_WITH_TIMEOUT(!field->property("validationError").toBool(),
                              "Validation error still shown for valid operator ID", 2000);

    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();
    QTRY_COMPARE_WITH_TIMEOUT(settings->operatorIDEU()->rawValue().toString(),
                              QString::fromLatin1(kValidPublicOperatorID), 2000);
    QTRY_COMPARE_WITH_TIMEOUT(field->property("text").toString(),
                              QString::fromLatin1(kValidPublicOperatorID), 2000);

    stopUI();
}

void RemoteIDSettingsUITest::_testMaximumLengthEnforced()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    QQuickItem* field = _operatorIDTextField(QString::fromLatin1(kEUFieldObjectName));
    QVERIFY(field);

    QCOMPARE(field->property("maximumLength").toInt(), 20);

    QVERIFY(_typeIntoField(field, QStringLiteral("123456789012345678901234")));
    QCOMPARE(field->property("text").toString().length(), 20);

    stopUI();
}

void RemoteIDSettingsUITest::_testRegionSwitchSwapsOperatorIDFields()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    QQuickItem* euField = _operatorIDTextField(QString::fromLatin1(kEUFieldObjectName));
    QVERIFY(euField);
    QVERIFY2(!findVisibleItem(_rootItem, QString::fromLatin1(kFAAFieldObjectName)),
             "FAA operator ID field visible in EU region");

    QVERIFY(_typeIntoField(euField, QString::fromLatin1(kValidFullOperatorID)));
    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();
    QTRY_COMPARE_WITH_TIMEOUT(settings->operatorIDEU()->rawValue().toString(),
                              QString::fromLatin1(kValidPublicOperatorID), 2000);

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::FAA));
    QQuickItem* faaField = nullptr;
    QTRY_VERIFY2_WITH_TIMEOUT((faaField = findVisibleItem(_rootItem, QString::fromLatin1(kFAAFieldObjectName))) != nullptr,
                              "FAA operator ID field not visible in FAA region", 2000);
    QTRY_VERIFY2_WITH_TIMEOUT(!findVisibleItem(_rootItem, QString::fromLatin1(kEUFieldObjectName)),
                              "EU operator ID field still visible in FAA region", 2000);

    faaField = _operatorIDTextField(QString::fromLatin1(kFAAFieldObjectName));
    QVERIFY(faaField);
    QVERIFY2(faaField->property("text").toString().isEmpty(), "FAA field not empty after region switch");

    QVERIFY(_typeIntoField(faaField, QString::fromLatin1(kInvalidOperatorID)));
    QTRY_COMPARE_WITH_TIMEOUT(settings->operatorIDFAA()->rawValue().toString(),
                              QString::fromLatin1(kInvalidOperatorID), 2000);

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::EU));
    QTRY_VERIFY2_WITH_TIMEOUT(findVisibleItem(_rootItem, QString::fromLatin1(kEUFieldObjectName)) != nullptr,
                              "EU operator ID field not restored in EU region", 2000);
    euField = _operatorIDTextField(QString::fromLatin1(kEUFieldObjectName));
    QVERIFY(euField);
    QCOMPARE(euField->property("text").toString(), QString::fromLatin1(kValidPublicOperatorID));
    QCOMPARE(settings->operatorIDFAA()->rawValue().toString(), QString::fromLatin1(kInvalidOperatorID));

    stopUI();
}

void RemoteIDSettingsUITest::_testFAARegionForcesLiveLocationInUI()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();
    settings->locationType()->setRawValue(static_cast<int>(RemoteIDSettings::LocationType::FIXED));

    QVERIFY(verifyEnabled(QStringLiteral("settingsTextField_latitudeFixed"), true, QStringLiteral("EU region, FIXED location")));

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::FAA));

    QCOMPARE(settings->locationType()->rawValue().toInt(), static_cast<int>(RemoteIDSettings::LocationType::LIVE));
    QVERIFY(verifyEnabled(QStringLiteral("settingsTextField_latitudeFixed"), false, QStringLiteral("FAA region forces LIVE")));

    stopUI();
}

void RemoteIDSettingsUITest::_testEUVehicleInfoGroupFollowsRegion()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    QQuickItem* euGroup = findVisibleItem(_rootItem, QStringLiteral("settingsGroup_EUVehicleInfo"));
    QVERIFY2(euGroup, "EU Vehicle Info group not visible in EU region");

    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::FAA));
    QTRY_VERIFY2_WITH_TIMEOUT(!euGroup->isVisible(), "EU Vehicle Info group still visible in FAA region", 2000);

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::EU));
    QTRY_VERIFY2_WITH_TIMEOUT(euGroup->isVisible(), "EU Vehicle Info group not restored in EU region", 2000);

    stopUI();
}

void RemoteIDSettingsUITest::_testEURegionForcesOperatorIDBroadcastInUI()
{
    startUI();
    if (QTest::currentTestFailed()) return;

    QVERIFY(_navigateToRemoteIDPage());

    RemoteIDSettings* settings = SettingsManager::instance()->remoteIDSettings();

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::FAA));
    settings->sendOperatorID()->setRawValue(false);
    QVERIFY(verifyEnabled(QStringLiteral("settingsCheckBox_sendOperatorID"), true, QStringLiteral("FAA region")));
    QVERIFY(verifyChecked(QStringLiteral("settingsCheckBox_sendOperatorID"), false, QStringLiteral("FAA region, broadcast off")));

    settings->region()->setRawValue(static_cast<int>(RemoteIDSettings::RegionOperation::EU));
    QVERIFY(verifyChecked(QStringLiteral("settingsCheckBox_sendOperatorID"), true, QStringLiteral("EU region forces broadcast")));
    QVERIFY(verifyEnabled(QStringLiteral("settingsCheckBox_sendOperatorID"), false, QStringLiteral("EU region locks broadcast")));

    stopUI();
}
