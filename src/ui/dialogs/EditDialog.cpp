#include "EditDialog.h"
#include "utils/StringUtils.h"
#include <imgui.h>
#include <algorithm>

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
    
    ImGui::OpenPopup(u8"编辑项目");
    
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(400, 0));
    
    if (ImGui::BeginPopupModal(u8"编辑项目", nullptr, ImGuiWindowFlags_NoResize)) {
        char nameBuf[256] = {};
        size_t copyLen = std::min(m_nameBuf.size(), sizeof(nameBuf) - 1);
        memcpy(nameBuf, m_nameBuf.c_str(), copyLen);
        if (ImGui::InputText(u8"名称", nameBuf, sizeof(nameBuf))) {
            m_nameBuf = nameBuf;
        }
        
        char targetBuf[1024] = {};
        copyLen = std::min(m_targetBuf.size(), sizeof(targetBuf) - 1);
        memcpy(targetBuf, m_targetBuf.c_str(), copyLen);
        if (ImGui::InputText(u8"目标", targetBuf, sizeof(targetBuf))) {
            m_targetBuf = targetBuf;
        }
        
        char argsBuf[512] = {};
        copyLen = std::min(m_argsBuf.size(), sizeof(argsBuf) - 1);
        memcpy(argsBuf, m_argsBuf.c_str(), copyLen);
        if (ImGui::InputText(u8"参数", argsBuf, sizeof(argsBuf))) {
            m_argsBuf = argsBuf;
        }
        
        char workDirBuf[1024] = {};
        copyLen = std::min(m_workingDirBuf.size(), sizeof(workDirBuf) - 1);
        memcpy(workDirBuf, m_workingDirBuf.c_str(), copyLen);
        if (ImGui::InputText(u8"工作目录", workDirBuf, sizeof(workDirBuf))) {
            m_workingDirBuf = workDirBuf;
        }
        
        ImGui::Checkbox(u8"以管理员运行", &m_runAsAdmin);
        
        ImGui::Separator();
        
        if (ImGui::Button(u8"保存", ImVec2(120, 0))) {
            SaveToItem();
            if (m_onSave) {
                m_onSave(*m_item);
            }
            Hide();
        }
        ImGui::SameLine();
        if (ImGui::Button(u8"取消", ImVec2(120, 0))) {
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
    m_runAsAdmin = m_item->runAsAdmin;
}

void EditDialog::SaveToItem() {
    if (!m_item) return;
    
    m_item->name = StringUtils::Utf8ToWString(m_nameBuf);
    m_item->target = StringUtils::Utf8ToWString(m_targetBuf);
    m_item->arguments = StringUtils::Utf8ToWString(m_argsBuf);
    m_item->workingDir = StringUtils::Utf8ToWString(m_workingDirBuf);
    m_item->runAsAdmin = m_runAsAdmin;
}

} // namespace mn