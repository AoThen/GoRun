#include "D3D11Renderer.h"

#ifdef _WIN32
#include <imgui.h>
#include <imgui_impl_win32.h>
#include <imgui_impl_dx11.h>
#include <windows.h>
#include <gdiplus.h>
#include <unordered_map>
#include "../ui/Theme.h"
#include "utils/Logger.h"
#pragma comment(lib, "gdiplus.lib")
#endif

namespace mn {

#ifdef _WIN32
// 纹理缓存
static std::unordered_map<std::wstring, ID3D11ShaderResourceView*> s_textureCache;
static Gdiplus::GdiplusStartupInput s_gdiplusInput;
static ULONG_PTR s_gdiplusToken = 0;

static ID3D11ShaderResourceView* LoadTextureFromFile(ID3D11Device* device, const std::wstring& path, int* width, int* height) {
    // 检查缓存
    auto it = s_textureCache.find(path);
    if (it != s_textureCache.end()) {
        return it->second;
    }
    
    // 初始化 GDI+
    static bool gdiplusInit = false;
    if (!gdiplusInit) {
        Gdiplus::GdiplusStartup(&s_gdiplusToken, &s_gdiplusInput, nullptr);
        gdiplusInit = true;
    }
    
    // 加载图片
    Gdiplus::Bitmap* bitmap = Gdiplus::Bitmap::FromFile(path.c_str());
    if (!bitmap || bitmap->GetLastStatus() != Gdiplus::Ok) {
        delete bitmap;
        return nullptr;
    }
    
    if (width) *width = bitmap->GetWidth();
    if (height) *height = bitmap->GetHeight();
    
    // 转换为 32 位 ARGB
    Gdiplus::Bitmap* converted = new Gdiplus::Bitmap(bitmap->GetWidth(), bitmap->GetHeight(), PixelFormat32bppARGB);
    Gdiplus::Graphics graphics(converted);
    graphics.DrawImage(bitmap, 0, 0, bitmap->GetWidth(), bitmap->GetHeight());
    delete bitmap;
    bitmap = converted;
    
    // 锁定像素数据
    Gdiplus::BitmapData bmpData;
    Gdiplus::Rect rect(0, 0, bitmap->GetWidth(), bitmap->GetHeight());
    bitmap->LockBits(&rect, Gdiplus::ImageLockModeRead, PixelFormat32bppARGB, &bmpData);
    
    // 创建 D3D11 纹理
    D3D11_TEXTURE2D_DESC texDesc = {};
    texDesc.Width = bitmap->GetWidth();
    texDesc.Height = bitmap->GetHeight();
    texDesc.MipLevels = 1;
    texDesc.ArraySize = 1;
    texDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    texDesc.SampleDesc.Count = 1;
    texDesc.Usage = D3D11_USAGE_DEFAULT;
    texDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
    
    // 转换 BGRA 到 RGBA
    unsigned char* pixels = (unsigned char*)bmpData.Scan0;
    for (int i = 0; i < bitmap->GetWidth() * bitmap->GetHeight(); i++) {
        unsigned char b = pixels[i * 4];
        pixels[i * 4] = pixels[i * 4 + 2];
        pixels[i * 4 + 2] = b;
    }
    
    D3D11_SUBRESOURCE_DATA initData = {};
    initData.pSysMem = pixels;
    initData.SysMemPitch = bmpData.Stride;
    
    ID3D11Texture2D* texture = nullptr;
    device->CreateTexture2D(&texDesc, &initData, &texture);
    
    bitmap->UnlockBits(&bmpData);
    delete bitmap;
    
    if (!texture) return nullptr;
    
    // 创建着色器资源视图
    D3D11_SHADER_RESOURCE_VIEW_DESC srvDesc = {};
    srvDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    srvDesc.ViewDimension = D3D11_SRV_DIMENSION_TEXTURE2D;
    srvDesc.Texture2D.MipLevels = texDesc.MipLevels;
    
    ID3D11ShaderResourceView* srv = nullptr;
    device->CreateShaderResourceView(texture, &srvDesc, &srv);
    texture->Release();
    
    if (srv) {
        s_textureCache[path] = srv;
    }
    
    return srv;
}
#endif

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
    ImGui::SetCurrentContext(static_cast<ImGuiContext*>(m_imguiContext));
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    
    // 获取 DPI 缩放因子
    HDC hdc = GetDC(nullptr);
    int dpi = GetDeviceCaps(hdc, LOGPIXELSX);
    ReleaseDC(nullptr, hdc);
    float dpiScale = dpi / 96.0f;
    m_dpiScale = dpiScale;
    
