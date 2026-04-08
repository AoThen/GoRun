#pragma once

#include "core/Types.h"
#include <functional>
#include <string>

namespace mn {

class EditDialog {
public:
    void Show(Item* item);
    void Hide();
    bool IsVisible() const;
    void Render();
    
    void OnSave(std::function<void(const Item&)> callback);

private:
    void LoadFromItem();
    void SaveToItem();
    void BrowseTarget();
    void BrowseIcon();
    
    Item* m_item = nullptr;
    bool m_visible = false;
    
    std::string m_nameBuf;
    std::string m_targetBuf;
    std::string m_argsBuf;
    std::string m_workingDirBuf;
    std::string m_keywordsBuf;      // 关键词
    std::string m_remarkBuf;        // 备注
    std::string m_iconPathBuf;      // 图标路径
    int m_iconIndex = 0;            // 图标索引
    bool m_runAsAdmin = false;
    
    std::function<void(const Item&)> m_onSave;
};

} // namespace mn
