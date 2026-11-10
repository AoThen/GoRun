#include "Localization.h"
#include "utils/PathUtils.h"
#include "utils/StringUtils.h"
#include "utils/Logger.h"
#include <nlohmann/json.hpp>
#include <fstream>
#include <filesystem>

namespace mn {

Localization* Localization::s_instance = nullptr;

Localization* Localization::Get() {
    return s_instance;
}

void Localization::Initialize(const std::wstring& langDir) {
    s_instance = this;
    m_langDir = langDir;
    LoadLanguageFile(m_currentLang);
}

void Localization::SetLanguage(const std::string& langCode) {
    m_currentLang = langCode;
    LoadLanguageFile(langCode);
}

bool Localization::LoadLanguageFile(const std::string& langCode) {
    std::wstring filePath = m_langDir + L"\\" + StringUtils::Utf8ToWString(langCode) + L".json";

    std::ifstream file(filePath);
    if (!file.is_open()) {
        LOG_ERROR("Localization: Failed to load language file: " + langCode);
        return false;
    }

    try {
        nlohmann::json j;
        file >> j;

        m_strings.clear();
        for (auto& [key, val] : j.items()) {
            if (val.is_string()) {
                m_strings[key] = StringUtils::Utf8ToWString(val.get<std::string>());
            }
        }
        return true;
    } catch (const std::exception& e) {
        LOG_ERROR(std::string("Localization: Failed to parse language file: ") + e.what());
        return false;
    }
}

std::wstring Localization::GetString(const std::string& key) const {
    auto it = m_strings.find(key);
    if (it != m_strings.end()) {
        return it->second;
    }
    return L"?" + StringUtils::Utf8ToWString(key) + L"?";
}

} // namespace mn