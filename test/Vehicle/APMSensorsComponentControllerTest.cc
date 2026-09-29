#include "APMSensorsComponentControllerTest.h"

#include <QtCore/QRegularExpression>

#include "APMSensorsComponentController.h"

void APMSensorsComponentControllerTest::_setupNeededIsFalseBeforeParametersLoad()
{
    ignoreLogMessage("AutoPilotPlugins.APM.apmautopilotplugin", QtWarningMsg,
                     QRegularExpression("prior to parametersReady"));
    ignoreLogMessage("AutoPilotPlugins.APMSensorsComponentController", QtWarningMsg,
                     QRegularExpression("Sensors component is missing"));

    APMSensorsComponentController controller(this);

    QVERIFY(!controller.compassSetupNeeded());
    QVERIFY(!controller.accelSetupNeeded());
}

UT_REGISTER_TEST(APMSensorsComponentControllerTest, TestLabel::Integration, TestLabel::Vehicle)
