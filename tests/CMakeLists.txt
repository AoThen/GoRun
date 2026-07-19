# GoRun 单元测试配置

# 查找所有测试源文件
file(GLOB_RECURSE TEST_SOURCES "${CMAKE_CURRENT_SOURCE_DIR}/*.cpp")

# 创建测试可执行文件
add_executable(GoRunTests ${TEST_SOURCES})

# 链接 Google Test
target_link_libraries(GoRunTests
    PRIVATE
        GTest::gtest
        GTest::gtest_main
        nlohmann_json
)

# 包含目录
target_include_directories(GoRunTests PRIVATE
    "${CMAKE_SOURCE_DIR}/src"
)

# 需要的源文件（工具类和核心业务逻辑）
set(TEST_UTIL_SOURCES
    "${CMAKE_SOURCE_DIR}/src/utils/StringUtils.cpp"
    "${CMAKE_SOURCE_DIR}/src/utils/PathUtils.cpp"
    "${CMAKE_SOURCE_DIR}/src/utils/Logger.cpp"
    "${CMAKE_SOURCE_DIR}/src/core/Storage.cpp"
    "${CMAKE_SOURCE_DIR}/src/core/Config.cpp"
)

target_sources(GoRunTests PRIVATE ${TEST_UTIL_SOURCES})

# Windows 特定配置
if(WIN32)
    target_link_libraries(GoRunTests PRIVATE shell32 shlwapi ole32 oleaut32)
    target_compile_definitions(GoRunTests PRIVATE _WIN32)
    
    # 使用静态运行时库
    if(MSVC)
        set_property(TARGET GoRunTests PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
        target_compile_options(GoRunTests PRIVATE /utf-8 /W3)
    endif()
endif()

# C++17 标准
target_compile_features(GoRunTests PRIVATE cxx_std_17)

# 注册测试
include(GoogleTest)
gtest_discover_tests(GoRunTests)
