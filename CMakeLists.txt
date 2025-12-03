cmake_minimum_required(VERSION 3.15)
project(manganese C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS ON)

# Build options
option(BUILD_WITH_OPENBLAS "Build with OpenBLAS support" ON)
option(BUILD_OPENBLAS "Build OpenBLAS from submodule (if not already built)" ON)
option(BUILD_V3_TARGET "Build x86-64-v3 (AVX2) compatible binary" ON)
option(BUILD_V4_TARGET "Build x86-64-v4 (AVX512) optimized binary" ON)

# Platform detection
if(WIN32 OR CMAKE_SYSTEM_NAME STREQUAL "Windows")
    set(PLATFORM_WINDOWS ON)
    add_definitions(-DPLATFORM_WINDOWS)
else()
    set(PLATFORM_UNIX ON)
    add_definitions(-DPLATFORM_UNIX)
endif()

# Base compiler flags
set(CMAKE_C_FLAGS_RELEASE "-O3 -flto")
set(CMAKE_C_FLAGS_BASE "-fopenmp" "-std=gnu2x")

# AVX512 flags
set(AVX512_FLAGS "-mavx512f" "-mavx512bw" "-mrdrnd")
set(AVX2_FLAGS "-mavx2" "-mrdrnd")

# Include directories
include_directories(
    ${CMAKE_SOURCE_DIR}/SIMDxorshift/include
)

if(PLATFORM_WINDOWS)
    include_directories(${CMAKE_SOURCE_DIR}/mman-win32)
endif()

