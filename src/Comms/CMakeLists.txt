target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        LinkConfiguration.cc
        LinkConfiguration.h
        LinkInterface.cc
        LinkInterface.h
        LinkManager.cc
        RadiomasterAx12.cc
        RadiomasterAx12.h
        SkydroidH16Links.cc
        SkydroidH16Links.h
        LinkManager.h
        LogReplayLink.cc
        LogReplayLink.h
        LogReplayLinkController.cc
        LogReplayLinkController.h
        MAVLinkProtocol.cc
        MAVLinkProtocol.h
        TCPLink.cc
        TCPLink.h
        UdpIODevice.cc
        UdpIODevice.h
        UDPLink.cc
        UDPLink.h
)

qt_add_resources(${CMAKE_PROJECT_NAME} json_comm
    PREFIX "/json"
    FILES USBBoardInfo.json
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

if(QGC_NO_SERIAL_LINK)
    target_compile_definitions(${CMAKE_PROJECT_NAME} PRIVATE QGC_NO_SERIAL_LINK)
else()
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            QGCSerialPortInfo.cc
            QGCSerialPortInfo.h
            SerialLink.cc
            SerialLink.h
    )

    if(NOT ANDROID)
        target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE Qt6::SerialPort)
    endif()
endif()

add_subdirectory(Bluetooth)

add_subdirectory(MockLink)
