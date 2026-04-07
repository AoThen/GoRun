#include "Config.h"
#include "Storage.h"

namespace mn {

void Config::Initialize(Storage* storage) {
    m_storage = storage;
}

std::wstring Config::GetGlobalHotkey() const {
    return m_storage ? m_storage->GetConfig("globalHotkey", L"Ctrl+Alt+M") : L"Ctrl+Alt+M";
}

void Config::SetGlobalHotkey(const std::wstring& hotkey) {
    if (m_storage) m_storage->SetConfig("globalHotkey", hotkey);
}

int Config::GetWindowX() const {
    if (!m_storage) return 100;
    try {
        return std::stoi(m_storage->GetConfig("windowX", L"100"));
    } catch (...) {
        return 100;
    }
}

int Config::GetWindowY() const {
    if (!m_storage) return 100;
    try {
        return std::stoi(m_storage->GetConfig("windowY", L"100"));
    } catch (...) {
        return 100;
    }
}

int Config::GetWindowWidth() const {
    if (!m_storage) return 800;
    try {
        return std::stoi(m_storage->GetConfig("windowWidth", L"800"));
    } catch (...) {
        return 800;
    }
}

int Config::GetWindowHeight() const {
    if (!m_storage) return 600;
    try {
        return std::stoi(m_storage->GetConfig("windowHeight", L"600"));
    } catch (...) {
        return 600;
    }
}

void Config::SetWindowPosition(int x, int y) {
    if (m_storage) {
        m_storage->SetConfig("windowX", std::to_wstring(x));
        m_storage->SetConfig("windowY", std::to_wstring(y));
    }
}

void Config::SetWindowSize(int width, int height) {
    if (m_storage) {
        m_storage->SetConfig("windowWidth", std::to_wstring(width));
        m_storage->SetConfig("windowHeight", std::to_wstring(height));
    }
}

} // namespace mn
