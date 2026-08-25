cmake_minimum_required(VERSION 3.15)

# Set the toolchain file for vcpkg on Windows (only if VCPKG_ROOT is set;
# on mingw we get libusb from pkg-config directly).
if(WIN32 AND NOT "$ENV{VCPKG_ROOT}" STREQUAL "")
    set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake")
endif()

project(devourer)

enable_testing()

# ---------------------------------------------------------------------------
# Per-chip build options. All default ON (backward compatible: a plain
# `cmake -S . -B build` builds every chip). Turn off the ones you don't need to
# drop their firmware blobs + generated PHY tables and shrink the binary — e.g.
# an 8812AU-only build:
#   cmake -S . -B build -DDEVOURER_8814=OFF \
#         -DDEVOURER_JAGUAR2_8822B=OFF -DDEVOURER_JAGUAR2_8821C=OFF \
#         -DDEVOURER_JAGUAR3_8822C=OFF -DDEVOURER_JAGUAR3_8822E=OFF
# ---------------------------------------------------------------------------
option(DEVOURER_JAGUAR1       "RTL8812AU / 8811AU / 8821AU (Jaguar1 core)"      ON)
option(DEVOURER_8814          "RTL8814AU (4T4R; requires DEVOURER_JAGUAR1)"     ON)
option(DEVOURER_JAGUAR2_8822B "RTL8822BU (Jaguar2, rtl8822b, 2T2R USB)"         ON)
option(DEVOURER_JAGUAR2_8821C "RTL8811CU / 8821CU (Jaguar2, rtl8821c, 1T1R)"    ON)
option(DEVOURER_JAGUAR3_8822C "RTL8812CU / 8822CU (Jaguar3, rtl8822c)"          ON)
option(DEVOURER_JAGUAR3_8822E "RTL8812EU / 8822EU (Jaguar3, rtl8822e)"          ON)
# Kestrel — the Wi-Fi 6 / 802.11ax generation (Realtek "G6 phl" architecture,
# vendor trees reference/rtl8852bu + reference/rtl8852cu): monitor RX, TX
# injection and channel/BW on-air validated on both chips (see
# src/kestrel/CLAUDE.md).
option(DEVOURER_KESTREL_8852B "RTL8852BU / 8832BU (Kestrel 11ax, G6)"           ON)
option(DEVOURER_KESTREL_8852C "RTL8852CU / 8832CU (Kestrel 11ax, G6)"           ON)
# PCIe transport (vfio-pci). Linux-only. The transport implements the HalMAC-
# generation (Jaguar2/3) 88xx buffer-descriptor DMA rings; the RTL8821CE is
# the only chip with a PCIe bring-up implemented + hardware-validated so far,
# hence the DEVOURER_JAGUAR2_8821C requirement below. Other HalMAC PCIe parts
# (RTL8822BE / RTL8822CE) could ride the same rings with per-chip HAL gates;
# Jaguar1 PCIe parts (RTL8812AE/8821AE) CANNOT — they are pre-HalMAC silicon
# with the older rtlwifi-style own-bit descriptor DMA, a different engine.
# OFF by default — libusb-only builds are bit-identical.
option(DEVOURER_PCIE          "PCIe transport via vfio-pci (Linux; RTL8821CE)"  OFF)

# Compile-time diagnostics floor (src/logger.h). Calls below the floor —
# including their argument expressions at DVR_TRACE/DVR_DEBUG sites — compile
# to nothing. Empty (the default) keeps the NDEBUG-derived behavior: release
# builds drop trace/debug, debug builds keep everything. Example production
# firmware build: -DDEVOURER_LOG_MAX_LEVEL=WARN
set(DEVOURER_LOG_MAX_LEVEL "" CACHE STRING
    "Compile-time log floor: TRACE, DEBUG, INFO, WARN, ERROR or SILENT (empty = NDEBUG-derived)")
set_property(CACHE DEVOURER_LOG_MAX_LEVEL PROPERTY STRINGS
    "" TRACE DEBUG INFO WARN ERROR SILENT)
