# ============================================================================
# Communications Unit Tests
# Tests for serial port detection and communication links
# ============================================================================

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        LinkConfigurationTest.cc
        LinkConfigurationTest.h
        LinkManagerTest.cc
        LinkManagerTest.h
        LogReplayLinkControllerTest.cc
        LogReplayLinkControllerTest.h
        LogReplayLinkTest.cc
        LogReplayLinkTest.h
        MAVLinkV1TrafficTest.cc
        MAVLinkV1TrafficTest.h
        QGCSerialPortInfoTest.cc
        SkydroidH16LinksTest.cc
        SkydroidH16LinksTest.h
        QGCSerialPortInfoTest.h
        TCPConfigurationTest.cc
        TCPConfigurationTest.h
        TCPLinkErrorTest.cc
        TCPLinkErrorTest.h
        LinkStateTest.cc
        LinkStateTest.h
        LinkDuplicateConnectTest.cc
        LinkDuplicateConnectTest.h
        UDPConfigurationTest.cc
        UDPConfigurationTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../QmlControls)
add_subdirectory(Bluetooth)

add_qgc_test(LinkConfigurationTest LABELS Unit Comms RESOURCE_LOCK Settings TempFiles)
add_qgc_test(LinkManagerTest LABELS Integration Comms SERIAL)
add_qgc_test(LogReplayLinkControllerTest LABELS Unit Comms)
add_qgc_test(LogReplayLinkTest LABELS Unit Comms)
add_qgc_test(MAVLinkV1TrafficTest LABELS Integration Comms)
add_qgc_test(QGCSerialPortInfoTest LABELS Unit Comms)
add_qgc_test(LinkDuplicateConnectTest LABELS Unit)
add_qgc_test(LinkStateTest LABELS Unit)
add_qgc_test(TCPConfigurationTest LABELS Unit)
add_qgc_test(TCPLinkErrorTest LABELS Unit)
add_qgc_test(UDPConfigurationTest LABELS Unit)
