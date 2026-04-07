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
    
    void SetCurrentCategory(const std::wstring& id);
    std::wstring GetCurrentCategory() const;

private:
    std::vector<Category>* m_categories = nullptr;
    std::wstring m_currentId;
    std::function<void(const std::wstring&)> m_onChanged;
};

} // namespace mn
