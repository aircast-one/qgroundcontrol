#pragma once

#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QPointer>

#include "MAVLinkLib.h"

Q_DECLARE_LOGGING_CATEGORY(AirUnitCameraControlLog)

class LinkInterface;

class AirUnitCameraControl : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool available   READ available   NOTIFY availableChanged)
    Q_PROPERTY(int  activeInput READ activeInput NOTIFY activeInputChanged)
    Q_PROPERTY(int  inputCount  READ inputCount  CONSTANT)

public:
    explicit AirUnitCameraControl(QObject *parent = nullptr);

    bool available() const { return _link && !_link.isNull(); }
    int activeInput() const { return _activeInput; }
    int inputCount() const { return kInputCount; }

    Q_INVOKABLE void selectInput(int input);
    Q_INVOKABLE void switchInput();

    void handleMessage(LinkInterface *link, const mavlink_message_t &message);

    static bool isCameraComponent(int componentId);
    static int cameraIdFromLegacyStreamInformation(const mavlink_message_t &message);

    static constexpr int kInputCount = 2;

signals:
    void availableChanged();
    void activeInputChanged();

protected:
    virtual void _sendMessage(LinkInterface *link, const mavlink_message_t &message);

private:
    void _sendCommand(uint16_t command, float param1);
    void _requestStreamInformation();
    void _setActiveInput(int input);

    QPointer<LinkInterface> _link;
    int _systemId = 0;
    int _componentId = 0;
    int _activeInput = -1;
    int _previousInput = -1;
};