# Build OpenBLAS if requested and not already built
# This matches the Makefile's OpenBLAS build configuration
if(BUILD_WITH_OPENBLAS AND BUILD_OPENBLAS AND NOT EXISTS "${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a")
    find_program(MAKE_PROGRAM make)
    find_program(NPROC_PROGRAM nproc)
    if(MAKE_PROGRAM)
        # Determine number of parallel jobs
        if(NPROC_PROGRAM)
            execute_process(COMMAND ${NPROC_PROGRAM} OUTPUT_VARIABLE NPROC_OUTPUT OUTPUT_STRIP_TRAILING_WHITESPACE)
            set(OPENBLAS_JOBS "-j${NPROC_OUTPUT}")
        else()
            set(OPENBLAS_JOBS "")
        endif()
        
        # Build OpenBLAS with the same flags as the Makefile
        # USE_OPENMP=1 ensures OpenBLAS is built with OpenMP support
        add_custom_target(build_openblas
            COMMAND ${MAKE_PROGRAM} -C ${CMAKE_SOURCE_DIR}/OpenBLAS 
                USE_OPENMP=1 USE_THREAD=1 NO_LAPACK=1 DYNAMIC_ARCH=1 ONLY_CBLAS=1 NO_SHARED=1 USE_TLS=1 
                ${OPENBLAS_JOBS}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Building OpenBLAS with OpenMP support (this may take a while)..."
            BYPRODUCTS ${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a
        )
        message(STATUS "OpenBLAS build target added. It will be built automatically as a dependency.")
    else()
        message(WARNING "make not found, cannot build OpenBLAS. Please build it manually or install it.")
    endif()
endif()

if(BUILD_WITH_OPENBLAS AND EXISTS "${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a")
    add_definitions(-DHAVE_OPENBLAS)
    include_directories(${CMAKE_SOURCE_DIR}/OpenBLAS)
    # Make OpenBLAS a dependency if we're building it
    if(BUILD_OPENBLAS AND TARGET build_openblas)
        # Note: We can't directly add it as a dependency here because targets are created later
        # The user needs to build OpenBLAS first, or we handle it in the target creation
    endif()
endif()

# Link libraries
# OpenMP - handle cross-compilation
if(CMAKE_CROSSCOMPILING AND CMAKE_SYSTEM_NAME STREQUAL "Windows")
    # For MinGW cross-compilation, OpenMP is typically provided by libgomp
    set(OpenMP_C_FOUND TRUE)
    set(OpenMP_C_FLAGS "-fopenmp")
    set(OpenMP_C_LIB_NAMES "gomp")
    find_library(OpenMP_C_LIBRARY
        NAMES ${OpenMP_C_LIB_NAMES}
        PATHS ${CMAKE_FIND_ROOT_PATH}/lib
        NO_DEFAULT_PATH
    )
    if(OpenMP_C_LIBRARY)
        add_library(OpenMP::OpenMP_C INTERFACE IMPORTED)
        set_target_properties(OpenMP::OpenMP_C PROPERTIES
            INTERFACE_COMPILE_OPTIONS "${OpenMP_C_FLAGS}"
            INTERFACE_LINK_LIBRARIES "${OpenMP_C_LIBRARY}"
        )
    else()
        # Fallback: just add -fopenmp flag
        add_library(OpenMP::OpenMP_C INTERFACE IMPORTED)
        set_target_properties(OpenMP::OpenMP_C PROPERTIES
            INTERFACE_COMPILE_OPTIONS "-fopenmp"
            INTERFACE_LINK_LIBRARIES "-fopenmp"
        )
    endif()
else()
    find_package(OpenMP REQUIRED)
endif()

# Platform-specific libraries
if(NOT PLATFORM_WINDOWS)
    find_library(M_LIB m)
    find_library(PTHREAD_LIB pthread)
endif()

# Source files
set(MANGANESE_SOURCES
    manganese.c
    tests.c
    hardware.c
)

# Platform-specific sources
if(PLATFORM_WINDOWS)
    list(APPEND MANGANESE_SOURCES mman-win32/mman.c)
endif()

# Object files with specific flags
set(TESTS_512_SOURCE tests-512.c)
set(TESTS_256_SOURCE tests-256.c)

# Function to create a build variant
function(create_build_variant variant_name arch_flag masm_flag)
    # SIMDxorshift library - always build with AVX512 (runtime detection will handle it)
    # The Makefile calls: $(MAKE) CFLAGS=-mavx512f -C SIMDxorshift simdxorshift128plus.o
    # This overrides SIMDxorshift's Makefile CFLAGS, so we match that behavior
    # However, we still want the parent Makefile's optimization flags
    add_library(simdxorshift128plus_obj_${variant_name} OBJECT 
        SIMDxorshift/src/simdxorshift128plus.c
    )
    target_compile_options(simdxorshift128plus_obj_${variant_name} PRIVATE 
        $<$<CONFIG:Release>:${CMAKE_C_FLAGS_RELEASE}>
        ${CMAKE_C_FLAGS_BASE}
        "-mavx512f"
        ${arch_flag}
    )
    if(masm_flag)
        target_compile_options(simdxorshift128plus_obj_${variant_name} PRIVATE ${masm_flag})
    endif()
    target_include_directories(simdxorshift128plus_obj_${variant_name} PRIVATE ${CMAKE_SOURCE_DIR}/SIMDxorshift/include)

    # Build tests-512.o with AVX512
    add_library(tests_512_obj_${variant_name} OBJECT ${TESTS_512_SOURCE})
    target_compile_options(tests_512_obj_${variant_name} PRIVATE ${AVX512_FLAGS})
    target_include_directories(tests_512_obj_${variant_name} PRIVATE ${CMAKE_SOURCE_DIR}/SIMDxorshift/include)
    if(BUILD_WITH_OPENBLAS AND EXISTS "${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a")
        target_include_directories(tests_512_obj_${variant_name} PRIVATE ${CMAKE_SOURCE_DIR}/OpenBLAS)
        target_compile_definitions(tests_512_obj_${variant_name} PRIVATE HAVE_OPENBLAS)
    endif()

    # Build tests-256.o with AVX2
    add_library(tests_256_obj_${variant_name} OBJECT ${TESTS_256_SOURCE})
    target_compile_options(tests_256_obj_${variant_name} PRIVATE ${AVX2_FLAGS})
    target_include_directories(tests_256_obj_${variant_name} PRIVATE ${CMAKE_SOURCE_DIR}/SIMDxorshift/include)
    if(BUILD_WITH_OPENBLAS AND EXISTS "${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a")
        target_include_directories(tests_256_obj_${variant_name} PRIVATE ${CMAKE_SOURCE_DIR}/OpenBLAS)
        target_compile_definitions(tests_256_obj_${variant_name} PRIVATE HAVE_OPENBLAS)
    endif()

    # Main executable
    add_executable(manganese${variant_name} ${MANGANESE_SOURCES}
        $<TARGET_OBJECTS:tests_512_obj_${variant_name}>
        $<TARGET_OBJECTS:tests_256_obj_${variant_name}>
        $<TARGET_OBJECTS:simdxorshift128plus_obj_${variant_name}>
    )

    # Set architecture-specific flags for main sources
    target_compile_options(manganese${variant_name} PRIVATE 
        ${CMAKE_C_FLAGS_BASE}
        ${arch_flag}
    )
    if(masm_flag)
        target_compile_options(manganese${variant_name} PRIVATE ${masm_flag})
    endif()

    target_link_libraries(manganese${variant_name} 
        OpenMP::OpenMP_C
    )

    if(NOT PLATFORM_WINDOWS)
        if(M_LIB)
            target_link_libraries(manganese${variant_name} ${M_LIB})
        endif()
        if(PTHREAD_LIB)
            target_link_libraries(manganese${variant_name} ${PTHREAD_LIB})
        endif()
    endif()

    if(BUILD_WITH_OPENBLAS)
        # Add OpenBLAS as a dependency if we're building it
        if(BUILD_OPENBLAS AND TARGET build_openblas)
            add_dependencies(manganese${variant_name} build_openblas)
        endif()
        # Link OpenBLAS if it exists (or will be built)
        if(EXISTS "${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a" OR (BUILD_OPENBLAS AND TARGET build_openblas))
            target_link_libraries(manganese${variant_name} ${CMAKE_SOURCE_DIR}/OpenBLAS/libopenblas.a)
        endif()
    endif()

    # Windows-specific settings
    if(PLATFORM_WINDOWS)
        set_target_properties(manganese${variant_name} PROPERTIES
            OUTPUT_NAME "manganese${variant_name}"
            SUFFIX ".exe"
        )
        target_link_options(manganese${variant_name} PRIVATE -static)
    endif()

    # Set output directory
    set_target_properties(manganese${variant_name} PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}
    )
