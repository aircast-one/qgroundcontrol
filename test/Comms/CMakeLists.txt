target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCSerialPortInfoTest.cc
        QGCSerialPortInfoTest.h
        TCPConfigurationTest.cc
        TCPConfigurationTest.h
        TCPLinkErrorTest.cc
        TCPLinkErrorTest.h
        LinkStateTest.cc
        LinkStateTest.h
        UDPConfigurationTest.cc
        UDPConfigurationTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../QmlControls)
