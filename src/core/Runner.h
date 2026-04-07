#pragma once

#include "Types.h"

namespace mn {

class Runner {
public:
    RunResult Run(const Item& item);
    RunResult RunAsAdmin(const Item& item);

private:
    std::wstring GetErrorMessage(unsigned long errorCode);
    RunError MapError(unsigned long errorCode);
};

} // namespace mn
