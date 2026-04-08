#pragma once

#include "core/Types.h"
#include <functional>
#include <vector>

namespace mn {

class CategoryTab {
public:
    void SetCategories(std::vector<Category>* categories);
    void Render();
    
    void OnCategoryChanged(std::function<void(const std::wstring& id)> callback);
    void OnCategoryAdd(std::function<void()> callback);
    void OnCategoryDelete(std::function<void(const std::wstring& id)> callback);
    void OnCategoryRename(std::function<void(const std::wstring& id)> callback);
    
    void SetCurrentCategory(const std::wstring& id);
    std::wstring GetCurrentCategory() const;

private:
    void RenderContextMenu(const Category& cat);
    
    std::vector<Category>* m_categories = nullptr;
    std::wstring m_currentId;
    std::function<void(const std::wstring&)> m_onChanged;
    std::function<void()> m_onAdd;
    std::function<void(const std::wstring&)> m_onDelete;
    std::function<void(const std::wstring&)> m_onRename;
};

} // namespace mn
