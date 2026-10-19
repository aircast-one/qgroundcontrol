#include "UsbRadioOfferTest.h"

#include <QtTest/QSignalSpy>

#include "AutoConnectSettings.h"
#include "Fact.h"
#include "SettingsManager.h"
#include "UsbRadio.h"

namespace {

const QString kPort = QStringLiteral("/dev/cu.usbmodemRADIOTEST1");
const QString kOther = QStringLiteral("/dev/cu.usbmodemRADIOTEST2");

Fact* chosenPort()
{
    return SettingsManager::instance()->autoConnectSettings()->usbRadioPort();
}

void forgetRadios()
{
    UsbRadio::instance()->portsPresent({});
    UsbRadio::instance()->dismissOffer(UsbRadio::instance()->offerPort());
}

}  // namespace

void UsbRadioOfferTest::init()
{
    UnitTest::init();
    _savedPort = chosenPort()->rawValue();
    chosenPort()->setRawValue(QString());
    forgetRadios();
}

void UsbRadioOfferTest::cleanup()
{
    forgetRadios();
    chosenPort()->setRawValue(_savedPort);
    UnitTest::cleanup();
}

void UsbRadioOfferTest::_aStreamingPortIsOfferedOnceAndNotNowKeepsItOffTheLinks_test()
{
    UsbRadio* const radio = UsbRadio::instance();
    QSignalSpy offers(radio, &UsbRadio::offerChanged);
    radio->radioSeen(kPort);
    QCOMPARE(radio->offerPort(), kPort);
    QCOMPARE(offers.count(), 1);
    QVERIFY(radio->isRadioPort(kPort));
    radio->dismissOffer(kPort);
    QCOMPARE(radio->offerPort(), QString());
    radio->radioSeen(kPort);
    QCOMPARE(radio->offerPort(), QString());
    QVERIFY(radio->isRadioPort(kPort));
    QCOMPARE(chosenPort()->rawValue().toString(), QString());
}

void UsbRadioOfferTest::_aRadioPluggedBackInIsOfferedAgain_test()
{
    UsbRadio* const radio = UsbRadio::instance();
    radio->radioSeen(kPort);
    radio->dismissOffer(kPort);
    radio->portsPresent({});
    QVERIFY(!radio->isRadioPort(kPort));
    radio->portsPresent({kPort});
    radio->radioSeen(kPort);
    QCOMPARE(radio->offerPort(), kPort);
    radio->portsPresent({});
    QCOMPARE(radio->offerPort(), QString());
}

void UsbRadioOfferTest::_useFliesWithThePortAndAChosenRadioIsNotOfferedAnother_test()
{
    UsbRadio* const radio = UsbRadio::instance();
    radio->radioSeen(kPort);
    radio->useOffer(kPort);
    QCOMPARE(chosenPort()->rawValue().toString(), kPort);
    QCOMPARE(radio->offerPort(), QString());
    QVERIFY(radio->isRadioPort(kPort));
    radio->radioSeen(kOther);
    QCOMPARE(radio->offerPort(), QString());
}

void UsbRadioOfferTest::_anOlderDialogUsesItsOwnPortAndDoesNotCloseANewerOffer_test()
{
    UsbRadio* const radio = UsbRadio::instance();
    radio->portsPresent({kPort, kOther});
    radio->radioSeen(kPort);
    radio->dismissOffer(kOther);
    QCOMPARE(radio->offerPort(), kPort);
    radio->useOffer(kOther);
    QCOMPARE(chosenPort()->rawValue().toString(), kOther);
    QCOMPARE(radio->offerPort(), QString());
}

UT_REGISTER_TEST(UsbRadioOfferTest, TestLabel::Unit, TestLabel::Joystick)
