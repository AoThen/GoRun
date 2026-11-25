use std::sync::mpsc::{channel, Sender};
use std::thread;

const HOTKEY_ID: u32 = 0xC001;

/// Windows 自动重置事件（P3-2）：用于即时唤醒热键 pump 线程的 kill 信号。
/// 原实现用 mpsc 通道 + 500ms MsgWait 轮询，kill 后 join 最长阻塞 500ms（UI 线程可见顿挫）；
/// 改用事件句柄接入 MsgWaitForMultipleObjects 的句柄数组，SetEvent 后等待立即返回，
/// join 阻塞降到毫秒级。RAII 封装 CloseHandle；Drop 在 join 之后执行（见 kill_pump_thread），
/// 避免 MsgWait 等待期间句柄被提前关闭。
#[cfg(windows)]
struct EventHandle(windows::Win32::Foundation::HANDLE);

/// 可跨线程的事件句柄值（P3-2）：windows 0.62 起 HANDLE（裸指针内含）不实现 Send，
/// 而 pump 线程需要该句柄在 MsgWaitForMultipleObjects 中等待。内核事件句柄跨线程等待/
/// 置位（SetEvent/MsgWait）是线程安全的；负责关闭的 RAII（EventHandle）保留在拥有线程，
/// join 完成前不关闭，故无跨线程 use-after-free。这里以 usize 承载句柄整数值（指针宽），
/// usize 原生 Send/Sync，避免对含裸指针的 HANDLE 做手工 impl。
#[cfg(windows)]
#[derive(Clone, Copy)]
struct SendHandle(usize);

#[cfg(windows)]
impl EventHandle {
    fn new() -> Option<Self> {
        use windows::Win32::System::Threading::CreateEventW;
        // 自动重置（bManualReset=false）、初始未置位；失败（极少，内核资源不足）时
        // 由调用方按注册失败处理（不启动 pump，避免无线程泄漏）。
        unsafe { CreateEventW(None, false, false, None).ok().map(EventHandle) }
    }

    fn handle(&self) -> windows::Win32::Foundation::HANDLE {
        self.0
    }

    fn signal(&self) {
        use windows::Win32::System::Threading::SetEvent;
        unsafe {
            let _ = SetEvent(self.0);
        }
    }
}