if(NOT DEVOURER_LOG_MAX_LEVEL STREQUAL "")
    set(_devourer_ll_valid TRACE DEBUG INFO WARN ERROR SILENT)
    if(NOT DEVOURER_LOG_MAX_LEVEL IN_LIST _devourer_ll_valid)
        message(FATAL_ERROR
            "DEVOURER_LOG_MAX_LEVEL=${DEVOURER_LOG_MAX_LEVEL} — must be one of "
            "TRACE, DEBUG, INFO, WARN, ERROR, SILENT (or empty).")
    endif()
endif()

if(DEVOURER_8814 AND NOT DEVOURER_JAGUAR1)
    message(FATAL_ERROR
        "DEVOURER_8814=ON requires DEVOURER_JAGUAR1=ON (8814 reuses the Jaguar1 HAL).")
endif()
# Convenience: the shared Jaguar2 HAL compiles when either chip variant is on.
if(DEVOURER_JAGUAR2_8822B OR DEVOURER_JAGUAR2_8821C)
    set(DEVOURER_JAGUAR2 ON)
else()
    set(DEVOURER_JAGUAR2 OFF)
endif()
# Convenience: the shared Jaguar3 port compiles when either variant is on.
if(DEVOURER_JAGUAR3_8822C OR DEVOURER_JAGUAR3_8822E)
    set(DEVOURER_JAGUAR3 ON)
else()
    set(DEVOURER_JAGUAR3 OFF)
endif()
# Convenience: the shared Kestrel HAL compiles when either variant is on.
if(DEVOURER_KESTREL_8852B OR DEVOURER_KESTREL_8852C)
    set(DEVOURER_KESTREL ON)
else()
    set(DEVOURER_KESTREL OFF)
endif()
if(NOT DEVOURER_JAGUAR1 AND NOT DEVOURER_JAGUAR2 AND NOT DEVOURER_JAGUAR3
   AND NOT DEVOURER_KESTREL)
    message(FATAL_ERROR
        "No chip support selected. Enable at least one of DEVOURER_JAGUAR1 / "
        "DEVOURER_JAGUAR2_8822B / DEVOURER_JAGUAR2_8821C / DEVOURER_JAGUAR3_8822C "
        "/ DEVOURER_JAGUAR3_8822E / DEVOURER_KESTREL_8852B / DEVOURER_KESTREL_8852C.")
endif()
if(DEVOURER_PCIE)
    if(NOT CMAKE_SYSTEM_NAME STREQUAL "Linux")
        message(FATAL_ERROR "DEVOURER_PCIE=ON requires Linux (vfio-pci).")
    endif()
    if(NOT DEVOURER_JAGUAR2_8821C)
        message(FATAL_ERROR
            "DEVOURER_PCIE=ON requires DEVOURER_JAGUAR2_8821C=ON: the RTL8821CE "
            "(Jaguar2 8821C) is currently the only chip with a PCIe bring-up "
            "implemented and hardware-validated, so a PCIe build without it has "
            "no usable chip. Not an architectural limit for the HalMAC families "
            "— RTL8822BE (Jaguar2) / RTL8822CE (Jaguar3) could reuse the same "
            "buffer-descriptor ring transport with per-chip HAL gates. Jaguar1 "
            "PCIe parts (RTL8812AE/8821AE) are out of reach of this transport: "
            "pre-HalMAC silicon with rtlwifi-style own-bit descriptor DMA, a "
            "different engine entirely.")
    endif()
endif()

# ---------------------------------------------------------------------------
# Sanitizers. Off by default; the value applies to the library, the examples
# and the selftests alike, so `ctest` and an on-hardware demo run share one
# instrumented build. The vendored halbb/halrf C is exempted from UBSan below
# (unaligned loads / signed shifts are pervasive in Realtek's sources and are
# not ours to fix).
#   cmake -S . -B build-asan -DDEVOURER_SANITIZE=address+undefined
# ---------------------------------------------------------------------------
set(DEVOURER_SANITIZE "" CACHE STRING
    "Sanitizer build: address, address+undefined, thread (empty = off)")
