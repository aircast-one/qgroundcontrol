#pragma once

#include <QtCore/QVariant>

#include "UnitTest.h"

class UsbRadioOfferTest : public UnitTest
{
    Q_OBJECT

private slots:
    void init() override;
    void cleanup() override;

    void _aStreamingPortIsOfferedOnceAndNotNowKeepsItOffTheLinks_test();
    void _aRadioPluggedBackInIsOfferedAgain_test();
    void _useFliesWithThePortAndAChosenRadioIsNotOfferedAnother_test();
    void _anOlderDialogUsesItsOwnPortAndDoesNotCloseANewerOffer_test();

private:
    QVariant _savedPort;
};
