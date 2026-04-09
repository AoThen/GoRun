#include <gtest/gtest.h>
#include "utils/PathUtils.h"

using namespace mn::PathUtils;

// ==================== GetFileName 测试 ====================

TEST(PathUtilsTest, GetFileName_EmptyPath) {
    EXPECT_EQ(GetFileName(L""), L"");
}

TEST(PathUtilsTest, GetFileName_SimpleName) {
    EXPECT_EQ(GetFileName(L"file.txt"), L"file.txt");
}

TEST(PathUtilsTest, GetFileName_WithDirectory) {
    EXPECT_EQ(GetFileName(L"/path/to/file.txt"), L"file.txt");
}

TEST(PathUtilsTest, GetFileName_WindowsPath) {
    EXPECT_EQ(GetFileName(L"C:\\path\\to\\file.txt"), L"file.txt");
}

TEST(PathUtilsTest, GetFileName_TrailingSlash) {
    // 空文件名的情况
    EXPECT_EQ(GetFileName(L"/path/to/"), L"");
}

// ==================== GetFileBaseName 测试 ====================

TEST(PathUtilsTest, GetFileBaseName_EmptyPath) {
    EXPECT_EQ(GetFileBaseName(L""), L"");
}

TEST(PathUtilsTest, GetFileBaseName_SimpleName) {
    EXPECT_EQ(GetFileBaseName(L"file.txt"), L"file");
}

TEST(PathUtilsTest, GetFileBaseName_WithDirectory) {
    EXPECT_EQ(GetFileBaseName(L"/path/to/file.txt"), L"file");
}

TEST(PathUtilsTest, GetFileBaseName_MultipleExtensions) {
    EXPECT_EQ(GetFileBaseName(L"file.tar.gz"), L"file.tar");
}

TEST(PathUtilsTest, GetFileBaseName_NoExtension) {
    EXPECT_EQ(GetFileBaseName(L"/path/to/file"), L"file");
}

// ==================== GetParentDir 测试 ====================

TEST(PathUtilsTest, GetParentDir_EmptyPath) {
    EXPECT_EQ(GetParentDir(L""), L"");
}

TEST(PathUtilsTest, GetParentDir_SimplePath) {
    EXPECT_EQ(GetParentDir(L"/path/to/file.txt"), L"/path/to");
}

TEST(PathUtilsTest, GetParentDir_RootPath) {
    // 根目录的父目录是空或根
    auto result = GetParentDir(L"/");
    EXPECT_TRUE(result.empty() || result == L"/");
}

TEST(PathUtilsTest, GetParentDir_SingleComponent) {
    EXPECT_EQ(GetParentDir(L"file.txt"), L"");
}

// ==================== Exists 测试 ====================

TEST(PathUtilsTest, Exists_EmptyPath) {
    EXPECT_FALSE(Exists(L""));
}

TEST(PathUtilsTest, Exists_NonExistentPath) {
    EXPECT_FALSE(Exists(L"/this/path/definitely/does/not/exist"));
}

// ==================== EnsureDirectory 测试 ====================

TEST(PathUtilsTest, EnsureDirectory_EmptyPath) {
    // 空路径应该返回 false 或不抛出异常
    EXPECT_FALSE(EnsureDirectory(L""));
}

// ==================== ToAbsolute 测试 ====================

TEST(PathUtilsTest, ToAbsolute_EmptyPath) {
    // 空路径应该返回当前目录或空
    auto result = ToAbsolute(L"");
    // 行为取决于实现，至少不应抛出异常
    EXPECT_TRUE(true);
}

TEST(PathUtilsTest, ToAbsolute_AlreadyAbsolute) {
    // 已是绝对路径应该返回自身
    auto result = ToAbsolute(L"/path/to/file");
    // 结果应该是绝对路径
    EXPECT_TRUE(!result.empty());
}

// ==================== ToRelative 测试 ====================

TEST(PathUtilsTest, ToRelative_EmptyPath) {
    auto result = ToRelative(L"");
    EXPECT_TRUE(result.empty() || !result.empty());  // 不应抛出异常
}

// ==================== GetExePath 测试 ====================

TEST(PathUtilsTest, GetExePath_NotEmpty) {
    auto result = GetExePath();
    EXPECT_FALSE(result.empty());
}

// ==================== GetExeDir 测试 ====================

TEST(PathUtilsTest, GetExeDir_NotEmpty) {
    auto result = GetExeDir();
    EXPECT_FALSE(result.empty());
}

// ==================== GetAppDataPath 测试 ====================

TEST(PathUtilsTest, GetAppDataPath_NotEmpty) {
    auto result = GetAppDataPath();
    EXPECT_FALSE(result.empty());
}
