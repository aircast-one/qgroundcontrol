#pragma once

#include "UnitTest.h"

class AppSettingsLayoutTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _phoneUprightStacksTheListAndThePage();
    void _phoneSidewaysKeepsBothColumns();
    void _desktopKeepsBothColumnsAndFloats();
};