    // 加载微软雅黑字体支持中文
    io.Fonts->Clear();
    
    // 字体配置 - 提高采样率以获得更清晰的字体
    ImFontConfig config;
    config.OversampleH = 3;  // 提高水平采样率
    config.OversampleV = 1;
    config.PixelSnapH = true;
    
    // 设置中文字符范围 - 包含基本中文和标点符号
    ImVector<ImWchar> ranges;
    ImFontGlyphRangesBuilder builder;
    builder.AddRanges(io.Fonts->GetGlyphRangesChineseSimplifiedCommon());
    builder.AddRanges(io.Fonts->GetGlyphRangesChineseFull());  // 添加完整中文字符集
    builder.BuildRanges(&ranges);
    
    // 基础字体大小（根据 DPI 缩放）
    float fontSize = 16.0f * dpiScale;
    
    // 尝试加载微软雅黑
    const char* fontPaths[] = {
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\dengxian.ttf",  // 等线字体
    };
    
    ImFont* font = nullptr;
    for (const char* path : fontPaths) {
        font = io.Fonts->AddFontFromFileTTF(path, fontSize, &config, ranges.Data);
        if (font) {
            LOG_INFO(std::string("Font loaded: ") + path);
            break;
        }
    }
    
    if (!font) {
        LOG_INFO("Failed to load Chinese font, using default");
        // 默认字体不支持中文，需要使用内置的紧凑字符集
        io.Fonts->AddFontDefault();
    }
    
    io.Fonts->Build();
    
    // 缩放 ImGui 样式
    ImGuiStyle& style = ImGui::GetStyle();
    style.ScaleAllSizes(dpiScale);
    
    ApplyLightTheme();
    
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
    // 释放纹理缓存
    for (auto& [path, srv] : s_textureCache) {
        if (srv) srv->Release();
    }
    s_textureCache.clear();
    
    if (s_gdiplusToken) {
        Gdiplus::GdiplusShutdown(s_gdiplusToken);
        s_gdiplusToken = 0;
    }
    
    ImGui::SetCurrentContext(static_cast<ImGuiContext*>(m_imguiContext));
    ImGui_ImplDX11_Shutdown();
    ImGui_ImplWin32_Shutdown();
    ImGui::DestroyContext(static_cast<ImGuiContext*>(m_imguiContext));
    
    if (m_rtv) m_rtv->Release();
    if (m_swapChain) m_swapChain->Release();
    if (m_context) m_context->Release();
    if (m_device) m_device->Release();
#endif
}

void D3D11Renderer::NewFrame()
{
#ifdef _WIN32
    ImGui::SetCurrentContext(static_cast<ImGuiContext*>(m_imguiContext));
    ImGui_ImplDX11_NewFrame();
    ImGui_ImplWin32_NewFrame();
    ImGui::NewFrame();
#endif
}

void D3D11Renderer::Render()
{
#ifdef _WIN32
    ImGui::SetCurrentContext(static_cast<ImGuiContext*>(m_imguiContext));
    ImGui::Render();
    
    float clear_color[4] = { 1.0f, 1.0f, 1.0f, 1.0f };
    m_context->OMSetRenderTargets(1, &m_rtv, nullptr);
    m_context->ClearRenderTargetView(m_rtv, clear_color);
    
    ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());
    m_swapChain->Present(1, 0);
#endif
}

void D3D11Renderer::Resize(int width, int height)
{
#ifdef _WIN32
    if (width <= 0 || height <= 0) return;
    
    if (m_rtv) {
        m_rtv->Release();
        m_rtv = nullptr;
    }
    
    HRESULT hr = m_swapChain->ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, 0);
    if (FAILED(hr)) return;
    
    ID3D11Texture2D* pBackBuffer = nullptr;
    hr = m_swapChain->GetBuffer(0, IID_PPV_ARGS(&pBackBuffer));
    if (FAILED(hr) || !pBackBuffer) return;
    
    hr = m_device->CreateRenderTargetView(pBackBuffer, nullptr, &m_rtv);
    pBackBuffer->Release();
    
    if (FAILED(hr)) {
        m_rtv = nullptr;
    }