#[cfg(windows)]
impl Drop for EventHandle {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// 平台无关的 kill 信号（P3-2）：避免在 `let` 上使用 `#[cfg]`（E0658 实验性）。
/// Windows 持事件句柄（SetEvent 即时唤醒 pump）；非 Windows 无长驻 pump，持占位变体。
/// 闭包内经 cfg 分支解构：Windows 匹配 Event（穷尽），非 Windows 分支不进循环。
enum KillSignal {
    #[cfg(windows)]
    Event(SendHandle),
    #[cfg(not(windows))]
    Dummy,
}

// SendHandle 以 usize 承载句柄，原生 Send/Sync，无需对 KillSignal 做手工 impl。

/// 创建 Windows kill 事件并存入 `slot`（P3-2，M-1/M-2 共用）。创建失败返回
/// None（调用方按注册失败处理，不启动 pump，避免线程/热键泄漏）。
/// 仅 Windows：非 Windows 无长驻 pump，由各管理器的 `setup_kill_signal`
/// 方法经 cfg 分支返回 Dummy。
#[cfg(windows)]
fn setup_kill_signal(slot: &mut Option<EventHandle>) -> Option<KillSignal> {
    let kill_event = EventHandle::new()?;
    let h = kill_event.handle();
    *slot = Some(kill_event);
    Some(KillSignal::Event(SendHandle(h.0 as usize)))
}

/// 平台无关的 kill 守卫（P3-2）：`kill_pump_thread` 取走信号后持有句柄，
/// 待 `join` 完成再 Drop（关闭事件句柄），避免 pump 仍在 MsgWait 时句柄被提前关闭。
/// 同样为避免 `#[cfg]` 作用于 `let`，用方法分派 + 平台中性枚举持有。
enum KillGuard {
    // 句柄"持有至 join 后 Drop 再关闭"，字段从不被读取（hold-until-drop 语义）
    #[cfg(windows)]
    #[allow(dead_code)]
    Event(Option<EventHandle>),
    #[cfg(not(windows))]
    Dummy,
}

#[derive(Debug, Clone)]
pub struct Hotkey {
    pub modifiers: u32,
    pub vk: u32,
}

/// US 布局标点键 → Windows 虚拟键码（VK_OEM 段）。
/// 同一物理键的上档字符（如 `:` 与 `;`）共享同一 VK，按主字符（下档）登记。
fn punct_vk(ch: char) -> Option<u32> {
    let vk = match ch {
        ';' | ':' => 0xBA, // VK_OEM_1
        '=' | '+' => 0xBB,
        ',' | '<' => 0xBC,
        '-' | '_' => 0xBD,
        '.' | '>' => 0xBE,
        '/' | '?' => 0xBF, // VK_OEM_2
        '`' | '~' => 0xC0, // VK_OEM_3
        '[' | '{' => 0xDB,
        '\\' | '|' => 0xDC,
        ']' | '}' => 0xDD,
        '\'' | '"' => 0xDE, // VK_OEM_7
        _ => return None,
    };
    Some(vk)
}

pub fn parse_hotkey_string(s: &str) -> Option<Hotkey> {
    let mut modifiers = 0u32;
    let mut vk = 0u32;
    let parts: Vec<&str> = s.split('+').collect();

    for (idx, part) in parts.iter().enumerate() {
        let is_last = idx == parts.len() - 1;
        let trimmed = part.trim();
        match trimmed.to_lowercase().as_str() {
            "ctrl" => modifiers |= 0x0002,  // MOD_CONTROL
            "alt" => modifiers |= 0x0001,   // MOD_ALT
            "shift" => modifiers |= 0x0004, // MOD_SHIFT
            "win" => modifiers |= 0x0008,   // MOD_WIN
            last => {
                if !is_last {
                    return None;
                }
                vk = match last.to_uppercase().as_str() {
                    c if c.len() == 1 => {
                        // c.len()==1（字节）⇒ 单字节 UTF-8 ⇒ 恰好 1 个 char，next() 必非空
                        let ch = c.chars().next().expect("单字节字符串必有字符");
                        if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
                            ch as u32
                        } else {
                            punct_vk(ch)?
                        }
                    }
                    "SPACE" => 0x20,
                    "TAB" => 0x09,
                    "ESCAPE" | "ESC" => 0x1B,
                    "ENTER" => 0x0D,
                    "BACKSPACE" => 0x08,
                    "DELETE" | "DEL" => 0x2E,
                    "INSERT" | "INS" => 0x2D,
                    "HOME" => 0x24,
                    "END" => 0x23,
                    "PAGEUP" => 0x21,
                    "PAGEDOWN" => 0x22,
                    "PRINTSCREEN" => 0x2C,
                    "PAUSE" => 0x13,
                    "NUMLOCK" => 0x90,
                    "CAPSLOCK" => 0x14,
                    "SCROLLLOCK" => 0x91,
                    "F1" => 0x70,
                    "F2" => 0x71,
                    "F3" => 0x72,
                    "F4" => 0x73,
                    "F5" => 0x74,
                    "F6" => 0x75,
                    "F7" => 0x76,
                    "F8" => 0x77,
                    "F9" => 0x78,
                    "F10" => 0x79,
                    "F11" => 0x7A,
                    "F12" => 0x7B,
                    _ => return None,
                };
            }
        }
    }

    if vk == 0 {
        None
    } else {
        Some(Hotkey { modifiers, vk })
    }
}

