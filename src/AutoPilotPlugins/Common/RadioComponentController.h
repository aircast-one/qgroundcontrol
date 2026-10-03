#pragma once

#include "RemoteControlCalibrationController.h"

class RadioComponentController : public RemoteControlCalibrationController
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(bool throttleReversed READ throttleReversed NOTIFY throttleReversedChanged)

public:
    RadioComponentController(QObject *parent = nullptr);
    ~RadioComponentController();

    static RadioComponentController *forActiveVehicle();

    enum BindModes {
        DSM2,
        DSMX7,
        DSMX8
    };
    Q_ENUM(BindModes)

    Q_INVOKABLE void spektrumBindMode(int mode);
    Q_INVOKABLE void crsfBindMode();

    bool throttleReversed() const { return _throttleReversed; }

    void start() final override;

signals:
    void throttleReversedCalFailure();
    void throttleReversedChanged();

private:
    QString _stickFunctionToParamName(RemoteControlCalibrationController::StickFunction function) const;
    bool _channelReversedParamValue(int channel);
    void _setChannelReversedParamValue(int channel, bool reversed);

    void _saveStoredCalibrationValues() override;
    void _readStoredCalibrationValues() override;

    QString _revParamFormat;
    bool _revParamIsBool = false;
    bool _throttleReversed = false;
};