#endif
}

void D3D11Renderer::UpdateDpiScale(float newScale)
{
#ifdef _WIN32
    if (newScale == m_dpiScale) return;
    
    float oldScale = m_dpiScale;
    m_dpiScale = newScale;
    
    ImGui::SetCurrentContext(static_cast<ImGuiContext*>(m_imguiContext));
    ImGuiIO& io = ImGui::GetIO();
    
    // 清除旧字体
    io.Fonts->Clear();
    
    // 字体配置
    ImFontConfig config;
    config.OversampleH = 3;
    config.OversampleV = 1;
    config.PixelSnapH = true;
    
    // 设置中文字符范围
    ImVector<ImWchar> ranges;
    ImFontGlyphRangesBuilder builder;
    builder.AddRanges(io.Fonts->GetGlyphRangesChineseSimplifiedCommon());
    builder.AddRanges(io.Fonts->GetGlyphRangesChineseFull());
    builder.BuildRanges(&ranges);
    
    // 基础字体大小（根据新的 DPI 缩放）
    float fontSize = 16.0f * newScale;
    
    // 尝试加载微软雅黑
    const char* fontPaths[] = {
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\dengxian.ttf",
    };
    
    ImFont* font = nullptr;
    for (const char* path : fontPaths) {
        font = io.Fonts->AddFontFromFileTTF(path, fontSize, &config, ranges.Data);
        if (font) break;
    }
    
    if (!font) {
        io.Fonts->AddFontDefault();
    }
    
    io.Fonts->Build();
    
    // 重新缩放 ImGui 样式（先恢复再应用新缩放）
    ImGuiStyle& style = ImGui::GetStyle();
    style.ScaleAllSizes(newScale / oldScale);
    
    // 更新字体纹理
    ImGui_ImplDX11_InvalidateDeviceObjects();
    ImGui_ImplDX11_CreateDeviceObjects();
#endif
}

ImTextureID D3D11Renderer::LoadTexture(const std::wstring& path) {
#ifdef _WIN32
    int w, h;
    ID3D11ShaderResourceView* srv = LoadTextureFromFile(m_device, path, &w, &h);
    return (ImTextureID)srv;
#else
    return ImTextureID(0);
#endif
}

void D3D11Renderer::UnloadTexture(ImTextureID texture) {
#ifdef _WIN32
    ID3D11ShaderResourceView* srv = (ID3D11ShaderResourceView*)texture;
    // 从缓存中移除并释放纹理
    for (auto it = s_textureCache.begin(); it != s_textureCache.end(); ++it) {
        if (it->second == srv) {
            if (it->second) it->second->Release();
            s_textureCache.erase(it);
            break;
        }
    }
#endif
}

void D3D11Renderer::UnloadTextureByPath(const std::wstring& path) {
#ifdef _WIN32
    auto it = s_textureCache.find(path);
    if (it != s_textureCache.end()) {
        if (it->second) it->second->Release();
        s_textureCache.erase(it);
    }
#endif
}

int D3D11Renderer::GetTextureWidth(ImTextureID texture) {
#ifdef _WIN32
    if (!texture) return 0;
    ID3D11ShaderResourceView* srv = (ID3D11ShaderResourceView*)texture;
    ID3D11Resource* resource = nullptr;
    srv->GetResource(&resource);
    ID3D11Texture2D* tex = nullptr;
    resource->QueryInterface(&tex);
    D3D11_TEXTURE2D_DESC desc;
    tex->GetDesc(&desc);
    tex->Release();
    resource->Release();
    return desc.Width;
#else
    return 0;
#endif
}

int D3D11Renderer::GetTextureHeight(ImTextureID texture) {
#ifdef _WIN32
    if (!texture) return 0;
    ID3D11ShaderResourceView* srv = (ID3D11ShaderResourceView*)texture;
    ID3D11Resource* resource = nullptr;
    srv->GetResource(&resource);
    ID3D11Texture2D* tex = nullptr;
    resource->QueryInterface(&tex);
    D3D11_TEXTURE2D_DESC desc;
    tex->GetDesc(&desc);
    tex->Release();
    resource->Release();
    return desc.Height;
#else
    return 0;
#endif
}

} // namespace mn