pub struct HotkeyManager {
    id: u32,
    registered: bool,
    pump_active: bool,
    thread_handle: Option<thread::JoinHandle<()>>,
    /// Windows：kill 事件句柄（SetEvent 即时唤醒 pump）；非 Windows：mpsc 通道占位。
    #[cfg(windows)]
    kill_event: Option<EventHandle>,
    #[cfg(not(windows))]
    kill_sender: Option<Sender<()>>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            id: HOTKEY_ID,
            registered: false,
            pump_active: false,
            thread_handle: None,
            #[cfg(windows)]
            kill_event: None,
            #[cfg(not(windows))]
            kill_sender: None,
        }
    }

    /// 注册全局热键并启动消息泵线程。触发时向 `sender` 发送 `()`。
    ///
    /// M-1 重入防御：入口先 `unregister()`（kill 旧 pump + 注销旧热键），
    /// 避免重复调用覆盖旧 `thread_handle`/kill 信号导致旧线程永不退出
    /// （线程 + `RegisterHotKey` 双重泄漏，且会使新键注册失败）。
    ///
    /// M-2：kill 信号为事件句柄（与 `ItemHotkeyManager` 同构），`SetEvent`
    /// 后 pump 的 `MsgWaitForMultipleObjects` 立即返回，`join` 阻塞降到毫秒级
    /// （旧实现 500ms 轮询 kill 通道，改键/退出时 UI 线程可感知顿挫）。
    pub fn register_with_notifier(&mut self, hotkey_str: &str, sender: Sender<()>) -> bool {
        let hk = match parse_hotkey_string(hotkey_str) {
            Some(h) => h,
            None => {
                log::error!(
                    "HotkeyManager: failed to parse hotkey string: {}",
                    hotkey_str
                );
                return false;
            }
        };

        // 重入防御：先注销旧的（幂等；未注册时为空操作）
        self.unregister();

        // 事件创建失败视为注册失败（不启动 pump，避免线程/热键泄漏）
        let kill_signal = match self.setup_kill_signal() {
            Some(k) => k,
            None => {
                log::error!("HotkeyManager: failed to create kill event, aborting register");
                return false;
            }
        };
        let (result_tx, result_rx) = channel::<bool>();
        let hotkey_id = self.id;

        let handle = thread::Builder::new()
            .name("hotkey-pump".to_string())
            .spawn(move || {
                #[cfg(windows)]
                unsafe {
                    use windows::Win32::Foundation::TRUE;
                    use windows::Win32::UI::Input::KeyboardAndMouse::{
                        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
                    };
                    use windows::Win32::UI::WindowsAndMessaging::{
                        DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, MSG,
                        PM_NOREMOVE, PM_REMOVE, QS_ALLINPUT, WM_HOTKEY,
                    };

                    // 从平台无关 KillSignal 解构出 kill 事件句柄（Windows 仅有
                    // Event 变体，match 穷尽；非 Windows 分支不进此块）
                    let kill_h = match kill_signal {
                        KillSignal::Event(SendHandle(v)) => {
                            windows::Win32::Foundation::HANDLE(v as *mut core::ffi::c_void)
                        }
                    };
                    let mut msg: MSG = std::mem::zeroed();
                    // 确保注册线程拥有消息队列（P0 防御：RegisterHotKey 要求线程有消息队列）
                    let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
                    let ok = RegisterHotKey(
                        None,
                        hotkey_id as i32,
                        HOT_KEY_MODIFIERS(hk.modifiers),
                        hk.vk,
                    )
                    .is_ok();
                    let _ = result_tx.send(ok);
                    if !ok {
                        return;
                    }

                    loop {
                        // kill 事件句柄接入 MsgWait 句柄数组：SetEvent 后本等待
                        // 立即返回（WAIT_OBJECT_0=0），join 不再受轮询周期影响。
                        let waited =
                            MsgWaitForMultipleObjects(Some(&[kill_h]), false, 500, QS_ALLINPUT);
                        match waited.0 {
                            0 => {
                                log::info!(
                                    "HotkeyManager: message pump thread received kill signal"
                                );
                                break;
                            }
                            258 => {} // WAIT_TIMEOUT：空闲，继续阻塞等待
                            0xFFFF_FFFF => {
                                log::error!("HotkeyManager: MsgWaitForMultipleObjects failed");
                                break;
                            }
                            _ => {
                                // 有消息（WAIT_OBJECT_0 等）：排空消息队列并分发
                                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) == TRUE {
                                    if msg.message == WM_HOTKEY
                                        && msg.wParam.0 as u32 == hotkey_id
                                    {
                                        let _ = sender.send(());
                                    }
                                    let _ = DispatchMessageW(&msg);
                                }
                            }
                        }
                    }

                    let _ = UnregisterHotKey(None, hotkey_id as i32);
                }

                #[cfg(not(windows))]
                {
                    let _ = &sender;
                    let _ = &kill_signal;
                    let _ = &hk;
                    let _ = hotkey_id;
                    // 必须回送结果，否则主线程 result_rx.recv() 永久阻塞
                    let _ = result_tx.send(false);
                    log::warn!(
                        "HotkeyManager: hotkey notifier thread not supported on this platform"
                    );
                }
            });

        match handle {
            Ok(h) => {
                self.thread_handle = Some(h);
                // P3-10：recv_timeout 兜底，pump 线程异常未回送结果时避免主线程永久挂起
                let ok = result_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap_or(false);
                self.registered = ok;
                self.pump_active = ok;
                if ok {
                    log::info!(
                        "HotkeyManager: registered hotkey '{}' (id=0x{:X})",
                        hotkey_str,
                        self.id
                    );
                } else {
                    log::error!("HotkeyManager: RegisterHotKey failed for '{}'", hotkey_str);
                }
                ok
            }
            Err(e) => {
                log::error!("HotkeyManager: failed to start message pump thread: {}", e);
                false
            }
        }
    }

    pub fn unregister(&mut self) -> bool {
        #[cfg(windows)]
        let had_pump = self.pump_active;
        self.kill_pump_thread();
        self.pump_active = false;

        if !self.registered {
            return true;
        }

        #[cfg(windows)]
        unsafe {
            if had_pump {
                self.registered = false;
                return true;
            }
            use windows::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;

            let result = UnregisterHotKey(None, self.id as i32);
            if result.is_ok() {
                log::info!("HotkeyManager: unregistered hotkey (id=0x{:X})", self.id);
                self.registered = false;
                true
            } else {
                log::error!(
                    "HotkeyManager: UnregisterHotKey failed (id=0x{:X}): {:?}",
                    self.id,
                    result.err()
                );
                false
            }
        }

        #[cfg(not(windows))]
        {
            self.registered = false;
            true
        }
    }

    fn kill_pump_thread(&mut self) {
        // 取走并置位 kill 信号；Windows 事件句柄随 KillGuard 持有，join 后再
        // Drop 关闭（避免 pump 线程仍在 MsgWait 时句柄被提前关闭）。
        let guard = self.take_kill_guard();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
            log::info!("HotkeyManager: message pump thread joined");
        }
        drop(guard);
    }

    /// 取走 kill 信号并置位（M-2）。Windows：SetEvent 使 pump 的 MsgWait 立即
    /// 返回；事件句柄放入 KillGuard 供 join 后关闭。非 Windows：发送通道信号。
    fn take_kill_guard(&mut self) -> KillGuard {
        #[cfg(windows)]
        {
            let ev = self.kill_event.take();
            if let Some(e) = &ev {
                e.signal();
            }
            KillGuard::Event(ev)
        }
        #[cfg(not(windows))]
        {
            if let Some(kill_tx) = self.kill_sender.take() {
                let _ = kill_tx.send(());
            }
            KillGuard::Dummy
        }
    }

    /// 创建平台相关 kill 信号并存入自身（M-2，与 `ItemHotkeyManager` 同构）。
    /// Windows：创建自动重置事件（复用共享 `setup_kill_signal`），失败返回
    /// None（调用方按注册失败处理，不启动 pump）。非 Windows：无长驻 pump，
    /// 返回 Dummy 占位。
    fn setup_kill_signal(&mut self) -> Option<KillSignal> {
        #[cfg(windows)]
        {
            setup_kill_signal(&mut self.kill_event)
        }
        #[cfg(not(windows))]
        {
            let _ = &mut self.kill_sender;
            Some(KillSignal::Dummy)
        }
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.unregister();
    }
}

