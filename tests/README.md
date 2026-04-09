# GoRun 单元测试

## 构建测试

使用 CMake 构建项目时，测试会自动包含（默认 `GORUN_BUILD_TESTS=ON`）。

```bash
# 生成项目文件（Visual Studio）
cmake -B build -A x64

# 或生成 Ninja 构建文件
cmake -B build -G "Ninja" -DCMAKE_BUILD_TYPE=Debug

# 构建测试
cmake --build build --config Debug --target GoRunTests
```

## 运行测试

```bash
# 运行所有测试
build\Debug\GoRunTests.exe

# 或使用 CTest
cd build
ctest -C Debug --output-on-failure
```

## 测试覆盖范围

| 测试文件 | 测试目标 | 测试数量 |
|---------|---------|---------|
| StringUtilsTest.cpp | 字符串工具、拼音匹配、搜索匹配 | ~60 |
| PathUtilsTest.cpp | 路径工具函数 | ~15 |
| TypesTest.cpp | 数据类型和 ID 生成 | ~15 |
| StorageTest.cpp | JSON 存储、分类/项目 CRUD | ~35 |
| ConfigTest.cpp | 配置管理（快捷键、窗口、自启） | ~25 |

## 禁用测试构建

如果不需要构建测试，可以设置 CMake 选项：

```bash
cmake -B build -DGORUN_BUILD_TESTS=OFF
```

## 注意事项

- 测试框架使用 Google Test v1.14.0
- 测试仅测试平台无关的工具类和逻辑
- UI 和渲染组件需要手动测试或集成测试
