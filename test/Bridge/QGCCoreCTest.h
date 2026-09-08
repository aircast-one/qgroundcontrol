#pragma once

#include "UnitTest.h"

class QGCCoreCTest : public UnitTest
{
    Q_OBJECT

private slots:
    void init();
    void cleanup();

    void _viewMessagesReachTheHeadThroughTheRustCore();
    void _viewPathsAreReadOnlyAtTheCAbi();
    void _viewFieldsProjectAndNameTheUnknown();
    void _clientsWatchIndependently();
};