set_property(CACHE DEVOURER_SANITIZE PROPERTY STRINGS
    "" address address+undefined thread)
if(NOT DEVOURER_SANITIZE STREQUAL "")
    if(MSVC)
        # MSVC ships AddressSanitizer only — no UBSan, no TSan, no LSan.
        # Degrade loudly rather than silently building something weaker than
        # the caller asked for.
        if(NOT DEVOURER_SANITIZE STREQUAL "address")
            message(FATAL_ERROR
                "DEVOURER_SANITIZE=${DEVOURER_SANITIZE} is unsupported on MSVC — "
                "it implements /fsanitize=address only. Use "
                "-DDEVOURER_SANITIZE=address, or build with GCC/Clang for "
                "undefined/thread.")
        endif()
        add_compile_options(/fsanitize=address)
        # ASan is incompatible with the runtime checks and with incremental
        # linking; both are on by default in Debug.
        add_compile_options($<$<CONFIG:Debug>:/Oy->)
        add_link_options(/INCREMENTAL:NO)
    else()
        if(DEVOURER_SANITIZE STREQUAL "address")
            set(_devourer_san "address")
        elseif(DEVOURER_SANITIZE STREQUAL "address+undefined")
            set(_devourer_san "address,undefined")
        elseif(DEVOURER_SANITIZE STREQUAL "thread")
            set(_devourer_san "thread")
        else()
            message(FATAL_ERROR
                "DEVOURER_SANITIZE=${DEVOURER_SANITIZE} — must be one of "
                "address, address+undefined, thread (or empty).")
        endif()
        add_compile_options(-fsanitize=${_devourer_san} -fno-omit-frame-pointer -g)
        add_link_options(-fsanitize=${_devourer_san})
    endif()
    message(STATUS "devourer: sanitizer build (${DEVOURER_SANITIZE})")
endif()

# Find pkg-config and then use it to locate libusb. The embedder can supply its
# own target instead (DEVOURER_LIBUSB_TARGET) when the system copy is unusable,
# e.g. a macOS universal build against single-arch Homebrew dylibs.
if(DEVOURER_LIBUSB_TARGET)
    set(DEVOURER_LIBUSB "${DEVOURER_LIBUSB_TARGET}")
else()
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(libusb REQUIRED IMPORTED_TARGET GLOBAL libusb-1.0)
    set(DEVOURER_LIBUSB PkgConfig::libusb)
endif()

add_library(devourer
    # --- generation-agnostic core (always compiled) ---
    src/logger.h
    src/Event.h
    hal/basic_types.h
    hal/hal_com_reg.h
    src/ieee80211_radiotap.h
    src/HalmacCfgParam.cpp
    src/HalmacCfgParam.h
    src/ParsedRadioPacket.cpp
    src/Radiotap.c
    src/RadiotapBuilder.cpp
    src/RadiotapBuilder.h
    src/RtlTransport.h
    src/RtlAdapter.cpp
    src/RtlAdapter.h
    src/RtlUsbAdapter.h # compat alias shim (RtlUsbAdapter = RtlAdapter)
    src/UsbTransport.cpp
    src/UsbTransport.h
    src/UsbDeviceLock.cpp
    src/UsbDeviceLock.h
    src/UsbOpen.cpp
    src/UsbOpen.h
    src/SelectedChannel.h
    src/RateDefinitions.h
    src/RxPacket.h
    src/TxDescBits.h
    src/BeamformingSounder.h
    src/TxMode.cpp
    src/TxMode.h
    src/TxPower.cpp
    src/TxPower.h
    src/TxStats.h
    src/ThermalStatus.h
    src/TxCaps.h
    src/LinkHealth.cpp
    src/LinkHealth.h
    src/RxQuality.h
    src/chanmig/ChannelScore.cpp
    src/chanmig/ChannelScore.h
    src/chanmig/MigGate.cpp
    src/chanmig/MigGate.h
    src/chanmig/ChannelDef.h
    src/chanmig/ActiveLink.h
    src/chanmig/EvidenceStore.h
    src/chanmig/ScanPlan.h
    src/chanmig/SurveyRecord.h
    src/chanmig/SurveyJsonl.h
    src/chanmig/JsonlLite.h
    src/chanmig/MigTypes.h
    src/chanmig/MigWire.h
    src/chanmig/MigConfig.h
    src/chanmig/MigClock.h
    src/chanmig/MigProposer.h
    src/chanmig/MigResponder.h
    src/hopset/HopsetState.h
    src/hopset/HopsetTypes.h
    src/hopset/HopsetWire.h
    src/hopset/HopsetAuthority.h
    src/hopset/HopsetFollower.h
    src/hopset/HopsetPolicy.h
    src/hopset/HopsetSense.h
    src/hopset/HopsetFusion.h
    src/hopset/HopsetEvents.h
    src/IRtlDevice.h
    src/SignalStop.cpp
    src/SignalStop.h
    src/WiFiDriver.cpp
    src/WiFiDriver.h
    src/registry_priv.h
    # LA-mode (phydm logic-analyzer) IQ capture — one implementation serves
    # all three generations (LaRegs carries the per-chip parameters).
    src/LaCapture.cpp
    src/LaCapture.h
    # phydm check_positive table walker — shared by Jaguar1 and Jaguar2
    # (both use the older addr/value + check_positive table format).
    src/PhyTableLoader.cpp
    src/PhyTableLoader.h
)

