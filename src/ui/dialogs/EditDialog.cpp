#include "EditDialog.h"
#include "utils/StringUtils.h"
#include <imgui.h>
#include <algorithm>

#ifdef _WIN32
#include <Windows.h>
#include <ShlObj.h>
#include <Commdlg.h>
#endif

namespace mn {

void EditDialog::Show(Item* item) {
    m_item = item;
    m_visible = true;
    if (item) {
        LoadFromItem();
    }
}

void EditDialog::Hide() {
    m_visible = false;
    m_item = nullptr;
}

bool EditDialog::IsVisible() const {
    return m_visible;
}

void EditDialog::Render() {
    if (!m_visible || !m_item) return;
    
    ImGui::OpenPopup("编辑项目");
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(480, 0));
    
    if (ImGui::BeginPopupModal("编辑项目", nullptr, ImGuiWindowFlags_NoResize)) {
        // 名称
        char nameBuf[256] = {};
        size_t copyLen = std::min(m_nameBuf.size(), sizeof(nameBuf) - 1);
        memcpy(nameBuf, m_nameBuf.c_str(), copyLen);
        if (ImGui::InputText("名称", nameBuf, sizeof(nameBuf))) {
            m_nameBuf = nameBuf;
        }
        
        // 目标路径（带浏览按钮）
        ImGui::BeginGroup();
        char targetBuf[1024] = {};
        copyLen = std::min(m_targetBuf.size(), sizeof(targetBuf) - 1);
        memcpy(targetBuf, m_targetBuf.c_str(), copyLen);
        ImGui::PushItemWidth(-70);
        if (ImGui::InputText("##target", targetBuf, sizeof(targetBuf))) {
            m_targetBuf = targetBuf;
        }
        ImGui::PopItemWidth();
        ImGui::SameLine();
        if (ImGui::Button("浏览...", ImVec2(60, 0))) {
            BrowseTarget();
        }
        ImGui::EndGroup();
        ImGui::SameLine(0, -1);
        ImGui::Text("目标");
        
        // 参数
        char argsBuf[512] = {};
        copyLen = std::min(m_argsBuf.size(), sizeof(argsBuf) - 1);
        memcpy(argsBuf, m_argsBuf.c_str(), copyLen);
        if (ImGui::InputText("参数", argsBuf, sizeof(argsBuf))) {
            m_argsBuf = argsBuf;
        }
        
        // 工作目录
        char workDirBuf[1024] = {};
        copyLen = std::min(m_workingDirBuf.size(), sizeof(workDirBuf) - 1);
        memcpy(workDirBuf, m_workingDirBuf.c_str(), copyLen);
        if (ImGui::InputText("工作目录", workDirBuf, sizeof(workDirBuf))) {
            m_workingDirBuf = workDirBuf;
        }
        
        // 关键词
        char keywordsBuf[512] = {};
        copyLen = std::min(m_keywordsBuf.size(), sizeof(keywordsBuf) - 1);
        memcpy(keywordsBuf, m_keywordsBuf.c_str(), copyLen);
        if (ImGui::InputTextWithHint("关键词", "多个关键词用逗号分隔", keywordsBuf, sizeof(keywordsBuf))) {
            m_keywordsBuf = keywordsBuf;
        }
        if (ImGui::IsItemHovered()) {
            ImGui::SetTooltip("关键词可用于搜索匹配，多个关键词用英文逗号分隔");
        }
        
        // 备注
        char remarkBuf[512] = {};
        copyLen = std::min(m_remarkBuf.size(), sizeof(remarkBuf) - 1);
        memcpy(remarkBuf, m_remarkBuf.c_str(), copyLen);
        if (ImGui::InputText("备注", remarkBuf, sizeof(remarkBuf))) {
            m_remarkBuf = remarkBuf;
        }
        
        // 图标路径
        ImGui::BeginGroup();
        char iconPathBuf[1024] = {};
        copyLen = std::min(m_iconPathBuf.size(), sizeof(iconPathBuf) - 1);
        memcpy(iconPathBuf, m_iconPathBuf.c_str(), copyLen);
        ImGui::PushItemWidth(-70);
        if (ImGui::InputText("##iconPath", iconPathBuf, sizeof(iconPathBuf))) {
            m_iconPathBuf = iconPathBuf;
        }
        ImGui::PopItemWidth();
        ImGui::SameLine();
        if (ImGui::Button("选择...", ImVec2(60, 0))) {
            BrowseIcon();
        }
        ImGui::EndGroup();
        ImGui::SameLine(0, -1);
        ImGui::Text("图标路径");
        
        // 图标索引
        ImGui::PushItemWidth(80);
        ImGui::InputInt("图标索引", &m_iconIndex);
        if (m_iconIndex < 0) m_iconIndex = 0;
        ImGui::PopItemWidth();
        
        // 管理员运行
        ImGui::Checkbox("以管理员运行", &m_runAsAdmin);
        
        ImGui::Separator();
        
        if (ImGui::Button("保存", ImVec2(120, 0))) {
            SaveToItem();
            if (m_onSave) {
                m_onSave(*m_item);
            }
            Hide();
        }
        ImGui::SameLine();
        if (ImGui::Button("取消", ImVec2(120, 0))) {
            Hide();
        }
        
        ImGui::EndPopup();
    }
}

