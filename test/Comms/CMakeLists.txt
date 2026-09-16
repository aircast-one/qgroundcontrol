target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCSerialPortInfoTest.cc
        SkydroidH16LinksTest.cc
        SkydroidH16LinksTest.h
        QGCSerialPortInfoTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
