#include <gtest/gtest.h>
#include "core/Types.h"

using namespace mn;

// ==================== Category 默认值测试 ====================

TEST(TypesTest, Category_DefaultValues) {
    Category cat;
    EXPECT_TRUE(cat.id.empty());
    EXPECT_TRUE(cat.name.empty());
    EXPECT_EQ(cat.sortOrder, 0);
    EXPECT_EQ(cat.viewType, ViewType::Icon);
    EXPECT_EQ(cat.iconSize, 48);
}

TEST(TypesTest, Category_Assignment) {
    Category cat;
    cat.id = L"cat_123";
    cat.name = L"测试分类";
    cat.sortOrder = 5;
    cat.viewType = ViewType::List;
    cat.iconSize = 64;
    
    EXPECT_EQ(cat.id, L"cat_123");
    EXPECT_EQ(cat.name, L"测试分类");
    EXPECT_EQ(cat.sortOrder, 5);
    EXPECT_EQ(cat.viewType, ViewType::List);
    EXPECT_EQ(cat.iconSize, 64);
}

// ==================== Item 默认值测试 ====================

TEST(TypesTest, Item_DefaultValues) {
    Item item;
    EXPECT_TRUE(item.id.empty());
    EXPECT_TRUE(item.name.empty());
    EXPECT_TRUE(item.target.empty());
    EXPECT_TRUE(item.arguments.empty());
    EXPECT_TRUE(item.workingDir.empty());
    EXPECT_TRUE(item.iconPath.empty());
    EXPECT_EQ(item.iconIndex, 0);
    EXPECT_FALSE(item.runAsAdmin);
    EXPECT_EQ(item.runCount, 0);
    EXPECT_TRUE(item.keywords.empty());
    EXPECT_TRUE(item.remark.empty());
    EXPECT_TRUE(item.categoryId.empty());
    EXPECT_EQ(item.sortOrder, 0);
}

TEST(TypesTest, Item_Assignment) {
    Item item;
    item.id = L"item_123";
    item.name = L"测试项目";
    item.target = L"C:\\test.exe";
    item.arguments = L"--flag";
    item.workingDir = L"C:\\";
    item.iconPath = L"C:\\test.ico";
    item.iconIndex = 2;
    item.runAsAdmin = true;
    item.runCount = 10;
    item.keywords = L"key1,key2";
    item.remark = L"备注";
    item.categoryId = L"cat_123";
    item.sortOrder = 3;
    
    EXPECT_EQ(item.id, L"item_123");
    EXPECT_EQ(item.name, L"测试项目");
    EXPECT_EQ(item.target, L"C:\\test.exe");
    EXPECT_EQ(item.arguments, L"--flag");
    EXPECT_EQ(item.workingDir, L"C:\\");
    EXPECT_EQ(item.iconPath, L"C:\\test.ico");
    EXPECT_EQ(item.iconIndex, 2);
    EXPECT_TRUE(item.runAsAdmin);
    EXPECT_EQ(item.runCount, 10);
    EXPECT_EQ(item.keywords, L"key1,key2");
    EXPECT_EQ(item.remark, L"备注");
    EXPECT_EQ(item.categoryId, L"cat_123");
    EXPECT_EQ(item.sortOrder, 3);
}

// ==================== RunResult 默认值测试 ====================

TEST(TypesTest, RunResult_DefaultValues) {
    RunResult result;
    EXPECT_FALSE(result.success);
    EXPECT_EQ(result.error, RunError::None);
    EXPECT_TRUE(result.errorMessage.empty());
}

TEST(TypesTest, RunResult_Assignment) {
    RunResult result;
    result.success = false;
    result.error = RunError::FileNotFound;
    result.errorMessage = L"文件不存在";
    
    EXPECT_FALSE(result.success);
    EXPECT_EQ(result.error, RunError::FileNotFound);
    EXPECT_EQ(result.errorMessage, L"文件不存在");
}

// ==================== ViewType 枚举测试 ====================

TEST(TypesTest, ViewType_Values) {
    EXPECT_EQ(static_cast<int>(ViewType::Icon), 0);
    EXPECT_EQ(static_cast<int>(ViewType::List), 1);
}

// ==================== RunError 枚举测试 ====================

TEST(TypesTest, RunError_Values) {
    EXPECT_EQ(static_cast<int>(RunError::None), 0);
    EXPECT_EQ(static_cast<int>(RunError::FileNotFound), 1);
    EXPECT_EQ(static_cast<int>(RunError::PathNotFound), 2);
    EXPECT_EQ(static_cast<int>(RunError::AccessDenied), 3);
    EXPECT_EQ(static_cast<int>(RunError::OutOfMemory), 4);
    EXPECT_EQ(static_cast<int>(RunError::DllNotFound), 5);
    EXPECT_EQ(static_cast<int>(RunError::Unknown), 6);
}

// ==================== GenerateId 测试 ====================

TEST(TypesTest, GenerateId_HasPrefix) {
    std::wstring id = GenerateId(L"cat");
    EXPECT_TRUE(id.find(L"cat_") == 0);
}

TEST(TypesTest, GenerateId_NotEmpty) {
    std::wstring id = GenerateId(L"item");
    EXPECT_FALSE(id.empty());
}

TEST(TypesTest, GenerateId_Unique) {
    std::wstring id1 = GenerateId(L"cat");
    std::wstring id2 = GenerateId(L"cat");
    // 由于有时间戳和随机数，两次生成的 ID 应该不同
    // 注意：如果执行速度极快，可能有极小概率相同
    EXPECT_NE(id1, id2);
}

TEST(TypesTest, GenerateId_DifferentPrefixes) {
    std::wstring catId = GenerateId(L"cat");
    std::wstring itemId = GenerateId(L"item");
    EXPECT_TRUE(catId.find(L"cat_") == 0);
    EXPECT_TRUE(itemId.find(L"item_") == 0);
}
