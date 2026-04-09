#include <gtest/gtest.h>
#include "core/Storage.h"
#include "utils/PathUtils.h"
#include "utils/StringUtils.h"
#include <fstream>
#include <filesystem>

using namespace mn;

// 测试夹具：创建临时目录和文件
class StorageTest : public ::testing::Test {
protected:
    std::wstring m_testDir;
    std::wstring m_testFile;
    
    void SetUp() override {
        // 创建临时测试目录
        m_testDir = PathUtils::GetAppDataPath() + L"\\GoRun_Test_" + std::to_wstring(std::time(nullptr));
        PathUtils::EnsureDirectory(m_testDir);
        m_testFile = m_testDir + L"\\test_storage.json";
    }
    
    void TearDown() override {
        // 清理测试文件
        std::error_code ec;
        std::filesystem::remove_all(StringUtils::WStringToUtf8(m_testDir), ec);
    }
};

// ==================== Initialize 测试 ====================

TEST_F(StorageTest, Initialize_CreatesFile) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    EXPECT_TRUE(PathUtils::Exists(m_testFile));
}

TEST_F(StorageTest, Initialize_CreatesValidJson) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    // 读取文件验证 JSON 结构
    std::ifstream file(m_testFile);
    ASSERT_TRUE(file.is_open());
    
    std::string content((std::istreambuf_iterator<char>(file)),
                        std::istreambuf_iterator<char>());
    file.close();
    
    // 应包含 version 字段
    EXPECT_TRUE(content.find("\"version\"") != std::string::npos);
}

// ==================== Category CRUD 测试 ====================

TEST_F(StorageTest, AddCategory_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"测试分类";
    cat.sortOrder = 1;
    cat.viewType = ViewType::Icon;
    cat.iconSize = 48;
    
    EXPECT_TRUE(storage.AddCategory(cat));
    
    auto categories = storage.GetCategories();
    ASSERT_EQ(categories.size(), 1);
    EXPECT_EQ(categories[0].id, L"cat_001");
    EXPECT_EQ(categories[0].name, L"测试分类");
}

TEST_F(StorageTest, AddCategory_EmptyIdFails) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.name = L"测试分类";
    
    EXPECT_FALSE(storage.AddCategory(cat));
}

TEST_F(StorageTest, GetCategory_Existing) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"测试分类";
    storage.AddCategory(cat);
    
    Category result = storage.GetCategory(L"cat_001");
    EXPECT_EQ(result.id, L"cat_001");
    EXPECT_EQ(result.name, L"测试分类");
}

TEST_F(StorageTest, GetCategory_NonExisting) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category result = storage.GetCategory(L"nonexistent");
    EXPECT_TRUE(result.id.empty());
}

TEST_F(StorageTest, UpdateCategory_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"原始名称";
    storage.AddCategory(cat);
    
    cat.name = L"更新名称";
    cat.sortOrder = 5;
    EXPECT_TRUE(storage.UpdateCategory(cat));
    
    Category result = storage.GetCategory(L"cat_001");
    EXPECT_EQ(result.name, L"更新名称");
    EXPECT_EQ(result.sortOrder, 5);
}

TEST_F(StorageTest, UpdateCategory_NonExistingFails) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"nonexistent";
    cat.name = L"测试";
    
    EXPECT_FALSE(storage.UpdateCategory(cat));
}

TEST_F(StorageTest, DeleteCategory_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"测试分类";
    storage.AddCategory(cat);
    
    EXPECT_TRUE(storage.DeleteCategory(L"cat_001"));
    
    auto categories = storage.GetCategories();
    EXPECT_EQ(categories.size(), 0);
}

TEST_F(StorageTest, DeleteCategory_WithItems) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    // 添加分类
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"测试分类";
    storage.AddCategory(cat);
    
    // 添加项目
    Item item;
    item.id = L"item_001";
    item.name = L"测试项目";
    item.categoryId = L"cat_001";
    storage.AddItem(item);
    
    // 删除分类应同时删除项目
    storage.DeleteCategory(L"cat_001");
    
    auto items = storage.GetItems();
    EXPECT_EQ(items.size(), 0);
}

// ==================== Item CRUD 测试 ====================

TEST_F(StorageTest, AddItem_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.id = L"item_001";
    item.name = L"测试项目";
    item.target = L"C:\\test.exe";
    item.categoryId = L"cat_001";
    
    EXPECT_TRUE(storage.AddItem(item));
    
    auto items = storage.GetItems();
    ASSERT_EQ(items.size(), 1);
    EXPECT_EQ(items[0].id, L"item_001");
    EXPECT_EQ(items[0].name, L"测试项目");
    EXPECT_EQ(items[0].target, L"C:\\test.exe");
}

