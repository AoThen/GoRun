#include <gtest/gtest.h>
#include "core/Config.h"
#include "core/Storage.h"
#include "utils/PathUtils.h"
#include <fstream>
#include <filesystem>

using namespace mn;

// 测试夹具
class ConfigTest : public ::testing::Test {
protected:
    std::wstring m_testDir;
    std::wstring m_testFile;
    Storage* m_storage = nullptr;
    Config* m_config = nullptr;
    
    void SetUp() override {
        // 创建临时测试目录
        m_testDir = PathUtils::GetAppDataPath() + L"\\GoRun_ConfigTest_" + std::to_wstring(std::time(nullptr));
        PathUtils::EnsureDirectory(m_testDir);
        m_testFile = m_testDir + L"\\test_config.json";
        
        m_storage = new Storage();
        m_storage->Initialize(m_testFile);
        
        m_config = new Config();
        m_config->Initialize(m_storage);
    }
    
    void TearDown() override {
        delete m_config;
        delete m_storage;
        
        // 清理测试目录
        std::error_code ec;
        std::filesystem::remove_all(m_testDir, ec);
    }
};

// ==================== GlobalHotkey 测试 ====================

TEST_F(ConfigTest, GetGlobalHotkey_Default) {
    // 未设置时返回默认值
    EXPECT_EQ(m_config->GetGlobalHotkey(), L"Ctrl+Alt+Z");
}

TEST_F(ConfigTest, SetGlobalHotkey_Success) {
    m_config->SetGlobalHotkey(L"Ctrl+Alt+M");
    EXPECT_EQ(m_config->GetGlobalHotkey(), L"Ctrl+Alt+M");
}

TEST_F(ConfigTest, SetGlobalHotkey_Custom) {
    m_config->SetGlobalHotkey(L"Win+R");
    EXPECT_EQ(m_config->GetGlobalHotkey(), L"Win+R");
}

TEST_F(ConfigTest, SetGlobalHotkey_Persist) {
    m_config->SetGlobalHotkey(L"Ctrl+Shift+Space");
    
    // 重新创建 Config 读取持久化的值
    Config config2;
    config2.Initialize(m_storage);
    EXPECT_EQ(config2.GetGlobalHotkey(), L"Ctrl+Shift+Space");
}

// ==================== WindowPosition 测试 ====================

TEST_F(ConfigTest, GetWindowPosition_Default) {
    EXPECT_EQ(m_config->GetWindowX(), 100);
    EXPECT_EQ(m_config->GetWindowY(), 100);
}

TEST_F(ConfigTest, SetWindowPosition_Success) {
    m_config->SetWindowPosition(200, 300);
    EXPECT_EQ(m_config->GetWindowX(), 200);
    EXPECT_EQ(m_config->GetWindowY(), 300);
}

TEST_F(ConfigTest, SetWindowPosition_Zero) {
    m_config->SetWindowPosition(0, 0);
    EXPECT_EQ(m_config->GetWindowX(), 0);
    EXPECT_EQ(m_config->GetWindowY(), 0);
}

TEST_F(ConfigTest, SetWindowPosition_Negative) {
    // 负数位置（多显示器场景）
    m_config->SetWindowPosition(-100, -200);
    EXPECT_EQ(m_config->GetWindowX(), -100);
    EXPECT_EQ(m_config->GetWindowY(), -200);
}

TEST_F(ConfigTest, SetWindowPosition_Large) {
    m_config->SetWindowPosition(1920, 1080);
    EXPECT_EQ(m_config->GetWindowX(), 1920);
    EXPECT_EQ(m_config->GetWindowY(), 1080);
}

// ==================== WindowSize 测试 ====================

TEST_F(ConfigTest, GetWindowSize_Default) {
    EXPECT_EQ(m_config->GetWindowWidth(), 800);
    EXPECT_EQ(m_config->GetWindowHeight(), 600);
}

TEST_F(ConfigTest, SetWindowSize_Success) {
    m_config->SetWindowSize(1024, 768);
    EXPECT_EQ(m_config->GetWindowWidth(), 1024);
    EXPECT_EQ(m_config->GetWindowHeight(), 768);
}

TEST_F(ConfigTest, SetWindowSize_Small) {
    m_config->SetWindowSize(400, 300);
    EXPECT_EQ(m_config->GetWindowWidth(), 400);
    EXPECT_EQ(m_config->GetWindowHeight(), 300);
}

