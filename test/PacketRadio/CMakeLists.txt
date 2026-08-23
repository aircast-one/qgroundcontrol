target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        PacketRadioTest.cc
        PacketRadioTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME}
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/libs/wfb
        ${CMAKE_SOURCE_DIR}/libs/wfb/wifi
        ${CMAKE_SOURCE_DIR}/libs/wfb/devourer/src
)
