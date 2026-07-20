#include <gtest/gtest.h>
#include "core/ItemManager.h"
#include "core/Storage.h"
#include "core/IconCache.h"
#include "utils/PathUtils.h"
#include "utils/StringUtils.h"
#include <fstream>
#include <filesystem>

#ifdef _WIN32
#include <gdiplus.h>
#pragma comment(lib, "gdiplus.lib")
#endif

using namespace mn;

class ItemManagerTest : public ::testing::Test {
protected:
    std::wstring m_testDir;
    std::wstring m_testFile;
    Storage* m_storage = nullptr;
    IconCache* m_iconCache = nullptr;
    ItemManager* m_manager = nullptr;
#ifdef _WIN32
    ULONG_PTR m_gdiplusToken = 0;
#endif

    void SetUp() override {
#ifdef _WIN32
        Gdiplus::GdiplusStartupInput gdiplusInput;
        Gdiplus::GdiplusStartup(&m_gdiplusToken, &gdiplusInput, nullptr);
#endif
        m_testDir = PathUtils::GetAppDataPath() + L"\\GoRun_IMTest_" + std::to_wstring(std::time(nullptr));
        PathUtils::EnsureDirectory(m_testDir);
        m_testFile = m_testDir + L"\\test.json";

        m_storage = new Storage();
        m_storage->Initialize(m_testFile);

        m_iconCache = new IconCache();
        m_iconCache->Initialize(m_testDir + L"\\icons");

        m_manager = new ItemManager();
        m_manager->Initialize(m_storage, m_iconCache);
    }

    void TearDown() override {
        delete m_manager;
        delete m_iconCache;
        delete m_storage;

        std::error_code ec;
        std::filesystem::remove_all(StringUtils::WStringToUtf8(m_testDir), ec);
#ifdef _WIN32
        if (m_gdiplusToken) {
            Gdiplus::GdiplusShutdown(m_gdiplusToken);
        }
#endif
    }

    std::wstring AddCategory(const std::wstring& name) {
        Category cat;
        cat.id = L"cat_" + name;
        cat.name = name;
        m_manager->AddCategory(cat);
        return cat.id;
    }

    std::wstring AddItem(const std::wstring& name, const std::wstring& catId) {
        Item item;
        item.id = L"item_" + name;
        item.name = name;
        item.target = L"C:\\test.exe";
        item.categoryId = catId;
        m_manager->AddItem(item);
        return item.id;
    }
};

TEST_F(ItemManagerTest, MoveItem_UpdatesCategoryId) {
    auto cat1 = AddCategory(L"cat1");
    auto cat2 = AddCategory(L"cat2");
    auto itemId = AddItem(L"test", cat1);

    m_manager->MoveItem(itemId, cat2);

    auto items = m_manager->SearchItems(L"test");
    ASSERT_EQ(items.size(), 1);
    EXPECT_EQ(items[0].categoryId, cat2);
}

TEST_F(ItemManagerTest, MoveItem_SameCategory_NoLoss) {
    auto cat1 = AddCategory(L"cat1");
    auto itemId = AddItem(L"test", cat1);

    auto& items = m_manager->GetItems(cat1);
    EXPECT_EQ(items.size(), 1);

    m_manager->MoveItem(itemId, cat1);

    EXPECT_EQ(items.size(), 1);
    EXPECT_EQ(items[0].id, itemId);
    EXPECT_EQ(items[0].categoryId, cat1);
}

TEST_F(ItemManagerTest, MoveItem_NotFound_NoCrash) {
    m_manager->MoveItem(L"nonexistent", L"cat_any");
}

TEST_F(ItemManagerTest, MoveItem_ItemAppearsInTargetCategory) {
    auto cat1 = AddCategory(L"cat1");
    auto cat2 = AddCategory(L"cat2");
    auto itemId = AddItem(L"test", cat1);

    m_manager->MoveItem(itemId, cat2);

    auto& itemsInCat1 = m_manager->GetItems(cat1);
    auto& itemsInCat2 = m_manager->GetItems(cat2);

    EXPECT_EQ(itemsInCat1.size(), 0);
    ASSERT_EQ(itemsInCat2.size(), 1);
    EXPECT_EQ(itemsInCat2[0].id, itemId);
}

TEST_F(ItemManagerTest, UpdateItem_CrossCategory_MovesItem) {
    auto cat1 = AddCategory(L"cat1");
    auto cat2 = AddCategory(L"cat2");
    auto itemId = AddItem(L"test", cat1);

    Item updatedItem;
    updatedItem.id = itemId;
    updatedItem.name = L"updated";
    updatedItem.target = L"C:\\test.exe";
    updatedItem.categoryId = cat2;

    m_manager->UpdateItem(updatedItem);

    auto& itemsInCat1 = m_manager->GetItems(cat1);
    auto& itemsInCat2 = m_manager->GetItems(cat2);

    EXPECT_EQ(itemsInCat1.size(), 0);
    ASSERT_EQ(itemsInCat2.size(), 1);
    EXPECT_EQ(itemsInCat2[0].name, L"updated");
}

TEST_F(ItemManagerTest, UpdateItem_SameCategory_UpdatesInPlace) {
    auto cat1 = AddCategory(L"cat1");
    auto itemId = AddItem(L"test", cat1);

    Item updatedItem;
    updatedItem.id = itemId;
    updatedItem.name = L"updated";
    updatedItem.target = L"C:\\new.exe";
    updatedItem.categoryId = cat1;

    m_manager->UpdateItem(updatedItem);

    auto& items = m_manager->GetItems(cat1);
    ASSERT_EQ(items.size(), 1);
    EXPECT_EQ(items[0].name, L"updated");
    EXPECT_EQ(items[0].target, L"C:\\new.exe");
}

TEST_F(ItemManagerTest, SearchItems_ReturnsCorrectCategoryAfterMove) {
    auto cat1 = AddCategory(L"cat1");
    auto cat2 = AddCategory(L"cat2");
    auto itemId = AddItem(L"notepad", cat1);

    m_manager->MoveItem(itemId, cat2);

    auto results = m_manager->SearchItems(L"notepad");
    ASSERT_EQ(results.size(), 1);
    EXPECT_EQ(results[0].categoryId, cat2);
}