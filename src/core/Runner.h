#pragma once

#include "Types.h"
#include <functional>

namespace mn {

class Runner {
public:
    using RunCallback = std::function<void(const Item&, bool success)>;
    
    RunResult Run(const Item& item);
    RunResult RunAsAdmin(const Item& item);
    
    void SetRunCallback(RunCallback callback) { m_runCallback = callback; }
    void SetSearchQuery(const std::wstring& query) { m_searchQuery = query; }

private:
    std::wstring ExpandVariables(const std::wstring& input, const Item& item);
    std::wstring GetErrorMessage(unsigned long errorCode);
    RunError MapError(unsigned long errorCode);
    
    RunCallback m_runCallback;
    std::wstring m_searchQuery;
};

} // namespace mn
