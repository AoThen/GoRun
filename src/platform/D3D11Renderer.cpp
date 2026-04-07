#include "D3D11Renderer.h"

#ifdef _WIN32
#include <imgui.h>
#include <imgui_impl_win32.h>
#include <imgui_impl_dx11.h>
#endif

namespace mn {

bool D3D11Renderer::Initialize(void* hwnd, int width, int height) {
#ifdef _WIN32
    DXGI_SWAP_CHAIN_DESC scd = {};
    scd.BufferCount = 2;
    scd.BufferDesc.Width = width;
    scd.BufferDesc.Height = height;
    scd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    scd.BufferDesc.RefreshRate.Numerator = 60;
    scd.BufferDesc.RefreshRate.Denominator = 1;
    scd.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;
    scd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    scd.OutputWindow = (HWND)hwnd;
    scd.SampleDesc.Count = 1;
    scd.SampleDesc.Quality = 0;
    scd.Windowed = TRUE;
    scd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;
    
    UINT createFlags = 0;
    
    D3D_FEATURE_LEVEL featureLevel;
    HRESULT hr = D3D11CreateDeviceAndSwapChain(
        nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
        createFlags, nullptr, 0,
        D3D11_SDK_VERSION, &scd,
        &m_swapChain, &m_device,
        &featureLevel, &m_context
    );
    
    if (FAILED(hr)) return false;
    
    ID3D11Texture2D* pBackBuffer;
    m_swapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    m_device->CreateRenderTargetView(pBackBuffer, nullptr, &m_rtv);
    pBackBuffer->Release();
    
    m_imguiContext = ImGui::CreateContext();
    ImGui::SetCurrentContext(m_imguiContext);
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    
    ImGui_ImplWin32_Init((HWND)hwnd);
    ImGui_ImplDX11_Init(m_device, m_context);
    
    return true;
#else
    return false;
#endif
}

void D3D11Renderer::Shutdown()
{
#ifdef _WIN32
    ImGui::SetCurrentContext(m_imguiContext);
    ImGui_ImplDX11_Shutdown();
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext(m_imguiContext);
    
    if (m_rtv) m_rtv->Release();
    if (m_swapChain) m_swapChain->Release();
    if (m_context) m_context->Release();
    if (m_device) m_device->Release();
#endif
}

void D3D11Renderer::NewFrame()
{
#ifdef _WIN32
    ImGui::SetCurrentContext(m_imguiContext);
    ImGui_ImplDX11_NewFrame();
    ImGui_ImplWin32_NewFrame();
    ImGui::NewFrame();
#endif
}

void D3D11Renderer::Render()
{
#ifdef _WIN32
    ImGui::SetCurrentContext(m_imguiContext);
    ImGui::Render();
    
    float clear_color[4] = { 0.1f, 0.1f, 0.12f, 1.0f };
    m_context->OMSetRenderTargets(1, &m_rtv, nullptr);
    m_context->ClearRenderTargetView(m_rtv, clear_color);
    
    ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());
    m_swapChain->Present(1, 0);
#endif
}

void D3D11Renderer::Resize(int width, int height)
{
#ifdef _WIN32
    if (m_rtv) {
        m_rtv->Release();
        m_rtv = nullptr;
    }
    
    m_swapChain->ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, 0);
    
    ID3D11Texture2D* pBackBuffer;
    m_swapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    m_device->CreateRenderTargetView(pBackBuffer, nullptr, &m_rtv);
    pBackBuffer->Release();
#endif
}

} // namespace mn