/// 条目级全局热键（F8/4.1）：一个 pump 线程注册多个条目热键（各配唯一 id），
/// 触发时按 `WM_HOTKEY.wParam`（id）查回 item_id，经 `Sender<String>` 分发给主线程。
pub struct ItemHotkeyManager {
    registered: bool,
    pump_active: bool,
    thread_handle: Option<thread::JoinHandle<()>>,
    /// 当前实际注册成功的条目热键集合（P3 差量跳过）：`register_all` 前先与
    /// 新集合做归一化比较，未变且 pump 存活则跳过"杀线程→全量重注册"。
    /// 仅含成功项；unregister/注册失败路径必须清空，防止误判"未变"静默失效。
    current: Vec<(String, String)>,
    /// Windows：kill 事件句柄（SetEvent 即时唤醒 pump）；非 Windows：mpsc 通道（无长驻 pump）。
    #[cfg(windows)]
    kill_event: Option<EventHandle>,
    #[cfg(not(windows))]
    kill_sender: Option<Sender<()>>,
}

/// 注册结果：失败热键串（去重，供调用方提示）+ 实际注册成功的 (item_id, 热键串)
/// 集合（供差量比较维护 `current`；失败项/超时/线程异常时不含对应项）。
struct RegisterOutcome {
    failed: Vec<String>,
    ok: Vec<(String, String)>,
}