TEST_F(ConfigTest, SetWindowSize_Large) {
    m_config->SetWindowSize(2560, 1440);
    EXPECT_EQ(m_config->GetWindowWidth(), 2560);
    EXPECT_EQ(m_config->GetWindowHeight(), 1440);
}

// ==================== AutoStart 测试 ====================

TEST_F(ConfigTest, GetAutoStart_Default) {
    EXPECT_FALSE(m_config->GetAutoStart());
}

TEST_F(ConfigTest, SetAutoStart_Enable) {
    m_config->SetAutoStart(true);
    EXPECT_TRUE(m_config->GetAutoStart());
}

TEST_F(ConfigTest, SetAutoStart_Disable) {
    m_config->SetAutoStart(true);
    m_config->SetAutoStart(false);
    EXPECT_FALSE(m_config->GetAutoStart());
}

TEST_F(ConfigTest, SetAutoStart_Persist) {
    m_config->SetAutoStart(true);
    
    // 重新创建 Config
    Config config2;
    config2.Initialize(m_storage);
    EXPECT_TRUE(config2.GetAutoStart());
}

// ==================== NullStorage 测试 ====================

TEST(ConfigNullTest, NullStorage_ReturnsDefaults) {
    Config config;
    config.Initialize(nullptr);
    
    // Storage 为空时应返回默认值
    EXPECT_EQ(config.GetGlobalHotkey(), L"Ctrl+Alt+Z");
    EXPECT_EQ(config.GetWindowX(), 100);
    EXPECT_EQ(config.GetWindowY(), 100);
    EXPECT_EQ(config.GetWindowWidth(), 800);
    EXPECT_EQ(config.GetWindowHeight(), 600);
    EXPECT_FALSE(config.GetAutoStart());
}

TEST(ConfigNullTest, NullStorage_SetDoesNotCrash) {
    Config config;
    config.Initialize(nullptr);
    
    // 设置操作不应崩溃（但不生效）
    config.SetGlobalHotkey(L"test");
    config.SetWindowPosition(1, 2);
    config.SetWindowSize(3, 4);
    config.SetAutoStart(true);
    
    // 仍返回默认值
    EXPECT_EQ(config.GetGlobalHotkey(), L"Ctrl+Alt+Z");
}

// ==================== 持久化测试 ====================

TEST_F(ConfigTest, AllSettings_Persist) {
    // 设置所有配置
    m_config->SetGlobalHotkey(L"Win+Q");
    m_config->SetWindowPosition(500, 400);
    m_config->SetWindowSize(1200, 800);
    m_config->SetAutoStart(true);
    
    // 重新加载
    Storage storage2;
    storage2.Load(m_testFile);
    Config config2;
    config2.Initialize(&storage2);
    
    // 验证所有配置已持久化
    EXPECT_EQ(config2.GetGlobalHotkey(), L"Win+Q");
    EXPECT_EQ(config2.GetWindowX(), 500);
    EXPECT_EQ(config2.GetWindowY(), 400);
    EXPECT_EQ(config2.GetWindowWidth(), 1200);
    EXPECT_EQ(config2.GetWindowHeight(), 800);
    EXPECT_TRUE(config2.GetAutoStart());
}

// ==================== 边界情况测试 ====================

TEST_F(ConfigTest, WindowPosition_Extreme) {
    // 极端值测试
    m_config->SetWindowPosition(-10000, -10000);
    EXPECT_EQ(m_config->GetWindowX(), -10000);
    EXPECT_EQ(m_config->GetWindowY(), -10000);
    
    m_config->SetWindowPosition(10000, 10000);
    EXPECT_EQ(m_config->GetWindowX(), 10000);
    EXPECT_EQ(m_config->GetWindowY(), 10000);
}

TEST_F(ConfigTest, WindowSize_Extreme) {
    // 极端尺寸测试
    m_config->SetWindowSize(1, 1);
    EXPECT_EQ(m_config->GetWindowWidth(), 1);
    EXPECT_EQ(m_config->GetWindowHeight(), 1);
    
    m_config->SetWindowSize(8192, 4320);  // 8K
    EXPECT_EQ(m_config->GetWindowWidth(), 8192);
    EXPECT_EQ(m_config->GetWindowHeight(), 4320);
}
