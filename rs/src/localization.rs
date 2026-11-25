// 本地化：语言文件编译期嵌入（resources/lang/zh-CN.json），tr() 取键值。
// v2 决策：仅保留简体中文，不再支持多语言切换（lang_code 参数保留以兼容调用点，
// 无论传何值均加载 zh-CN 资源）。UI 文本接线：菜单 + 空状态 + 对话框。

use std::collections::HashMap;

/// 编译期嵌入的唯一语言资源（简体中文）。
const ZH_CN_JSON: &str = include_str!("../../resources/lang/zh-CN.json");

pub struct Localization {
    strings: HashMap<String, String>,
    lang_code: String,
}

impl Localization {
    pub fn new(lang_code: &str) -> Self {
        let mut loc = Localization {
            strings: HashMap::new(),
            lang_code: lang_code.to_string(),
        };
        loc.load();
        log::info!("Localization created: lang={}", lang_code);
        loc
    }

    fn load(&mut self) {
        // 绿色软件：语言文件编译期嵌入，无需运行时外部文件
        match serde_json::from_str::<HashMap<String, String>>(ZH_CN_JSON) {
            Ok(json) => {
                log::info!("Language file loaded: zh-CN, {} strings", json.len());
                self.strings = json;
            }
            Err(_) => {
                log::error!("Failed to parse language JSON: zh-CN");
            }
        }
    }

    /// 取键对应文案；缺失时回退为键本身（便于发现漏翻译）。
    ///
    /// 注意：返回值同时借用 `self` 与 `key`，因此 `key` 为临时值时需先绑定局部变量
    /// 或直接 `.to_string()`。若要放开该限制，应改为返回 `Cow<'a, str>`，
    /// 但那会波及全部调用点，暂不采用。
    pub fn tr<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    #[allow(dead_code)]
    pub fn language_code(&self) -> &str {
        &self.lang_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_key_returns_key() {
        let loc = Localization::new("zh-CN");
        assert_eq!(loc.tr("SomeKey"), "SomeKey");
    }

    #[test]
    fn test_tr_hit_returns_translation() {
        let loc = Localization::new("zh-CN");
        assert_eq!(loc.tr("LANG_NAME"), "简体中文");
        assert_eq!(loc.tr("Menu_Settings"), "设置…");
    }

    #[test]
    fn test_any_lang_code_loads_zh_cn_resource() {
        // 仅中文：无论传什么 lang_code，都加载 zh-CN 资源；lang_code 保留原值
        assert_eq!(Localization::new("zh-CN").language_code(), "zh-CN");
        assert_eq!(Localization::new("en-US").language_code(), "en-US");
        let loc = Localization::new("xx-unknown");
        assert!(!loc.tr("LANG_NAME").is_empty(), "资源应非空");
        assert_ne!(loc.tr("LANG_NAME"), "LANG_NAME", "应命中 zh-CN 译文而非键");
    }

    #[test]
    fn test_zh_cn_contains_essential_keys() {
        // 守卫：关键文案 key 缺失会显示原始键名（回归防护）
        let loc = Localization::new("zh-CN");
        let essential = [
            "LANG_NAME",
            "Default_Category",
            "Menu_NewItem",
            "Menu_NewC",
            "Menu_DelCurrentC",
            "Menu_RenameCurrentC",
            "Menu_ViewHotKey",
            "Menu_ViewIcon0",
            "Menu_ViewIcon1",
            "Menu_SwitchLight",
            "Menu_SwitchDark",
            "Menu_FollowMouse",
            "Menu_ExportConfig",
            "Menu_ImportConfig",
            "Menu_Settings",
            "Menu_About",
            "Menu_Exit",
            "Title_ConfirmDelete",
            "Tip_DelCate",
            "Btn_Del",
            "Btn_Cancel",
            "Status_SaveFailed",
            "Status_RunFail",
            "Dlg_Settings_Title",
            "Dlg_BG_Label",
            "Dlg_BG_Select",
            "Dlg_BG_Clear",
            "Dlg_BG_Dim",
            "Dlg_Close",
            "Dlg_Log_Level",
            "Dlg_Icon_Size",
            "Dlg_Font_Size",
            "Dlg_Auto_Hide",
            "SI_Log_Min",
            "SI_Log_Std",
            "SI_Log_Dbg",
            "SI_Icon_32",
            "SI_Icon_48",
            "SI_Icon_64",
            "SI_Icon_96",
            "SI_Font_Small",
            "SI_Font_Medium",
            "SI_Font_Large",
            "Status_LogLevelSet",
            "Tip_NeedOneCategory",
            "Status_Pasted",
            "Tip_RunAll",
            "Tip_DelItem",
            "Tray_ShowHide",
            "Tray_AutoStart",
            "STR_On",
            "STR_Off",
            "Tip_AutoStart_On",
            "Tip_AutoStart_Off",
            "Tip_AutoStart_Fail",
            "Tray_LowResident",
            "Tray_Exit",
            "About_Version",
            "About_Desc",
            "Btn_OK",
            "Tip_Search",
            "Tip_NoItems",
            "Tip_DropToAdd",
            "STR_Category",
            "Title_EditItem",
            "Title_ExportConfig",
            "Title_ImportConfig",
            "Title_SelectTarget",
            "Title_SelectIcon",
            "Title_SelectBgImage",
            "STR_FilterConfig",
            "STR_FilterAll",
            "STR_FilterPrograms",
            "STR_FilterIcons",
            "STR_FilterImages",
            "Status_Exported",
            "Status_ExportFailed",
            "Status_ImportFailedInvalid",
            "Status_ImportFailedSave",
            "Status_Imported",
            "Status_HotkeyConflict",
            "Tip_ImportConfirm",
            "Menu_CleanInvalid",
            "Title_CleanInvalid",
            "Tip_CleanInvalidConfirm",
            "Tip_CleanInvalidDone",
            "Tip_CleanInvalidNone",
            "Tip_CleanScanFail",
        ];
        for key in essential {
            assert_ne!(loc.tr(key), key, "zh-CN.json 缺少关键文案 key: {}", key);
        }
    }
}