/// 热键归一化：以 `parse_hotkey_string` 的 (modifiers, vk) 为等价判据
///（"ctrl+m" 与 "Ctrl+M" 等价）；解析失败的串按 trim 后原串参与比较
///（两端都会注册失败，行为一致）。
fn normalized_hotkey(hk: &str) -> String {
    match parse_hotkey_string(hk.trim()) {
        Some(h) => format!("{:x}:{:x}", h.modifiers, h.vk),
        None => format!("raw:{}", hk.trim()),
    }
}

/// 集合级归一化（排序去重，顺序无关比较）。item_id 必须参与比较：
/// 热键串不变但归属条目变化（清除 A 改设 B、两条目互换热键）时，
/// pump 线程内旧 id_map 仍指向旧条目，跳过重注册会触发错误条目（P1-1）。
fn normalized_hotkey_set(hotkeys: &[(String, String)]) -> Vec<String> {
    let mut v: Vec<String> = hotkeys
        .iter()
        .map(|(id, hk)| format!("{}\u{1}{}", id, normalized_hotkey(hk)))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// 失败列表构造：按输入顺序去重的热键串。
fn dedup_hotkey_strings(hotkeys: &[(String, String)]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for (_, hk) in hotkeys {
        if !v.contains(hk) {
            v.push(hk.clone());
        }
    }
    v
}

impl ItemHotkeyManager {
    pub fn new() -> Self {
        ItemHotkeyManager {
            registered: false,
            pump_active: false,
            thread_handle: None,
            current: Vec::new(),
            #[cfg(windows)]
            kill_event: None,
            #[cfg(not(windows))]
            kill_sender: None,
        }
    }

    /// 创建平台相关的 kill 信号并存入自身（P3-2），Windows 复用共享
    /// `setup_kill_signal` 自由函数。事件创建失败返回 None（调用方按注册
    /// 失败处理，不启动 pump）。非 Windows：无长驻 pump，返回 Dummy 占位。
    fn setup_kill_signal(&mut self) -> Option<KillSignal> {
        #[cfg(windows)]
        {
            setup_kill_signal(&mut self.kill_event)
        }
        #[cfg(not(windows))]
        {
            let _ = &mut self.kill_sender;
            Some(KillSignal::Dummy)
        }
    }

    /// 注册全部条目热键（`(item_id, hotkey_str)`）。空列表视为成功（无条目热键）。
    /// 触发时向 `sender` 发送被按下的条目 `item_id`。单个失败（解析/注册错误）不阻断其他项。
    /// 返回注册失败的热键字符串列表（按输入顺序去重；空 = 全部成功）。
    /// 与全局热键/其他程序冲突的项同样计入失败（RegisterHotKey 系统层失败）。
    /// 非 Windows 平台不支持条目热键，返回空列表（不视为失败，避免误报）。
    /// P3 差量：集合未变（归一化比较）且 pump 线程存活时跳过注销重建，
    /// 消除"保存未改热键的条目也全量重注册"的开销与热键短暂失效窗口。
    /// 差量跳过判定（纯判定，便于跨平台测试）：pump 存活且新集合与当前
    /// 注册集合归一化相等时跳过重建。
    /// 已知微窗口（P3，接受）：pump 因 MsgWait 失败自行退出与本次调用并发时，
    /// is_finished() 可能仍为 false 而误跳过——发生条件极苛刻，且下次任意
    /// 集合变化会自愈；旧全量路径无此窗口但每次保存都杀线程重建。
    fn can_skip_reregister(&self, hotkeys: &[(String, String)]) -> bool {
        self.pump_active
            && self
                .thread_handle
                .as_ref()
                .is_some_and(|h| !h.is_finished())
            && normalized_hotkey_set(hotkeys) == normalized_hotkey_set(&self.current)
    }

    pub fn register_all(
        &mut self,
        hotkeys: &[(String, String)],
        sender: Sender<String>,
    ) -> Vec<String> {
        if self.can_skip_reregister(hotkeys) {
            return Vec::new();
        }
        self.unregister();
        if hotkeys.is_empty() {
            self.registered = true;
            return Vec::new();
        }

        // P3-2：kill 信号经平台无关 KillSignal 枚举传递（避免 `#[cfg]` 作用于 `let`
        // 表达式——E0658 实验性特性；用方法分派 + 闭包内 match 解构，各平台稳定编译）。
        // Windows 事件创建失败视为注册失败（不启动 pump，避免线程/热键泄漏）。
        let kill_signal = match self.setup_kill_signal() {
            Some(k) => k,
            None => {
                log::error!("ItemHotkeyManager: failed to create kill event, aborting register");
                self.current.clear();
                return dedup_hotkey_strings(hotkeys);
            }
        };
        let (result_tx, result_rx) = channel::<RegisterOutcome>();
        let hotkeys_owned: Vec<(String, String)> = hotkeys.to_vec();

        let handle = thread::Builder::new()
            .name("item-hotkey-pump".to_string())
            .spawn(move || {
                #[cfg(windows)]
                unsafe {
                    use std::collections::HashMap;
                    use windows::Win32::Foundation::TRUE;
                    use windows::Win32::UI::Input::KeyboardAndMouse::{
                        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
                    };
                    use windows::Win32::UI::WindowsAndMessaging::{
                        DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, QS_ALLINPUT,
                        MSG, PM_NOREMOVE, PM_REMOVE, WM_HOTKEY,
                    };

                    let mut msg: MSG = std::mem::zeroed();
                    // 从平台无关 KillSignal 解构出 kill 事件句柄（Windows 仅有 Event 变体，
                    // match 穷尽；非 Windows 分支不进此块，见 cfg(not(windows)) 分支）。
                    let kill_h = match kill_signal {
                        KillSignal::Event(SendHandle(v)) => {
                            windows::Win32::Foundation::HANDLE(v as *mut core::ffi::c_void)
                        }
                    };
                    // 确保注册线程拥有消息队列（P0 防御：RegisterHotKey 要求线程有消息队列）
                    let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);

                    // id 从 0xC100 起，避免与全局热键 HOTKEY_ID(0xC001) 冲突
                    let mut id_map: HashMap<u32, String> = HashMap::new();
                    let mut failed: Vec<String> = Vec::new();
                    let mut ok: Vec<(String, String)> = Vec::new();
                    for (i, (item_id, hk_str)) in hotkeys_owned.iter().enumerate() {
                        let hkid = 0xC100 + i as u32;
                        match parse_hotkey_string(hk_str) {
                            Some(hk) => {
                                let r = RegisterHotKey(
                                    None,
                                    hkid as i32,
                                    HOT_KEY_MODIFIERS(hk.modifiers),
                                    hk.vk,
                                );
                                if r.is_ok() {
                                    id_map.insert(hkid, item_id.clone());
                                    ok.push((item_id.clone(), hk_str.clone()));
                                } else {
                                    log::error!(
                                        "ItemHotkeyManager: RegisterHotKey failed for '{}' (id=0x{:X}): {:?}",
                                        hk_str, hkid, r.err()
                                    );
                                    if !failed.contains(hk_str) {
                                        failed.push(hk_str.clone());
                                    }
                                }
                            }
                            None => {
                                log::error!(
                                    "ItemHotkeyManager: invalid hotkey string: '{}'",
                                    hk_str
                                );
                                if !failed.contains(hk_str) {
                                    failed.push(hk_str.clone());
                                }
                            }
                        }
                    }
                    let _ = result_tx.send(RegisterOutcome { failed, ok });
                    if id_map.is_empty() {
                        log::warn!(
                            "ItemHotkeyManager: no item hotkey registered, pump exiting"
                        );
                        return;
                    }

                    loop {
                        // P3-2：kill 事件句柄接入 MsgWait 句柄数组，SetEvent 后本等待
                        // 立即返回（WAIT_OBJECT_0=0），join 不再受 500ms 轮询周期影响。
                        let waited =
                            MsgWaitForMultipleObjects(Some(&[kill_h]), false, 500, QS_ALLINPUT);
                        match waited.0 {
                            0 => {
                                log::info!("ItemHotkeyManager: pump thread received kill signal");
                                break;
                            }
                            258 => {}
                            0xFFFF_FFFF => {
                                log::error!("ItemHotkeyManager: MsgWaitForMultipleObjects failed");
                                break;
                            }
                            _ => {
                                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) == TRUE {
                                    if msg.message == WM_HOTKEY {
                                        if let Some(item_id) = id_map.get(&(msg.wParam.0 as u32)) {
                                            let _ = sender.send(item_id.clone());
                                        }
                                    }
                                    let _ = DispatchMessageW(&msg);
                                }
                            }
                        }
                    }

                    for hkid in id_map.keys() {
                        let _ = UnregisterHotKey(None, *hkid as i32);
                    }
                }

                #[cfg(not(windows))]
                {
                    let _ = &sender;
                    let _ = &kill_signal;
                    let _ = &hotkeys_owned;
                    // 平台不支持：不算失败（无失败反馈），避免调用方误判配置错误
                    let _ = result_tx.send(RegisterOutcome {
                        failed: Vec::new(),
                        ok: Vec::new(),
                    });
                    log::warn!(
                        "ItemHotkeyManager: item hotkeys not supported on this platform"
                    );
                }
            });

        match handle {
            Ok(h) => {
                self.thread_handle = Some(h);
                // 超时保守视为全部失败（与旧 bool 语义一致：拿不到结果不报成功）；
                // current 同步置空，防止下次误判"集合未变"跳过注册
                let outcome = result_rx
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap_or_else(|_| {
                        log::error!("ItemHotkeyManager: register result timeout");
                        RegisterOutcome {
                            failed: dedup_hotkey_strings(hotkeys),
                            ok: Vec::new(),
                        }
                    });
                let any_ok = outcome.failed.len() < hotkeys.len();
                self.registered = any_ok || hotkeys.is_empty();
                self.pump_active = any_ok;
                self.current = outcome.ok;
                outcome.failed
            }
            Err(e) => {
                log::error!("ItemHotkeyManager: failed to start pump thread: {}", e);
                self.current.clear();
                dedup_hotkey_strings(hotkeys)
            }
        }
    }

    pub fn unregister(&mut self) -> bool {
        self.kill_pump_thread();
        self.pump_active = false;
        self.registered = false;
        // 线程已终止（热键实际全部注销）：差量集合必须清空，
        // 防止下次 register_all 误判"未变"跳过导致热键静默失效
        self.current.clear();
        true
    }

    fn kill_pump_thread(&mut self) {
        // 取走并置位 kill 信号；Windows 事件句柄随 KillGuard 持有，join 后再 Drop 关闭
        //（避免 pump 线程仍在 MsgWait 时句柄被提前关闭）。非 Windows 发送通道信号（无长驻 pump）。
        let guard = self.take_kill_guard();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
            log::info!("ItemHotkeyManager: pump thread joined");
        }
        drop(guard);
    }

    /// 取走 kill 信号并置位（P3-2）。Windows：SetEvent 使 pump 的 MsgWait 立即返回；
    /// 事件句柄放入 KillGuard 供 join 后关闭。非 Windows：发送通道信号（若有）。
    fn take_kill_guard(&mut self) -> KillGuard {
        #[cfg(windows)]
        {
            let ev = self.kill_event.take();
            if let Some(e) = &ev {
                e.signal();
            }
            KillGuard::Event(ev)
        }
        #[cfg(not(windows))]
        {
            if let Some(kill_tx) = self.kill_sender.take() {
                let _ = kill_tx.send(());
            }
            KillGuard::Dummy
        }
    }
}