void EditDialog::OnSave(std::function<void(const Item&)> callback) {
    m_onSave = callback;
}

void EditDialog::LoadFromItem() {
    if (!m_item) return;
    
    m_nameBuf = StringUtils::WStringToUtf8(m_item->name);
    m_targetBuf = StringUtils::WStringToUtf8(m_item->target);
    m_argsBuf = StringUtils::WStringToUtf8(m_item->arguments);
    m_workingDirBuf = StringUtils::WStringToUtf8(m_item->workingDir);
    m_keywordsBuf = StringUtils::WStringToUtf8(m_item->keywords);
    m_remarkBuf = StringUtils::WStringToUtf8(m_item->remark);
    m_iconPathBuf = StringUtils::WStringToUtf8(m_item->iconPath);
    m_iconIndex = m_item->iconIndex;
    m_runAsAdmin = m_item->runAsAdmin;
}

void EditDialog::SaveToItem() {
    if (!m_item) return;
    
    m_item->name = StringUtils::Utf8ToWString(m_nameBuf);
    m_item->target = StringUtils::Utf8ToWString(m_targetBuf);
    m_item->arguments = StringUtils::Utf8ToWString(m_argsBuf);
    m_item->workingDir = StringUtils::Utf8ToWString(m_workingDirBuf);
    m_item->keywords = StringUtils::Utf8ToWString(m_keywordsBuf);
    m_item->remark = StringUtils::Utf8ToWString(m_remarkBuf);
    m_item->iconPath = StringUtils::Utf8ToWString(m_iconPathBuf);
    m_item->iconIndex = m_iconIndex;
    m_item->runAsAdmin = m_runAsAdmin;
}

void EditDialog::BrowseTarget() {
#ifdef _WIN32
    wchar_t filename[MAX_PATH] = {0};
    
    OPENFILENAMEW ofn = {};
    ofn.lStructSize = sizeof(OPENFILENAMEW);
    ofn.hwndOwner = nullptr;
    ofn.lpstrFilter = L"可执行文件 (*.exe)\0*.exe\0"
                      L"快捷方式 (*.lnk)\0*.lnk\0"
                      L"所有文件 (*.*)\0*.*\0";
    ofn.lpstrFile = filename;
    ofn.nMaxFile = MAX_PATH;
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    
    if (GetOpenFileNameW(&ofn)) {
        m_targetBuf = StringUtils::WStringToUtf8(filename);
        
        // 如果工作目录为空，自动填充
        if (m_workingDirBuf.empty() && filename[0] != L'\0') {
            std::wstring path(filename);
            size_t lastSlash = path.find_last_of(L"\\");
            if (lastSlash != std::wstring::npos) {
                m_workingDirBuf = StringUtils::WStringToUtf8(path.substr(0, lastSlash));
            }
        }
        
        // 如果名称为空，自动填充文件名
        if (m_nameBuf.empty() && filename[0] != L'\0') {
            std::wstring path(filename);
            size_t lastSlash = path.find_last_of(L"\\");
            size_t lastDot = path.find_last_of(L".");
            if (lastSlash != std::wstring::npos && lastDot != std::wstring::npos && lastDot > lastSlash) {
                m_nameBuf = StringUtils::WStringToUtf8(path.substr(lastSlash + 1, lastDot - lastSlash - 1));
            }
        }
    }
#endif
}

void EditDialog::BrowseIcon() {
#ifdef _WIN32
    wchar_t filename[MAX_PATH] = {0};
    
    OPENFILENAMEW ofn = {};
    ofn.lStructSize = sizeof(OPENFILENAMEW);
    ofn.hwndOwner = nullptr;
    ofn.lpstrFilter = L"图标文件 (*.ico;*.exe;*.dll)\0*.ico;*.exe;*.dll\0"
                      L"图片文件 (*.png;*.jpg;*.bmp)\0*.png;*.jpg;*.bmp\0"
                      L"所有文件 (*.*)\0*.*\0";
    ofn.lpstrFile = filename;
    ofn.nMaxFile = MAX_PATH;
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    
    if (GetOpenFileNameW(&ofn)) {
        m_iconPathBuf = StringUtils::WStringToUtf8(filename);
    }
#endif
}

} // namespace mn