#pragma once

#include <string>
#include <unordered_map>

namespace mn {

class Localization {
public:
    void Initialize(const std::wstring& langDir);
    void SetLanguage(const std::string& langCode);
    const std::string& GetCurrentLanguage() const { return m_currentLang; }
    std::wstring GetString(const std::string& key) const;

    static Localization* Get();

private:
    bool LoadLanguageFile(const std::string& langCode);

    std::unordered_map<std::string, std::wstring> m_strings;
    std::string m_currentLang = "zh-CN";
    std::wstring m_langDir;

    static Localization* s_instance;
};

inline std::wstring Tr(const std::string& key) {
    auto* loc = Localization::Get();
    return loc ? loc->GetString(key) : L"?" + std::wstring(key.begin(), key.end()) + L"?";
}

} // namespace mn