target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        Joystick.cc
        Joystick.h
        JoystickManager.cc
        JoystickManager.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        JoystickSDL.cc
        JoystickSDL.h
)

if(NOT QGC_NO_SERIAL_LINK)
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            UsbRadio.cc
            UsbRadio.h
    )
    qt_add_resources(${CMAKE_PROJECT_NAME} "edgetx_script"
        PREFIX "/edgetx"
        BASE "${CMAKE_SOURCE_DIR}/tools/edgetx"
        FILES "${CMAKE_SOURCE_DIR}/tools/edgetx/aircast-usb.lua"
    )
endif()