target_compile_features(devourer PUBLIC cxx_std_20)

# PUBLIC: logger.h is header-only, so the library and its consumers must agree
# on the compile-time log floor (ODR).
if(NOT DEVOURER_LOG_MAX_LEVEL STREQUAL "")
    target_compile_definitions(devourer PUBLIC
        DEVOURER_LOG_MAX_LEVEL=DEVOURER_LL_${DEVOURER_LOG_MAX_LEVEL})
endif()

# Link devourer with libusb as resolved above.
target_link_libraries(devourer PUBLIC ${DEVOURER_LIBUSB})

# Android (Termux / NDK): the diagnostics plane routes to logcat via
# __android_log_write (src/logger.h), which lives in the system `log` library.
if(ANDROID)
    target_link_libraries(devourer PUBLIC log)
endif()

target_include_directories(devourer PUBLIC hal)
target_include_directories(devourer PUBLIC hal/phydm/rtl8814a)
target_include_directories(devourer PUBLIC hal/phydm/rtl8822c)
target_include_directories(devourer PUBLIC hal/phydm/rtl8822e)
target_include_directories(devourer PUBLIC hal/phydm/rtl8822b)
target_include_directories(devourer PUBLIC hal/phydm/rtl8821c)
target_include_directories(devourer PUBLIC src)

# --- per-chip source groups (see options above) ---
if(DEVOURER_JAGUAR1)
    target_sources(devourer PRIVATE
        hal/hal8812a_fw.c hal/hal8812a_fw.h
        hal/hal8821a_fw.c hal/hal8821a_fw.h
        hal/Hal8812PwrSeq.c hal/Hal8812PwrSeq.h
        hal/Hal8821APwrSeq.c hal/Hal8821APwrSeq.h
        hal/Hal8812a_TxPwrTrack.cpp hal/Hal8812a_TxPwrTrack.h
        hal/Hal8812PhyReg.h hal/Hal8821PhyReg.h
        hal/rtl8812a_hal.h hal/rtl8812a_recv.h hal/rtl8812a_spec.h
        src/Firmware.h
        src/jaguar1/RtlJaguarDevice.cpp src/jaguar1/RtlJaguarDevice.h
        src/jaguar1/HalModule.cpp src/jaguar1/HalModule.h
        src/jaguar1/RadioManagementModule.cpp src/jaguar1/RadioManagementModule.h
        src/jaguar1/EepromManager.cpp src/jaguar1/EepromManager.h
        src/jaguar1/FirmwareManager.cpp src/jaguar1/FirmwareManager.h
        src/jaguar1/Iqk8812a.cpp src/jaguar1/Iqk8812a.h
        src/jaguar1/PhydmWatchdog.cpp src/jaguar1/PhydmWatchdog.h
        src/jaguar1/PowerTracking8812a.cpp src/jaguar1/PowerTracking8812a.h
        src/jaguar1/FrameParser.cpp src/jaguar1/FrameParser.h
        # NB: PhyTableLoader was promoted to src/ (shared with Jaguar2).
        src/jaguar1/BbDbgportReader.cpp src/jaguar1/BbDbgportReader.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR1=1)
