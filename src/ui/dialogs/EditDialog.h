#pragma once

#include "core/Types.h"
#include <functional>
#include <string>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn {

class EditDialog {
public:
    void Show(Item* item);
    void Hide();
    bool IsVisible() const;
    void Render();

    void OnSave(std::function<void(const Item&)> callback);

#ifdef _WIN32
    void SetOwner(HWND hwnd) { m_hwndOwner = hwnd; }
#endif

private:
    void LoadFromItem();
    void SaveToItem();
    void BrowseTarget();
    void BrowseIcon();

    Item* m_item = nullptr;
    bool m_visible = false;
    bool m_openPopup = false;
    bool m_showInputError = false;

    std::string m_nameBuf;
    std::string m_targetBuf;
    std::string m_argsBuf;
    std::string m_workingDirBuf;
    std::string m_keywordsBuf;
    std::string m_remarkBuf;
    std::string m_iconPathBuf;
    int m_iconIndex = 0;
    bool m_runAsAdmin = false;

#ifdef _WIN32
    HWND m_hwndOwner = nullptr;
#endif

    std::function<void(const Item&)> m_onSave;
};

} // namespace mn