impl Default for ItemHotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ItemHotkeyManager {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_ctrl_alt_m() {
        let hk = parse_hotkey_string("Ctrl+Alt+M").unwrap();
        assert_eq!(hk.modifiers, 0x0002 | 0x0001);
        assert_eq!(hk.vk, 'M' as u32);
    }

    #[test]
    fn test_parse_f12() {
        let hk = parse_hotkey_string("Ctrl+F12").unwrap();
        assert_eq!(hk.modifiers, 0x0002);
        assert_eq!(hk.vk, 0x7B);
    }

    #[test]
    fn test_parse_punct_keys() {
        let hk = parse_hotkey_string("Ctrl+/").unwrap();
        assert_eq!(hk.modifiers, 0x0002);
        assert_eq!(hk.vk, 0xBF);
        let hk = parse_hotkey_string("Ctrl+'").unwrap();
        assert_eq!(hk.modifiers, 0x0002);
        assert_eq!(hk.vk, 0xDE);
        let hk = parse_hotkey_string("Ctrl+;").unwrap();
        assert_eq!(hk.modifiers, 0x0002);
        assert_eq!(hk.vk, 0xBA);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_hotkey_string("").is_none());
        assert!(parse_hotkey_string("Ctrl+").is_none());
        // '+' 本身是分隔符，无法作为键位表达（既有限制）
        assert!(parse_hotkey_string("Ctrl++").is_none());
    }

    #[test]
    fn test_normalized_hotkey_case_and_trim_insensitive() {
        // 大小写/首尾空白差异归一化等价（"ctrl+m" 与 "Ctrl+M" 语义相同）
        assert_eq!(normalized_hotkey("Ctrl+M"), normalized_hotkey("ctrl+m"));
        assert_eq!(normalized_hotkey(" Ctrl+M "), normalized_hotkey("Ctrl+M"));
    }

    #[test]
    fn test_normalized_hotkey_set_order_and_case_insensitive() {
        let a = vec![
            ("i1".to_string(), "Ctrl+M".to_string()),
            ("i2".to_string(), "Ctrl+Alt+X".to_string()),
        ];
        let b = vec![
            ("i2".to_string(), "ctrl+alt+x".to_string()),
            ("i1".to_string(), " Ctrl+M".to_string()),
        ];
        // 同 id 同热键、仅顺序/大小写不同 → 集合相等（可差量跳过）
        assert_eq!(normalized_hotkey_set(&a), normalized_hotkey_set(&b));
        // 热键串不变但归属条目变化（迁移/互换）→ 集合不同（必须重注册，
        // 否则 pump 旧 id_map 仍指向旧条目，会触发错误条目——P1-1）
        let c = vec![
            ("i9".to_string(), "Ctrl+M".to_string()),
            ("i8".to_string(), "Ctrl+Alt+X".to_string()),
        ];
        assert_ne!(normalized_hotkey_set(&a), normalized_hotkey_set(&c));
    }

    #[test]
    fn test_normalized_hotkey_invalid_uses_raw() {
        // 解析失败的串按 trim 后原串参与比较（两端都会注册失败，行为一致）；
        // raw 通道保留原串字面（含大小写）——与可解析串的大小写无关性不同
        assert_eq!(normalized_hotkey("Ctrl++"), "raw:Ctrl++");
        assert_eq!(normalized_hotkey(" Ctrl++ "), "raw:Ctrl++");
        assert_ne!(normalized_hotkey("Ctrl++"), normalized_hotkey("Ctrl+M"));
    }

    #[test]
    fn test_dedup_hotkey_strings_keeps_order() {
        let hk = vec![
            ("a".to_string(), "Ctrl+M".to_string()),
            ("b".to_string(), "Ctrl+X".to_string()),
            ("c".to_string(), "Ctrl+M".to_string()),
        ];
        assert_eq!(
            dedup_hotkey_strings(&hk),
            vec!["Ctrl+M".to_string(), "Ctrl+X".to_string()]
        );
    }

    #[test]
    fn test_can_skip_reregister_when_unchanged_and_alive() {
        let mut mgr = ItemHotkeyManager::new();
        mgr.pump_active = true;
        mgr.thread_handle = Some(thread::spawn(|| {
            thread::sleep(Duration::from_millis(300));
        }));
        mgr.current = vec![("i1".into(), "Ctrl+1".into())];
        assert!(mgr.can_skip_reregister(&[("i1".into(), "Ctrl+1".into())]));
        // 大小写/空白归一化后相等仍视为未变
        assert!(mgr.can_skip_reregister(&[("i1".into(), " ctrl+1 ".into())]));
    }

    #[test]
    fn test_can_skip_reregister_false_when_set_changed() {
        let mut mgr = ItemHotkeyManager::new();
        mgr.pump_active = true;
        mgr.thread_handle = Some(thread::spawn(|| {
            thread::sleep(Duration::from_millis(300));
        }));
        mgr.current = vec![("i1".into(), "Ctrl+1".into())];
        // 热键迁移到另一条目（id 变化）：集合不同，不得跳过
        assert!(!mgr.can_skip_reregister(&[("i2".into(), "Ctrl+1".into())]));
        // 新增条目热键：集合不同，不得跳过
        assert!(!mgr.can_skip_reregister(&[
            ("i1".into(), "Ctrl+1".into()),
            ("i2".into(), "Ctrl+2".into())
        ]));
    }

    #[test]
    fn test_can_skip_reregister_false_when_pump_dead() {
        let mut mgr = ItemHotkeyManager::new();
        mgr.current = vec![("i1".into(), "Ctrl+1".into())];
        // pump 未激活：不跳过
        assert!(!mgr.can_skip_reregister(&[("i1".into(), "Ctrl+1".into())]));
        // pump 线程已结束：不跳过
        mgr.pump_active = true;
        mgr.thread_handle = Some(thread::spawn(|| {}));
        thread::sleep(Duration::from_millis(50));
        assert!(!mgr.can_skip_reregister(&[("i1".into(), "Ctrl+1".into())]));
    }
}