endif()

if(DEVOURER_8814)
    target_sources(devourer PRIVATE
        hal/hal8814a_fw.c hal/hal8814a_fw.h
        hal/Hal8814PwrSeq.c hal/Hal8814PwrSeq.h
        hal/Hal8814PhyReg.h
        hal/phydm/rtl8814a/Hal8814_PhyTables.c
        hal/phydm/rtl8814a/Hal8814_PhyTables.h
        src/jaguar1/Iqk8814a.cpp src/jaguar1/Iqk8814a.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_8814=1)
endif()

# --- Jaguar2 (RTL8822BU / RTL8811CU-8821CU) ---
# Hybrid: HalMAC firmware/MAC-init/pwr-seq (like Jaguar3) + the older
# check_positive phydm table format (shared src/PhyTableLoader, like Jaguar1).
# Multi-chip via ChipVariant strategy dispatch (Jaguar2PhyTables / Jaguar2-
# Calibration), mirroring the Jaguar3 8822C/8822E split. The shared block below
# holds the orchestrator/HAL/halmac/strategy-trio + the per-variant dispatcher
# .cpp files (self-guarded on DEVOURER_HAVE_JAGUAR2_8822B/_8821C); each chip's
# firmware blob + generated tables live in its own variant block.
if(DEVOURER_JAGUAR2)
    target_sources(devourer PRIVATE
        src/jaguar2/RtlJaguar2Device.cpp src/jaguar2/RtlJaguar2Device.h
        src/jaguar2/HalJaguar2.cpp src/jaguar2/HalJaguar2.h
        src/jaguar2/HalmacJaguar2Fw.cpp src/jaguar2/HalmacJaguar2Fw.h
        src/jaguar2/HalmacJaguar2MacInit.cpp src/jaguar2/HalmacJaguar2MacInit.h
        src/jaguar2/HalmacJaguar2Regs.h
        src/jaguar2/FrameParserJaguar2.h
        src/jaguar2/ChipVariant.h
        src/jaguar2/Jaguar2PhyTables.h
        src/jaguar2/Jaguar2Calibration.h
        src/jaguar2/Phy8822bTables.cpp
        src/jaguar2/Halrf8822b.cpp src/jaguar2/Halrf8822b.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR2=1)
endif()

# 8822B variant data (2T2R): firmware blob + generated phydm tables + txpwr_lmt.
if(DEVOURER_JAGUAR2_8822B)
    target_sources(devourer PRIVATE
        hal/hal8822b_fw.c hal/hal8822b_fw.h
        hal/phydm/rtl8822b/Hal8822b_PhyTables.c hal/phydm/rtl8822b/Hal8822b_PhyTables.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR2_8822B=1)
endif()

# 8821C variant (1T1R): per-variant dispatcher impls (Phy8821cTables / Halrf8821c)
# + firmware blob + generated phydm tables. Mirrors the Jaguar3 8822E block.
if(DEVOURER_JAGUAR2_8821C)
    target_sources(devourer PRIVATE
        src/jaguar2/Phy8821cTables.cpp
        src/jaguar2/Halrf8821c.cpp src/jaguar2/Halrf8821c.h
        hal/hal8821c_fw.c hal/hal8821c_fw.h
        hal/phydm/rtl8821c/Hal8821c_PhyTables.c hal/phydm/rtl8821c/Hal8821c_PhyTables.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR2_8821C=1)
