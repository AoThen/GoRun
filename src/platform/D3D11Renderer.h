#pragma once

#ifdef _WIN32
#include <d3d11.h>
#include <imgui.h>
#include <string>
#endif

namespace mn {

class D3D11Renderer {
public:
    bool Initialize(void* hwnd, int width, int height);
    void Shutdown();
    void NewFrame();
    void Render();
    void Resize(int width, int height);
    
    // DPI 相关
    float GetDpiScale() const { return m_dpiScale; }
    void UpdateDpiScale(float newScale);
    
    // 纹理加载
    ImTextureID LoadTexture(const std::wstring& path);
    void UnloadTexture(ImTextureID texture);
    void UnloadTextureByPath(const std::wstring& path);
    int GetTextureWidth(ImTextureID texture);
    int GetTextureHeight(ImTextureID texture);

private:
#ifdef _WIN32
    ID3D11Device* m_device = nullptr;
    ID3D11DeviceContext* m_context = nullptr;
    IDXGISwapChain* m_swapChain = nullptr;
    ID3D11RenderTargetView* m_rtv = nullptr;
    void* m_imguiContext = nullptr;
    float m_dpiScale = 1.0f;
#endif
};

} // namespace mn