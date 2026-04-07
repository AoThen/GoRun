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
    
    Item* m_item = nullptr;
    bool m_visible = false;
    
    std::string m_nameBuf;
    std::string m_targetBuf;
    std::string m_argsBuf;
    std::string m_workingDirBuf;
    bool m_runAsAdmin = false;
    
    std::function<void(const Item&)> m_onSave;
};

} // namespace mn
