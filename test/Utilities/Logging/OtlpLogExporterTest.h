#pragma once

#include "UnitTest.h"

class OtlpLogExporterTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _logsURL_data();
    void _logsURL();
    void _severityMapping();
    void _payloadShape();
    void _queueIsBoundedAndDropsTheOldest();
    void _nothingIsQueuedWhileDisabled();
    void _noKeyShipsWithTheApplication();
};