endfunction()

# Build x86-64-v3 variant (AVX2 baseline, works on AVX2 CPUs, uses AVX512 if available)
if(BUILD_V3_TARGET)
    if(NOT CMAKE_CROSSCOMPILING AND NOT PLATFORM_WINDOWS)
        create_build_variant("-avx2" "-march=x86-64-v3" "-masm=intel")
    elseif(CMAKE_CROSSCOMPILING AND CMAKE_SYSTEM_NAME STREQUAL "Windows")
        create_build_variant("-avx2" "-march=x86-64-v3" "")
    else()
        create_build_variant("-avx2" "-march=x86-64-v3" "")
    endif()
endif()

# Build x86-64-v4 variant (AVX512 optimized, requires AVX512 CPU)
if(BUILD_V4_TARGET)
    if(NOT CMAKE_CROSSCOMPILING AND NOT PLATFORM_WINDOWS)
        create_build_variant("-avx512" "-march=x86-64-v4" "-masm=intel")
    elseif(CMAKE_CROSSCOMPILING AND CMAKE_SYSTEM_NAME STREQUAL "Windows")
        create_build_variant("-avx512" "-march=x86-64-v4" "")
    else()
        create_build_variant("-avx512" "-march=x86-64-v4" "")
    endif()
endif()

# Default target (for backward compatibility, use v3 if available, otherwise v4)
if(BUILD_V3_TARGET)
    add_custom_target(manganese ALL DEPENDS manganese-avx2)
elseif(BUILD_V4_TARGET)
    add_custom_target(manganese ALL DEPENDS manganese-avx512)
endif()