endif()

# --- PCIe transport (vfio-pci; see src/PcieTransport.h) ---
if(DEVOURER_PCIE)
    target_sources(devourer PRIVATE
        src/PcieTransport.cpp src/PcieTransport.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_PCIE=1)
endif()

if(DEVOURER_JAGUAR3)
    # Shared Jaguar3 port (compiled when either variant is on). Phy8822cTables.cpp
    # and Halrf8822c.cpp also carry the make_jaguar3_*() dispatchers, so they are
    # always built when Jaguar3 is on; their per-variant bodies are #if-guarded.
    target_sources(devourer PRIVATE
        src/jaguar3/Jaguar3PhyTables.h
        src/jaguar3/Jaguar3Calibration.h
        src/jaguar3/ChipVariant.h
        src/jaguar3/RtlJaguar3Device.cpp src/jaguar3/RtlJaguar3Device.h
        src/jaguar3/HalJaguar3.cpp src/jaguar3/HalJaguar3.h
        src/jaguar3/HalmacJaguar3Fw.cpp src/jaguar3/HalmacJaguar3Fw.h
        src/jaguar3/HalmacJaguar3MacInit.cpp src/jaguar3/HalmacJaguar3MacInit.h
        src/jaguar3/HalmacJaguar3Regs.h
        src/jaguar3/RadioManagementJaguar3.cpp src/jaguar3/RadioManagementJaguar3.h
        src/jaguar3/PhydmRuntimeJaguar3.cpp src/jaguar3/PhydmRuntimeJaguar3.h
        src/jaguar3/FrameParserJaguar3.h
        src/jaguar3/PhyTableLoaderJaguar3.cpp src/jaguar3/PhyTableLoaderJaguar3.h
        src/jaguar3/Phy8822cTables.cpp
        src/jaguar3/Halrf8822c.cpp src/jaguar3/Halrf8822c.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR3=1)
endif()

if(DEVOURER_JAGUAR3_8822C)
    target_sources(devourer PRIVATE
        hal/hal8822c_fw.c hal/hal8822c_fw.h
        hal/phydm/rtl8822c/Hal8822c_PhyTables.c hal/phydm/rtl8822c/Hal8822c_PhyTables.h
        hal/phydm/rtl8822c/Hal8822c_IqkNctl.c hal/phydm/rtl8822c/Hal8822c_IqkNctl.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR3_8822C=1)
endif()

if(DEVOURER_JAGUAR3_8822E)
    target_sources(devourer PRIVATE
        hal/hal8822e_fw.c hal/hal8822e_fw.h
        hal/phydm/rtl8822e/Hal8822e_PhyTables.c hal/phydm/rtl8822e/Hal8822e_PhyTables.h
        src/jaguar3/Phy8822eTables.cpp
        src/jaguar3/Halrf8822e.cpp src/jaguar3/Halrf8822e.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_JAGUAR3_8822E=1)
endif()

# --- Kestrel (Wi-Fi 6 / 802.11ax: RTL8852BU / RTL8852CU) ---
# Fourth generation, ported from the vendor "G6 phl" trees (reference/
# rtl8852bu, reference/rtl8852cu): mac_ax MAC/FW plane, halbb PHY tables,
# halrf calibration. Dispatched PID-first in WiFiDriver (the 0x00FC byte is
# R_AX_SYS_CHIPINFO on this silicon — see src/kestrel/ChipVariant.h). Shared
# core below; per-variant firmware blobs + tables live in their variant
# blocks.
if(DEVOURER_KESTREL)
    target_sources(devourer PRIVATE
        src/kestrel/ChipVariant.h
        src/kestrel/KestrelUsbIds.h
        src/kestrel/MacRegAx.h
        src/kestrel/HalKestrel.cpp src/kestrel/HalKestrel.h
        src/kestrel/KestrelFw.cpp src/kestrel/KestrelFw.h
        src/kestrel/KestrelFwSched.cpp
        src/kestrel/KestrelLe.h
        src/kestrel/RtlKestrelDevice.cpp src/kestrel/RtlKestrelDevice.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_KESTREL=1)