TEST_F(StorageTest, AddItem_EmptyIdFails) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.name = L"测试项目";
    
    EXPECT_FALSE(storage.AddItem(item));
}

TEST_F(StorageTest, AddItem_AllFields) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.id = L"item_001";
    item.name = L"完整项目";
    item.target = L"C:\\app.exe";
    item.arguments = L"--flag --port=8080";
    item.workingDir = L"C:\\work";
    item.iconPath = L"C:\\icon.ico";
    item.iconIndex = 2;
    item.runAsAdmin = true;
    item.runCount = 10;
    item.keywords = L"key1,key2";
    item.remark = L"备注信息";
    item.categoryId = L"cat_001";
    item.sortOrder = 5;
    
    EXPECT_TRUE(storage.AddItem(item));
    
    Item result = storage.GetItem(L"item_001");
    EXPECT_EQ(result.name, L"完整项目");
    EXPECT_EQ(result.arguments, L"--flag --port=8080");
    EXPECT_EQ(result.workingDir, L"C:\\work");
    EXPECT_EQ(result.iconPath, L"C:\\icon.ico");
    EXPECT_EQ(result.iconIndex, 2);
    EXPECT_TRUE(result.runAsAdmin);
    EXPECT_EQ(result.runCount, 10);
    EXPECT_EQ(result.keywords, L"key1,key2");
    EXPECT_EQ(result.remark, L"备注信息");
    EXPECT_EQ(result.sortOrder, 5);
}

TEST_F(StorageTest, GetItem_Existing) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.id = L"item_001";
    item.name = L"测试项目";
    storage.AddItem(item);
    
    Item result = storage.GetItem(L"item_001");
    EXPECT_EQ(result.id, L"item_001");
    EXPECT_EQ(result.name, L"测试项目");
}

TEST_F(StorageTest, GetItem_NonExisting) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item result = storage.GetItem(L"nonexistent");
    EXPECT_TRUE(result.id.empty());
}

TEST_F(StorageTest, UpdateItem_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.id = L"item_001";
    item.name = L"原始名称";
    item.target = L"C:\\old.exe";
    storage.AddItem(item);
    
    item.name = L"更新名称";
    item.target = L"C:\\new.exe";
    item.runCount = 5;
    EXPECT_TRUE(storage.UpdateItem(item));
    
    Item result = storage.GetItem(L"item_001");
    EXPECT_EQ(result.name, L"更新名称");
    EXPECT_EQ(result.target, L"C:\\new.exe");
    EXPECT_EQ(result.runCount, 5);
}

TEST_F(StorageTest, DeleteItem_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.id = L"item_001";
    item.name = L"测试项目";
    storage.AddItem(item);
    
    EXPECT_TRUE(storage.DeleteItem(L"item_001"));
    
    auto items = storage.GetItems();
    EXPECT_EQ(items.size(), 0);
}

TEST_F(StorageTest, GetItems_ByCategory) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    // 添加两个分类的项目
    Item item1;
    item1.id = L"item_001";
    item1.name = L"项目1";
    item1.categoryId = L"cat_001";
    storage.AddItem(item1);
    
    Item item2;
    item2.id = L"item_002";
    item2.name = L"项目2";
    item2.categoryId = L"cat_001";
    storage.AddItem(item2);
    
    Item item3;
    item3.id = L"item_003";
    item3.name = L"项目3";
    item3.categoryId = L"cat_002";
    storage.AddItem(item3);
    
    auto items1 = storage.GetItems(L"cat_001");
    EXPECT_EQ(items1.size(), 2);
    
    auto items2 = storage.GetItems(L"cat_002");
    EXPECT_EQ(items2.size(), 1);
    
    auto allItems = storage.GetItems();
    EXPECT_EQ(allItems.size(), 3);
}

TEST_F(StorageTest, GetItemIdsByCategory) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item1;
    item1.id = L"item_001";
    item1.categoryId = L"cat_001";
    storage.AddItem(item1);
    
    Item item2;
    item2.id = L"item_002";
    item2.categoryId = L"cat_001";
    storage.AddItem(item2);
    
    auto ids = storage.GetItemIdsByCategory(L"cat_001");
    ASSERT_EQ(ids.size(), 2);
    EXPECT_EQ(ids[0], L"item_001");
    EXPECT_EQ(ids[1], L"item_002");
}

// ==================== Config 测试 ====================

TEST_F(StorageTest, SetConfig_Success) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    EXPECT_TRUE(storage.SetConfig("globalHotkey", L"Ctrl+Alt+M"));
    EXPECT_EQ(storage.GetConfig("globalHotkey"), L"Ctrl+Alt+M");
}

