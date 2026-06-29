cmake_minimum_required(VERSION 3.26)
project(sparrow LANGUAGES C)

# ── Find Rust / Cargo ──────────────────────────────────────────────
# Search PATH first, then the standard rustup install location, so a freshly
# installed toolchain is found even when PATH hasn't been refreshed yet
# (e.g. Visual Studio / the shell was opened before Rust was installed).
find_program(CARGO_EXECUTABLE cargo
  PATHS
    "$ENV{CARGO_HOME}/bin"
    "$ENV{USERPROFILE}/.cargo/bin"
    "$ENV{HOME}/.cargo/bin"
)
if(NOT CARGO_EXECUTABLE)
  message(FATAL_ERROR
    "cargo not found.\n"
    "  1) Install Rust:  https://rustup.rs   (or:  winget install Rustlang.Rustup)\n"
    "     then:  rustup toolchain install nightly\n"
    "  2) Make sure cargo is on PATH (rustup adds %USERPROFILE%\\.cargo\\bin).\n"
    "  3) RESTART the shell / fully restart Visual Studio so the new PATH is seen.\n"
    "  4) DELETE the CMake cache and reconfigure (VS: 'Delete Cache and Reconfigure',\n"
    "     or remove the build/out directory) — CMake caches this not-found result.")
endif()
message(STATUS "Found cargo: ${CARGO_EXECUTABLE}")

set(CARGO_MANIFEST_DIR "${CMAKE_SOURCE_DIR}")

# ── Helper: detect target triple ───────────────────────────────────
if(WIN32)
  if(CMAKE_SIZEOF_VOID_P EQUAL 8)
    set(RUST_TARGET_TRIPLE "x86_64-pc-windows-msvc")
  else()
    set(RUST_TARGET_TRIPLE "i686-pc-windows-msvc")
  endif()
elseif(APPLE)
  if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64|aarch64")
    set(RUST_TARGET_TRIPLE "aarch64-apple-darwin")
  else()
    set(RUST_TARGET_TRIPLE "x86_64-apple-darwin")
  endif()
else()
  if(CMAKE_SIZEOF_VOID_P EQUAL 8)
    set(RUST_TARGET_TRIPLE "x86_64-unknown-linux-gnu")
  else()
    set(RUST_TARGET_TRIPLE "i686-unknown-linux-gnu")
  endif()
endif()

message(STATUS "Rust target triple: ${RUST_TARGET_TRIPLE}")

# ── Build type mapping (works for multi-config generators too) ──────
# Visual Studio is a MULTI-CONFIG generator: CMAKE_BUILD_TYPE is empty at
# configure time and the real config (Debug/Release) is only known at BUILD
# time. Use a generator expression so cargo's profile, its output directory,
# and the copy step below all agree — on single-config (Make/Ninja) AND
# multi-config (Visual Studio) generators.  `--profile dev` == the default
# debug build and still emits into target/debug/.
set(CARGO_BUILD_DIR "$<IF:$<CONFIG:Debug>,debug,release>")
set(CARGO_PROFILE_FLAG "--profile" "$<IF:$<CONFIG:Debug>,dev,release>")

# ── Output paths ───────────────────────────────────────────────────
set(CARGO_OUT_DIR "${CARGO_MANIFEST_DIR}/target/${CARGO_BUILD_DIR}")

# Cargo output file names are platform-specific:
if(WIN32)
  set(CARGO_DLL_NAME       "sparrow.dll")
  set(CARGO_STATICLIB_NAME "sparrow.lib")
  set(CARGO_IMPORTLIB_NAME "sparrow.dll.lib")

  set(DLL_FILE       "${CARGO_OUT_DIR}/SparrowLib.dll")
  set(LIB_FILE       "${CARGO_OUT_DIR}/SparrowLib.dll.lib")
  set(STATIC_LIB_FILE "${CARGO_OUT_DIR}/SparrowLib.lib")
elseif(APPLE)
  set(CARGO_DLL_NAME       "libsparrow.dylib")
  set(CARGO_STATICLIB_NAME "libsparrow.a")

  set(DLL_FILE       "${CARGO_OUT_DIR}/libSparrowLib.dylib")
  set(STATIC_LIB_FILE "${CARGO_OUT_DIR}/libSparrowLib.a")
else()
  set(CARGO_DLL_NAME       "libsparrow.so")
  set(CARGO_STATICLIB_NAME "libsparrow.a")

  set(DLL_FILE       "${CARGO_OUT_DIR}/libSparrowLib.so")
  set(STATIC_LIB_FILE "${CARGO_OUT_DIR}/libSparrowLib.a")
endif()

# ── Collect Rust source files for IDE visibility ───────────────────
file(GLOB_RECURSE RUST_SOURCES "${CARGO_MANIFEST_DIR}/src/*.rs")
file(GLOB_RECURSE RUST_TESTS "${CARGO_MANIFEST_DIR}/tests/*.rs")
file(GLOB_RECURSE WASM_SOURCES "${CARGO_MANIFEST_DIR}/sparrow-wasm/src/*.rs")
list(APPEND RUST_PROJECT_FILES
  "${CARGO_MANIFEST_DIR}/Cargo.toml"
  "${CARGO_MANIFEST_DIR}/Cargo.lock"
  "${CARGO_MANIFEST_DIR}/sparrow-wasm/Cargo.toml"
  ${RUST_SOURCES}
  ${RUST_TESTS}
  ${WASM_SOURCES}
)