endif()

if(DEVOURER_KESTREL_8852B)
    target_sources(devourer PRIVATE
        # Generated by tools/extract_8852b_fw.py (two NICCE images, cut-selected
        # at runtime: CBV -> u2, CCV+ -> u3). Consumed by KestrelFw.
        hal/hal8852b_fw.c hal/hal8852b_fw.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_KESTREL_8852B=1)
endif()

if(DEVOURER_KESTREL_8852C)
    target_sources(devourer PRIVATE
        # Generated by tools/extract_8852c_fw.py (two NIC images, cut-selected
        # at runtime: CAV -> u1, CBV+ -> u2).
        hal/hal8852c_fw.c hal/hal8852c_fw.h)
    target_compile_definitions(devourer PUBLIC DEVOURER_HAVE_KESTREL_8852C=1)
endif()

# Vendored halbb/halrf G6 planes — Realtek's baseband + RF-calibration C
# compiled VERBATIM behind a thin shim (hal/halbb/hal_headers_le.h supplies a
# minimal PHL surface) + a C glue layer (kestrel_halbb_glue.h /
# kestrel_halrf_glue.h), instead of hand-transcribed C++. One chip-agnostic
# core (vendored from reference/rtl8852cu by tools/vendor_hal{bb,rf}_8852c.sh)
# carries the vendor's own per-chip runtime dispatch; the per-chip backends
# (halbb_8852b/halrf_8852b from reference/rtl8852bu, halbb_8852c/halrf_8852c
# from reference/rtl8852cu) join it per the DEVOURER_KESTREL_* options. All
# compilers: the vendor C is portable (audited: no GNU-only constructs); the
# dead-code stripping is -ffunction-sections/--gc-sections on GNU/Clang and
# /Gy + /OPT:REF on MSVC.
if(DEVOURER_KESTREL)
    if(MSVC)
        set(_kestrel_vendor_cflags /W0 /Gy)
    else()
        set(_kestrel_vendor_cflags -w -ffunction-sections -fdata-sections)
        # Realtek's sources are full of unaligned loads, signed shifts and
        # out-of-range enum stores. They are vendored verbatim (edit the
        # extractors, never these files), so UBSan on them reports noise we
        # would never act on and drowns the findings from our own code.
        if(DEVOURER_SANITIZE MATCHES "undefined")
            list(APPEND _kestrel_vendor_cflags -fno-sanitize=undefined)
        endif()
    endif()

    file(GLOB KESTREL_HALBB_C "${CMAKE_CURRENT_SOURCE_DIR}/hal/halbb/g6/vendor/*.c")
    file(GLOB KESTREL_HALRF_C "${CMAKE_CURRENT_SOURCE_DIR}/hal/halrf/g6/vendor/*.c")
    if(DEVOURER_KESTREL_8852B)
        file(GLOB _halbb_8852b "${CMAKE_CURRENT_SOURCE_DIR}/hal/halbb/g6/vendor/halbb_8852b/*.c")
        file(GLOB _halrf_8852b "${CMAKE_CURRENT_SOURCE_DIR}/hal/halrf/g6/vendor/halrf_8852b/*.c")
        list(APPEND KESTREL_HALBB_C ${_halbb_8852b})
        list(APPEND KESTREL_HALRF_C ${_halrf_8852b})
        list(APPEND _kestrel_halbb_defs CONFIG_RTL8852B=1 BB_8852B_SUPPORT=1)
        list(APPEND _kestrel_halrf_defs CONFIG_RTL8852B=1 RF_8852B_SUPPORT=1)
    endif()
    if(DEVOURER_KESTREL_8852C)
        file(GLOB _halbb_8852c "${CMAKE_CURRENT_SOURCE_DIR}/hal/halbb/g6/vendor/halbb_8852c/*.c")
        file(GLOB _halrf_8852c "${CMAKE_CURRENT_SOURCE_DIR}/hal/halrf/g6/vendor/halrf_8852c/*.c")
        list(APPEND KESTREL_HALBB_C ${_halbb_8852c})
        list(APPEND KESTREL_HALRF_C ${_halrf_8852c})
        list(APPEND _kestrel_halbb_defs CONFIG_RTL8852C=1 BB_8852C_SUPPORT=1)
        list(APPEND _kestrel_halrf_defs CONFIG_RTL8852C=1 RF_8852C_SUPPORT=1)
    endif()

    add_library(kestrel_halbb OBJECT
        ${KESTREL_HALBB_C}
        hal/halbb/g6/kestrel_halbb_glue.c
        hal/halbb/g6/kestrel_halbb_helpers.c)
    set_target_properties(kestrel_halbb PROPERTIES C_STANDARD 11
        POSITION_INDEPENDENT_CODE ON)
    target_include_directories(kestrel_halbb PRIVATE
        hal/halbb hal/halbb/g6
        hal/halbb/g6/vendor
        hal/halbb/g6/vendor/halbb_8852b hal/halbb/g6/vendor/halbb_8852c)
    # HALBB_DBCC_SUPPORT / HALBB_FW_OFLD_SUPPORT / HALBB_DBG_TRACE_SUPPORT are
    # #ifdef-tested in the vendor code, so being OFF means staying UNDEFINED —
    # a `=0` define still compiles the feature branches in (with fw-offload
    # compiled in, the 8852B BBCR writer routes every table write through the
    # fwofld wrapper and applies nothing).
    target_compile_definitions(kestrel_halbb PRIVATE
        ${_kestrel_halbb_defs}
        PLATFOM_IS_LITTLE_ENDIAN=1)
    target_compile_options(kestrel_halbb PRIVATE ${_kestrel_vendor_cflags})
    target_sources(devourer PRIVATE $<TARGET_OBJECTS:kestrel_halbb>)
    # C++ side includes only kestrel_halbb_glue.h (opaque handle + bridge).
    target_include_directories(devourer PRIVATE hal/halbb/g6)
    # Strip unreferenced vendored sections at link. ld64 (macOS) spells it
    # -dead_strip; MSVC's /OPT:REF is implied in Release via /Gy.
    if(APPLE)
        target_link_options(devourer INTERFACE -Wl,-dead_strip)
    elseif(NOT MSVC)
        target_link_options(devourer INTERFACE -Wl,--gc-sections)
    endif()

    # Vendored halrf-G6 RF calibrations (DACK / RX-DCK / IQK / LCK-RCK / efuse
    # trim + TSSI-DE load), compiled VERBATIM behind the SAME shim
    # (hal/halrf/hal_headers_le.h just reuses the halbb one + the bridge's
    # RF-register plane). Driven via kestrel_halrf_glue.h.
    add_library(kestrel_halrf OBJECT
        ${KESTREL_HALRF_C}
        hal/halrf/g6/kestrel_halrf_glue.c
        hal/halrf/g6/kestrel_halrf_helpers.c)
    set_target_properties(kestrel_halrf PROPERTIES C_STANDARD 11
        POSITION_INDEPENDENT_CODE ON)
    target_include_directories(kestrel_halrf PRIVATE
        hal/halrf hal/halbb hal/halbb/g6
        hal/halrf/g6/vendor
        hal/halrf/g6/vendor/halrf_8852b hal/halrf/g6/vendor/halrf_8852c
        hal/halbb/g6/vendor
        hal/halbb/g6/vendor/halbb_8852b hal/halbb/g6/vendor/halbb_8852c)
    target_compile_definitions(kestrel_halrf PRIVATE
        ${_kestrel_halrf_defs} PLATFOM_IS_LITTLE_ENDIAN=1
        KESTREL_HALRF_PRESENT=1)
    target_compile_options(kestrel_halrf PRIVATE ${_kestrel_vendor_cflags})
    target_sources(devourer PRIVATE $<TARGET_OBJECTS:kestrel_halrf>)
    target_include_directories(devourer PRIVATE hal/halrf/g6)
endif()