TEST_F(StorageTest, GetConfig_DefaultValue) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    EXPECT_EQ(storage.GetConfig("nonexistent", L"default"), L"default");
}

TEST_F(StorageTest, SetConfig_Multiple) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    storage.SetConfig("key1", L"value1");
    storage.SetConfig("key2", L"value2");
    storage.SetConfig("key3", L"value3");
    
    EXPECT_EQ(storage.GetConfig("key1"), L"value1");
    EXPECT_EQ(storage.GetConfig("key2"), L"value2");
    EXPECT_EQ(storage.GetConfig("key3"), L"value3");
}

// ==================== Load/Save 测试 ====================

TEST_F(StorageTest, Load_ExistingFile) {
    // 先创建并保存数据
    {
        Storage storage;
        storage.Initialize(m_testFile);
        
        Category cat;
        cat.id = L"cat_001";
        cat.name = L"测试分类";
        storage.AddCategory(cat);
        
        Item item;
        item.id = L"item_001";
        item.name = L"测试项目";
        item.categoryId = L"cat_001";
        storage.AddItem(item);
        
        storage.SetConfig("testKey", L"testValue");
    }
    
    // 重新加载数据
    Storage storage;
    EXPECT_TRUE(storage.Load(m_testFile));
    
    auto categories = storage.GetCategories();
    ASSERT_EQ(categories.size(), 1);
    EXPECT_EQ(categories[0].name, L"测试分类");
    
    auto items = storage.GetItems();
    ASSERT_EQ(items.size(), 1);
    EXPECT_EQ(items[0].name, L"测试项目");
    
    EXPECT_EQ(storage.GetConfig("testKey"), L"testValue");
}

TEST_F(StorageTest, Load_InvalidJson) {
    // 写入无效 JSON
    std::ofstream file(m_testFile);
    file << "{ invalid json }";
    file.close();
    
    Storage storage;
    EXPECT_FALSE(storage.Load(m_testFile));
}

TEST_F(StorageTest, Save_NewPath) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"测试分类";
    storage.AddCategory(cat);
    
    std::wstring newPath = m_testDir + L"\\new_storage.json";
    EXPECT_TRUE(storage.Save(newPath));
    EXPECT_TRUE(PathUtils::Exists(newPath));
}

// ==================== 中文支持测试 ====================

TEST_F(StorageTest, ChineseCategoryName) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Category cat;
    cat.id = L"cat_001";
    cat.name = L"常用软件";
    storage.AddCategory(cat);
    
    Storage storage2;
    storage2.Load(m_testFile);
    
    Category result = storage2.GetCategory(L"cat_001");
    EXPECT_EQ(result.name, L"常用软件");
}

TEST_F(StorageTest, ChineseItemName) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    Item item;
    item.id = L"item_001";
    item.name = L"记事本";
    item.target = L"C:\\Windows\\notepad.exe";
    item.remark = L"系统自带文本编辑器";
    storage.AddItem(item);
    
    Storage storage2;
    storage2.Load(m_testFile);
    
    Item result = storage2.GetItem(L"item_001");
    EXPECT_EQ(result.name, L"记事本");
    EXPECT_EQ(result.remark, L"系统自带文本编辑器");
}

// ==================== 边界情况测试 ====================

TEST_F(StorageTest, EmptyData) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    EXPECT_EQ(storage.GetCategories().size(), 0);
    EXPECT_EQ(storage.GetItems().size(), 0);
}

TEST_F(StorageTest, MultipleCategoriesAndItems) {
    Storage storage;
    storage.Initialize(m_testFile);
    
    // 添加多个分类
    for (int i = 0; i < 5; i++) {
        Category cat;
        cat.id = L"cat_" + std::to_wstring(i);
        cat.name = L"分类" + std::to_wstring(i);
        cat.sortOrder = i;
        storage.AddCategory(cat);
    }
    
    // 添加多个项目
    for (int i = 0; i < 20; i++) {
        Item item;
        item.id = L"item_" + std::to_wstring(i);
        item.name = L"项目" + std::to_wstring(i);
        item.categoryId = L"cat_" + std::to_wstring(i % 5);
        storage.AddItem(item);
    }
    
    EXPECT_EQ(storage.GetCategories().size(), 5);
    EXPECT_EQ(storage.GetItems().size(), 20);
    
    // 验证每个分类的项目数量
    for (int i = 0; i < 5; i++) {
        auto items = storage.GetItems(L"cat_" + std::to_wstring(i));
        EXPECT_EQ(items.size(), 4);  // 20 / 5 = 4
    }
}
