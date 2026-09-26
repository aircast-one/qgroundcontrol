target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCBridgeCoreTest.cc
        QGCBridgeCoreTest.h
)

if(TARGET qgc_core_cargo)
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            QGCCoreCTest.cc
            QGCCoreCTest.h
    )
    # The test finds its fixtures through __FILE__ and records checkout-relative keys by replacing
    # the checkout root in them. Under QGC_REPRODUCIBLE_BUILD's -ffile-prefix-map that root cleans to
    # ".", every dot in every key is replaced, and nothing resolves; the last map given wins.
    set(_qgc_core_test_options -Wno-shadow)
    if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
        list(APPEND _qgc_core_test_options "-ffile-prefix-map=${CMAKE_SOURCE_DIR}=${CMAKE_SOURCE_DIR}")
    endif()
    set_source_files_properties(QGCCoreCTest.cc TARGET_DIRECTORY ${CMAKE_PROJECT_NAME} PROPERTIES COMPILE_OPTIONS "${_qgc_core_test_options}")
    add_qgc_test(QGCCoreCTest LABELS Unit TIMEOUT 1800)
endif()

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
add_qgc_test(QGCBridgeCoreTest LABELS Unit)
