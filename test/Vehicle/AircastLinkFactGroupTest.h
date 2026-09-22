#pragma once

#include "UnitTest.h"

class AircastLinkFactGroupTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _cellularStatusFillsTheFacts();
    void _unknownQualityIsAGapInTheHistory();
    void _historyKeepsAnHourAtFiveSeconds();
};
