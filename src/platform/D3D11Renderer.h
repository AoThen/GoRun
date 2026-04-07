#pragma once

#ifdef _WIN32
#include <d3d11.h>
#endif

namespace mn {

class D3D11Renderer {
public:
    bool Initialize(void* hwnd, int width, int height);
    void Shutdown();
    void NewFrame();
    void Render();
    void Resize(int width, int height);

private:
#ifdef _WIN32
    ID3D11Device* m_device = nullptr;
    ID3D11DeviceContext* m_context = nullptr;
    IDXGISwapChain* m_swapChain = nullptr;
    ID3D11RenderTargetView* m_rtv = nullptr;
    void* m_imguiContext = nullptr;
#endif
};

} // namespace mn
