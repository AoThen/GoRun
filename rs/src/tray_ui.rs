//! 托盘 UI 动作桥接（P1-3 自 main.rs 拆出）

use std::cell::RefCell;
use std::rc::Rc;

use crate::app_state::AppState;
use crate::localization::Localization;
use crate::tray::TrayMessage;
use crate::MainWindow;
use slint::ComponentHandle;

/// 需要主线程访问 AppState 的托盘动作（经 Arc<Mutex> 队列 + 500ms Timer 消费）。
pub enum TrayUiAction {
    ToggleFollowMouse,
    SetLanguage(String),
}

pub fn start_tray_handler(
    rx: std::sync::mpsc::Receiver<TrayMessage>,
    ui_weak: slint::Weak<MainWindow>,
    tray_ui: std::sync::Arc<std::sync::Mutex<Vec<TrayUiAction>>>,
) {
    std::thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            match msg {
                TrayMessage::Show => {
                    log::info!("Tray: Show window requested");
                    let weak = ui_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            if crate::tray::is_follow_mouse() {
                                crate::geometry::position_at_cursor(&ui);
                            }
                            let _ = ui.show();
                        }
                    })
                    .ok();
                }
                TrayMessage::Hide => {
                    log::info!("Tray: Hide window requested");
                    let weak = ui_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            let _ = ui.hide();
                        }
                    })
                    .ok();
                }
                TrayMessage::Toggle => {
                    // 托盘双击切换显隐（P1-6）；跟随鼠标时显示前定位到光标。
                    log::info!("Tray: Toggle window requested");
                    let weak = ui_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() {
                            if ui.window().is_visible() {
                                let _ = ui.hide();
                            } else {
                                if crate::tray::is_follow_mouse() {
                                    crate::geometry::position_at_cursor(&ui);
                                }
                                let _ = ui.show();
                            }
                        }
                    })
                    .ok();
                }
                TrayMessage::ToggleAutoStart => {
                    // 纯注册表操作，无需 AppState
                    let enabled = crate::tray::toggle_autostart();
                    log::info!("Autostart toggled: enabled={}", enabled);
                }
                TrayMessage::ToggleFollowMouse => {
                    if let Ok(mut q) = tray_ui.lock() {
                        q.push(TrayUiAction::ToggleFollowMouse);
                    }
                }
                TrayMessage::SetLanguage(lang) => {
                    if let Ok(mut q) = tray_ui.lock() {
                        q.push(TrayUiAction::SetLanguage(lang));
                    }
                }
                TrayMessage::Quit => {
                    log::info!("Tray: Quit requested");
                    let _ = slint::quit_event_loop();
                    break;
                }
            }
        }
    });
}

/// 处理需要 AppState 的托盘动作（主线程 Timer 回调内执行）。
pub fn handle_tray_ui_action(
    action: TrayUiAction,
    ui: &MainWindow,
    state: &Rc<RefCell<AppState>>,
) {
    match action {
        TrayUiAction::ToggleFollowMouse => {
            let mut s = state.borrow_mut();
            let cur = s
                .manager
                .config()
                .extra_config
                .get("follow_mouse")
                .map(|v| v == "1")
                .unwrap_or(false);
            let new = !cur;
            s.manager
                .config_mut()
                .extra_config
                .insert("follow_mouse".into(), if new { "1".into() } else { "0".into() });
            s.save_if_modified();
            let lang = s
                .manager
                .config()
                .extra_config
                .get("language")
                .cloned()
                .unwrap_or_else(|| "zh-CN".into());
            crate::tray::set_ui_state(new, &lang);
            log::info!("Follow mouse toggled: {}", new);
        }
        TrayUiAction::SetLanguage(lang) => {
            let mut s = state.borrow_mut();
            s.manager
                .config_mut()
                .extra_config
                .insert("language".into(), lang.clone());
            s.save_if_modified();
            let follow = s
                .manager
                .config()
                .extra_config
                .get("follow_mouse")
                .map(|v| v == "1")
                .unwrap_or(false);
            crate::tray::set_ui_state(follow, &lang);
            log::info!("Language set to {}", lang);
            drop(s);
            // 刷新共享语言表（AppState.loc）+ 菜单栏/空状态/确认/侧栏文本
            let new_loc = {
                let st = state.borrow();
                *st.loc.borrow_mut() = Localization::new(&lang);
                st.loc.clone()
            };
            apply_localized_texts(ui, &new_loc, ui.get_is_dark_theme());
        }
    }
}

/// 写入全部本地化 UI 文本（启动初始化与语言切换共用）。
pub fn apply_localized_texts(ui: &MainWindow, loc: &Rc<RefCell<Localization>>, dark_theme: bool) {
    let menu_theme_key = if dark_theme {
        "Menu_SwitchLight"
    } else {
        "Menu_SwitchDark"
    };
    let lb = loc.borrow();
    ui.set_menu_text_new_item(lb.tr("Menu_NewItem").into());
    ui.set_menu_text_new_category(lb.tr("Menu_NewC").into());
    ui.set_menu_text_toggle_view(lb.tr("Menu_View").into());
    ui.set_menu_text_toggle_theme(lb.tr(menu_theme_key).into());
    ui.set_menu_text_about(lb.tr("Menu_About").into());
    ui.set_menu_text_hotkey(lb.tr("Menu_ViewHotKey").into());
    ui.set_about_version_text(lb.tr("About_Version").into());
    ui.set_about_desc_text(lb.tr("About_Desc").into());
    ui.set_about_ok_text(lb.tr("Btn_OK").into());
    ui.set_sb_new_item_text(lb.tr("Menu_NewItem").into());
    ui.set_sb_search_hint_text(lb.tr("Tip_Search").into());
    ui.set_sb_switch_list_text(lb.tr("SB_SwitchList").into());
    ui.set_sb_switch_icon_text(lb.tr("SB_SwitchIcon").into());
    ui.set_sb_switch_light_text(lb.tr("SB_SwitchLight").into());
    ui.set_sb_switch_dark_text(lb.tr("SB_SwitchDark").into());
    ui.set_empty_text(lb.tr("Tip_NoItems").into());
    ui.set_confirm_confirm_text(lb.tr("Btn_Del").into());
    ui.set_confirm_cancel_text(lb.tr("Btn_Cancel").into());
    ui.set_sidebar_title_text(lb.tr("STR_Category").into());
    ui.set_sidebar_add_category_text(lb.tr("Menu_NewC").into());
    ui.set_input_dialog_confirm_text(lb.tr("Btn_OK").into());
    ui.set_input_dialog_cancel_text(lb.tr("Btn_Cancel").into());
    crate::tray::set_ui_texts(crate::tray::TrayTexts {
        show: lb.tr("Tray_ShowHide").to_string(),
        autostart: lb.tr("Tray_AutoStart").to_string(),
        follow_mouse: lb.tr("Menu_FollowMouse").to_string(),
        exit: lb.tr("Tray_Exit").to_string(),
    });
}
