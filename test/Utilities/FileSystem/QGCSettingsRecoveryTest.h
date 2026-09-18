#pragma once

#include "UnitTest.h"

class QGCSettingsRecoveryTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _unreadableFileIsReplaced();
    void _readOnlyFileKeepsItsValues();
    void _writableFileIsLeftAlone();
};