# Make Rust sources visible in Visual Studio's Solution Explorer
if(MSVC OR (CMAKE_GENERATOR STREQUAL "Visual Studio 17 2022" OR CMAKE_GENERATOR MATCHES "Visual Studio"))
  # Create a source group tree starting from the src dir, so VS shows nested folders
  source_group(TREE "${CARGO_MANIFEST_DIR}/src" PREFIX "Rust\\src" FILES ${RUST_SOURCES})
  source_group(TREE "${CARGO_MANIFEST_DIR}/tests" PREFIX "Rust\\tests" FILES ${RUST_TESTS})
  source_group("Rust" FILES "${CARGO_MANIFEST_DIR}/Cargo.toml" "${CARGO_MANIFEST_DIR}/Cargo.lock")
endif()

# ── Options ────────────────────────────────────────────────────────
option(BUILD_WITH_JNI "Build sparrow with JNI support (requires JDK)" ON)

# ── Cargo build (cdylib + staticlib) ───────────────────────────────
# Config-aware via generator expression, so the cargo output dir always
# matches the copy step (fixes the Debug/Release mismatch on Visual Studio).
set(CARGO_TARGET_ARGS build ${CARGO_PROFILE_FLAG})
if(BUILD_WITH_JNI)
  list(APPEND CARGO_TARGET_ARGS --features jni)
endif()

# Build the list of post-build copy commands (platform-specific)
set(COPY_COMMANDS
  COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CARGO_OUT_DIR}/${CARGO_DLL_NAME}" "${DLL_FILE}"
  COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CARGO_OUT_DIR}/${CARGO_STATICLIB_NAME}" "${STATIC_LIB_FILE}"
)
# Windows import library (.dll.lib) — only exists on Windows
if(WIN32)
  list(APPEND COPY_COMMANDS
    COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CARGO_OUT_DIR}/${CARGO_IMPORTLIB_NAME}" "${LIB_FILE}"
  )
endif()

# BYPRODUCTS must include all expected outputs
set(BYPRODUCTS_LIST "${DLL_FILE}" "${STATIC_LIB_FILE}")
if(WIN32)
  list(APPEND BYPRODUCTS_LIST "${LIB_FILE}")
endif()

add_custom_target(SparrowLib ALL
  COMMAND "${CARGO_EXECUTABLE}" ${CARGO_TARGET_ARGS}
  ${COPY_COMMANDS}
  WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
  COMMENT "Building sparrow Rust library and renaming to SparrowLib"
  BYPRODUCTS ${BYPRODUCTS_LIST}
  SOURCES ${RUST_PROJECT_FILES}
)

# ── Convenience targets ────────────────────────────────────────────
add_custom_target(cargo-build-debug
  COMMAND "${CARGO_EXECUTABLE}" build
  WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
  COMMENT "Build Rust debug binaries with cargo"
)

add_custom_target(cargo-build-release
  COMMAND "${CARGO_EXECUTABLE}" build --release
  WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
  COMMENT "Build Rust release binaries with cargo"
)

add_custom_target(cargo-test
  COMMAND "${CARGO_EXECUTABLE}" test
  WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
  COMMENT "Run Rust tests with cargo"
)

add_custom_target(cargo-run
  COMMAND "${CARGO_EXECUTABLE}" run --bin sparrow-cli
  WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
  COMMENT "Run sparrow binary with cargo"
)

add_custom_target(cargo-bench
  COMMAND "${CARGO_EXECUTABLE}" run --bin bench
  WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
  COMMENT "Run bench binary with cargo"
)

# ── WASM build (wasm-pack) ─────────────────────────────────────────
find_program(WASM_PACK_EXECUTABLE wasm-pack
  PATHS
    "$ENV{CARGO_HOME}/bin"
    "$ENV{USERPROFILE}/.cargo/bin"
    "$ENV{HOME}/.cargo/bin"
)
if(NOT WASM_PACK_EXECUTABLE)
  message(WARNING
    "wasm-pack not found. Install with:  cargo install wasm-pack\n"
    "The 'wasm-build' target will not be available."
  )
else()
  message(STATUS "Found wasm-pack: ${WASM_PACK_EXECUTABLE}")

  add_custom_target(wasm-build
    COMMAND "${WASM_PACK_EXECUTABLE}" build sparrow-wasm --target web
    WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
    COMMENT "Build sparrow-wasm for browser/web"
  )

  add_custom_target(wasm-build-bundler
    COMMAND "${WASM_PACK_EXECUTABLE}" build sparrow-wasm --target bundler
    WORKING_DIRECTORY "${CARGO_MANIFEST_DIR}"
    COMMENT "Build sparrow-wasm for Node.js / bundler"
  )
endif()

# ── Install rules ──────────────────────────────────────────────────
include(GNUInstallDirs)

install(FILES "${DLL_FILE}"
  DESTINATION "${CMAKE_INSTALL_BINDIR}"
  COMPONENT runtime
)

install(FILES "${STATIC_LIB_FILE}"
  DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  COMPONENT development
)

if(WIN32)
  install(FILES "${LIB_FILE}"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    COMPONENT development
  )
endif()

# ── Header copy ────────────────────────────────────────────────────
install(FILES "${CMAKE_SOURCE_DIR}/include/sparrow.h"
  DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  COMPONENT development
)
