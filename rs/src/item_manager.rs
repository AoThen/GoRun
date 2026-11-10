// 以下方法当前无调用者，但为规划中的功能（拖拽排序、分类编辑、移动条目等）预留，
// 保留而非删除；文件级 #![allow(dead_code)] 抑制编译警告。
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;

use crate::model::{AppConfig, Category, Item};

/// 拼音缓存上限（超限整体清空，条目键为文本原文，旧条目自然无害）
const PINYIN_CACHE_MAX: usize = 4096;

pub struct ItemManager {
    config: AppConfig,
    modified: bool,
    /// P1-9：文本 → 拼音对（首字母, 全拼）缓存；None 表示不含 CJK（无需拼音匹配）。
    pinyin_cache: RefCell<HashMap<String, Option<(String, String)>>>,
}

impl ItemManager {
    pub fn new(config: AppConfig) -> Self {
        log::debug!(
            "ItemManager created with {} categories",
            config.data.categories.len()
        );
        ItemManager {
            config,
            modified: false,
            pinyin_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AppConfig {
        self.modified = true;
        &mut self.config
    }

    pub fn categories(&self) -> &Vec<Category> {
        &self.config.data.categories
    }

    pub fn items(&self, category_id: &str) -> Vec<Item> {
        if category_id.is_empty() {
            return self.config.data.items.clone();
        }
        self.config
            .data
            .items
            .iter()
            .filter(|i| i.category_id == category_id)
            .cloned()
            .collect()
    }

    pub fn all_items(&self) -> Vec<Item> {
        self.config.data.items.clone()
    }

    /// P1-9：拼音匹配。文本级缓存，同一文本仅首次计算（首字母 + 全拼），
    /// 避免每次搜索按键对每个汉字项重算 lazy_pinyin。
    fn pinyin_match(&self, text: &str, q: &str) -> bool {
        let entry = {
            let mut cache = self.pinyin_cache.borrow_mut();
            match cache.get_mut(text) {
                Some(v) => v.clone(),
                None => {
                    if cache.len() >= PINYIN_CACHE_MAX {
                        cache.clear();
                    }
                    let computed = pinyin_pair(text);
                    cache.insert(text.to_string(), computed.clone());
                    computed
                }
            }
        };
        match entry {
            Some((init, full)) => init.contains(q) || full.contains(q),
            None => false,
        }
    }

    pub fn search_items(&self, query: &str) -> Vec<Item> {
        let q = query.to_lowercase();
        let q_trim = q.trim();
        if q_trim.is_empty() {
            return self.config.data.items.clone();
        }
        let mut scored: Vec<(Item, i32)> = self
            .config
            .data
            .items
            .iter()
            .filter(|i| {
                let name_l = i.name.to_lowercase();
                let kw_l = i.keywords.to_lowercase();
                let target_l = i.target.to_lowercase();
                if name_l.contains(&q)
                    || kw_l.contains(&q)
                    || target_l.contains(&q)
                {
                    return true;
                }
                // 拼音匹配（P2 #1/#2 + P1-9）：按文本缓存拼音对，避免每次搜索逐字符重算
                self.pinyin_match(&i.name, &q) || self.pinyin_match(&i.keywords, &q)
            })
            .map(|item| {
                let mut score = 0;
                if item.name.to_lowercase() == q {
                    score += 1000;
                } else if item.name.to_lowercase().starts_with(&q) {
                    score += 100;
                } else {
                    score += 10;
                }
                if item.keywords.to_lowercase().contains(&q) {
                    score += 50;
                }
                score += item.run_count;
                (item.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        let results: Vec<Item> = scored.into_iter().map(|(item, _)| item).collect();
        log::debug!("Search: query={}, results={}", query, results.len());
        results
    }

    pub fn add_category(&mut self, mut category: Category) {
        if category.id.is_empty() {
            category.id = crate::model::generate_id("cat");
        }
        log::info!("Category added: id={}, name={}", category.id, category.name);
        self.config.data.categories.push(category);
        self.modified = true;
    }

    pub fn update_category(&mut self, category: &Category) -> bool {
        if let Some(c) = self
            .config
            .data
            .categories
            .iter_mut()
            .find(|c| c.id == category.id)
        {
            *c = category.clone();
            self.modified = true;
            log::info!("Category updated: id={}", category.id);
            true
        } else {
            log::warn!("Category update failed: id={} not found", category.id);
            false
        }
    }

    /// 删除分类（级联删除项目）。返回是否实际删除；至少保留一个分类，避免空分类显示全部项目（P1-1）。
    pub fn delete_category(&mut self, id: &str) -> bool {
        if self.config.data.categories.len() <= 1 {
            log::warn!(
                "Cannot delete category: only {} left (id={})",
                self.config.data.categories.len(),
                id
            );
            return false;
        }
        let removed = self.config.data.items.iter().any(|i| i.category_id == id);
        self.config.data.items.retain(|i| i.category_id != id);
        self.config.data.categories.retain(|c| c.id != id);
        self.modified = true;
        log::info!(
            "Category deleted: id={}, cascaded_items={}",
            id,
            removed
        );
        true
    }

    /// 分类上移一位（在 categories 向量内交换，替代拖拽排序，决策①）。
    pub fn move_category_up(&mut self, id: &str) -> bool {
        self.move_category_in_vec(id, true)
    }

    /// 分类下移一位（在 categories 向量内交换，替代拖拽排序，决策①）。
    pub fn move_category_down(&mut self, id: &str) -> bool {
        self.move_category_in_vec(id, false)
    }

    fn move_category_in_vec(&mut self, id: &str, up: bool) -> bool {
        let cats = &mut self.config.data.categories;
        let idx = match cats.iter().position(|c| c.id == id) {
            Some(i) => i,
            None => return false,
        };
        let j = if up {
            if idx == 0 {
                return false;
            }
            idx - 1
        } else {
            if idx + 1 >= cats.len() {
                return false;
            }
            idx + 1
        };
        cats.swap(idx, j);
        self.modified = true;
        log::info!(
            "Category moved {} at {}, target {}",
            if up { "up" } else { "down" },
            idx,
            j
        );
        true
    }

    pub fn add_item(&mut self, mut item: Item) -> bool {
        if item.category_id.is_empty() {
            log::warn!("Item add failed: empty category_id");
            return false;
        }
        if item.id.is_empty() {
            item.id = crate::model::generate_id("item");
        }
        log::info!("Item added: id={}, name={}", item.id, item.name);
        self.config.data.items.push(item);
        self.modified = true;
        true
    }

    pub fn update_item(&mut self, item: &Item) -> bool {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item.id) {
            *i = item.clone();
            self.modified = true;
            log::info!("Item updated: id={}", item.id);
            true
        } else {
            log::warn!("Item update failed: id={} not found", item.id);
            false
        }
    }

    pub fn delete_item(&mut self, id: &str) -> bool {
        let before = self.config.data.items.len();
        self.config.data.items.retain(|i| i.id != id);
        let removed = self.config.data.items.len() < before;
        if removed {
            self.modified = true;
            log::info!("Item deleted: id={}", id);
        } else {
            log::warn!("Item delete failed: id={} not found", id);
        }
        removed
    }

    pub fn move_item(&mut self, item_id: &str, category_id: &str) -> bool {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item_id) {
            i.category_id = category_id.to_string();
            self.modified = true;
            log::info!("Item moved: {} -> {}", item_id, category_id);
            true
        } else {
            false
        }
    }

    /// 在同一分类内上移一位（与相邻兄弟交换，替代拖拽排序，决策①）。
    pub fn move_item_up(&mut self, item_id: &str) -> bool {
        self.swap_with_sibling(item_id, true)
    }

    /// 在同一分类内下移一位（与相邻兄弟交换，替代拖拽排序，决策①）。
    pub fn move_item_down(&mut self, item_id: &str) -> bool {
        self.swap_with_sibling(item_id, false)
    }

    fn swap_with_sibling(&mut self, item_id: &str, up: bool) -> bool {
        let items = &mut self.config.data.items;
        let idx = match items.iter().position(|i| i.id == item_id) {
            Some(i) => i,
            None => return false,
        };
        let cat = items[idx].category_id.clone();
        // 向前/向后找同分类的最近兄弟（跳过其他分类的项）
        let target = if up {
            (0..idx)
                .rev()
                .find(|&j| items[j].category_id == cat)
        } else {
            ((idx + 1)..items.len()).find(|&j| items[j].category_id == cat)
        };
        if let Some(j) = target {
            items.swap(idx, j);
            self.modified = true;
            log::info!("Item moved {} at {}, target {}", if up { "up" } else { "down" }, idx, j);
            true
        } else {
            log::debug!("Item {} already at boundary", item_id);
            false
        }
    }

    pub fn increment_run_count(&mut self, item_id: &str) {
        if let Some(i) = self.config.data.items.iter_mut().find(|i| i.id == item_id) {
            i.run_count += 1;
            self.modified = true;
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn reset_modified(&mut self) {
        self.modified = false;
    }

    pub fn handle_drop(&mut self, files: &[std::path::PathBuf], category_id: &str) -> usize {
        let mut count = 0;
        for path in files {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let target = path.to_string_lossy().to_string();
            let working_dir = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let item = Item {
                id: crate::model::generate_id("item"),
                name,
                target,
                working_dir,
                category_id: category_id.to_string(),
                ..Default::default()
            };

            if self.add_item(item) {
                count += 1;
            }
        }
        count
    }

    pub fn get_item(&self, id: &str) -> Option<&Item> {
        self.config.data.items.iter().find(|i| i.id == id)
    }

    /// 按名称排序分类内项目（背景右键菜单"按名称排序"）。
    pub fn sort_items_by_name(&mut self, category_id: &str) {
        self.sort_category_items(category_id, |a, b| {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        });
    }

    /// 按使用次数排序分类内项目（背景右键菜单"按使用次数排序"）。
    pub fn sort_items_by_usage(&mut self, category_id: &str) {
        self.sort_category_items(category_id, |a, b| b.run_count.cmp(&a.run_count));
    }

    /// 对分类内项目稳定排序（仅重排该分类子集，不影响其他分类相对顺序）。
    fn sort_category_items<F>(&mut self, category_id: &str, cmp: F)
    where
        F: Fn(&Item, &Item) -> std::cmp::Ordering,
    {
        let items = &mut self.config.data.items;
        let mut positions: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.category_id == category_id)
            .map(|(idx, _)| idx)
            .collect();
        if positions.len() < 2 {
            return;
        }
        let total = positions.len();
        let mut subset: Vec<Item> = positions.iter().map(|&p| items[p].clone()).collect();
        subset.sort_by(&cmp);
        for (slot, item) in positions.drain(..).zip(subset) {
            items[slot] = item;
        }
        self.modified = true;
        log::info!("Sorted {} items in category {}", total, category_id);
    }
}

impl Default for ItemManager {
    fn default() -> Self {
        ItemManager::new(AppConfig::default())
    }
}

/// 判断字符是否为 CJK 汉字（拼音匹配仅对汉字生效）。
fn is_cjk(c: char) -> bool {
    let u = c as u32;
    (0x4E00..=0x9FFF).contains(&u) || (0x3400..=0x4DBF).contains(&u)
}

/// 字符串是否含 CJK 汉字（用于拼音匹配前置判断）。
fn contains_cjk(s: &str) -> bool {
    s.chars().any(is_cjk)
}

/// 取文本的拼音首字母（非汉字保留原字符小写）。
fn pinyin_initial(text: &str) -> String {
    let args = pinyin::Args::new();
    let mut out = String::new();
    for ch in text.chars() {
        if !is_cjk(ch) {
            out.push(ch.to_ascii_lowercase());
            continue;
        }
        if let Some(p) = pinyin::lazy_pinyin(&ch.to_string(), &args).into_iter().next() {
            if let Some(c) = p.chars().next() {
                out.push(c.to_ascii_lowercase());
            }
        }
    }
    out
}

/// 取文本的拼音对（首字母 + 全拼）（P1-9，供缓存）；非中文返回 None。
fn pinyin_pair(text: &str) -> Option<(String, String)> {
    if !contains_cjk(text) {
        return None;
    }
    Some((pinyin_initial(text), pinyin_full(text)))
}

/// 取文本的完整拼音（多音字取第一个读音，非汉字保留原字符小写）。
fn pinyin_full(text: &str) -> String {
    let args = pinyin::Args::new();
    let mut out = String::new();
    for ch in text.chars() {
        if !is_cjk(ch) {
            out.push(ch.to_ascii_lowercase());
            continue;
        }
        if let Some(p) = pinyin::lazy_pinyin(&ch.to_string(), &args).into_iter().next() {
            out.push_str(&p.to_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppData, Config};

    fn make_item(id: &str, name: &str, cat: &str) -> Item {
        Item {
            id: id.into(),
            name: name.into(),
            target: format!("C:\\bin\\{}.exe", name),
            category_id: cat.into(),
            ..Default::default()
        }
    }

    fn make_manager(items: Vec<Item>) -> ItemManager {
        let config = AppConfig {
            version: "test".into(),
            data: AppData {
                categories: vec![],
                items,
            },
            config: Config::default(),
            extra_config: Default::default(),
        };
        ItemManager::new(config)
    }

    #[test]
    fn test_search_exact_name_ranks_first() {
        let m = make_manager(vec![
            make_item("a", "Calculator Pro", "c1"),
            make_item("b", "Calculator", "c1"),
        ]);
        let res = m.search_items("calculator");
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].name, "Calculator");
    }

    #[test]
    fn test_search_matches_keyword() {
        let mut m = make_manager(vec![]);
        let mut it = make_item("k", "Calc Tool", "c1");
        it.keywords = "math calculator".into();
        m.config.data.items.push(it);
        let res = m.search_items("math");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].id, "k");
    }

    #[test]
    fn test_search_no_result() {
        let m = make_manager(vec![make_item("a", "Foo", "c1")]);
        assert!(m.search_items("zzz").is_empty());
    }

    #[test]
    fn test_add_update_delete_item() {
        let mut m = make_manager(vec![]);
        assert!(m.add_item(make_item("i1", "One", "c1")));
        assert!(m.is_modified());
        assert_eq!(m.config.data.items.len(), 1);

        let it = make_item("i1", "One v2", "c1");
        assert!(m.update_item(&it));
        assert_eq!(m.config.data.items[0].name, "One v2");

        assert!(m.delete_item("i1"));
        assert!(m.config.data.items.is_empty());
        assert!(!m.delete_item("nope"));
    }

    #[test]
    fn test_delete_category_cascades_items() {
        let mut m = make_manager(vec![make_item("i1", "A", "c1")]);
        m.add_category(Category {
            id: "c1".into(),
            name: "C1".into(),
            ..Default::default()
        });
        m.add_category(Category {
            id: "c2".into(),
            name: "C2".into(),
            ..Default::default()
        });
        m.config.data.items.push(make_item("i2", "B", "c2"));
        assert!(m.delete_category("c1"));
        assert!(m.config.data.items.iter().all(|i| i.category_id != "c1"));
        assert_eq!(m.config.data.items.len(), 1);
        assert_eq!(m.config.data.categories.len(), 1);
    }

    #[test]
    fn test_delete_category_keeps_last_category() {
        let mut m = make_manager(vec![make_item("i1", "A", "c1")]);
        m.add_category(Category {
            id: "c1".into(),
            name: "C1".into(),
            ..Default::default()
        });
        // add_category 会置 modified，重置以便验证 delete 失败不改状态
        m.reset_modified();
        // 仅剩一个分类时拒绝删除（P1-1）
        assert!(!m.delete_category("c1"));
        assert_eq!(m.config.data.categories.len(), 1);
        assert!(!m.is_modified());
    }

    #[test]
    fn test_move_item() {
        let mut m = make_manager(vec![make_item("i1", "A", "c1")]);
        assert!(m.move_item("i1", "c2"));
        assert_eq!(m.config.data.items[0].category_id, "c2");
        assert!(!m.move_item("nope", "c2"));
    }

    #[test]
    fn test_increment_run_count_affects_search_rank() {
        let mut m = make_manager(vec![make_item("a", "Tool Alpha", "c1")]);
        let mut b = make_item("b", "Tool Beta", "c1");
        b.run_count = 5;
        m.config.data.items.push(b);
        m.increment_run_count("a");
        let res = m.search_items("tool");
        assert_eq!(res[0].id, "b");
    }

    #[test]
    fn test_add_category_generates_id() {
        let mut m = make_manager(vec![]);
        m.add_category(Category {
            name: "New".into(),
            ..Default::default()
        });
        assert_eq!(m.config.data.categories.len(), 1);
        assert!(!m.config.data.categories[0].id.is_empty());
    }

    #[test]
    fn test_handle_drop_adds_items() {
        let mut m = make_manager(vec![]);
        let paths = vec![std::path::PathBuf::from("C:\\Tools\\foo.exe")];
        let n = m.handle_drop(&paths, "c1");
        assert_eq!(n, 1);
        assert_eq!(m.config.data.items.len(), 1);
        assert_eq!(m.config.data.items[0].category_id, "c1");
    }

    #[test]
    fn test_move_item_up_down_within_category() {
        // 交替分类，验证仅在同分类内交换
        let mut m = make_manager(vec![
            make_item("a", "A", "c1"),
            make_item("x", "X", "c2"),
            make_item("b", "B", "c1"),
        ]);
        // b(索引2) 上移 → 应与 a(索引0) 交换（跳过 c2 的 x）
        assert!(m.move_item_up("b"));
        assert_eq!(m.config.data.items[0].id, "b");
        assert_eq!(m.config.data.items[1].id, "x");
        assert_eq!(m.config.data.items[2].id, "a");
        // b 已到顶，再上移失败
        assert!(!m.move_item_up("b"));
        // a(索引2) 在 c1 组尾部，下移无同分类兄弟 → 失败
        assert!(!m.move_item_down("a"));
        // 未知 id
        assert!(!m.move_item_up("nope"));
    }

    #[test]
    fn test_move_category_up_down() {
        let mut m = make_manager(vec![]);
        m.add_category(Category {
            id: "c1".into(),
            name: "C1".into(),
            ..Default::default()
        });
        m.add_category(Category {
            id: "c2".into(),
            name: "C2".into(),
            ..Default::default()
        });
        m.add_category(Category {
            id: "c3".into(),
            name: "C3".into(),
            ..Default::default()
        });
        assert!(m.move_category_up("c2"));
        assert_eq!(m.config.data.categories[0].id, "c2");
        assert_eq!(m.config.data.categories[1].id, "c1");
        assert!(m.move_category_down("c1"));
        assert_eq!(m.config.data.categories[0].id, "c2");
        assert_eq!(m.config.data.categories[1].id, "c3");
        assert_eq!(m.config.data.categories[2].id, "c1");
        assert!(!m.move_category_up("c2")); // 顶
        assert!(!m.move_category_down("c1")); // 底
        assert!(!m.move_category_up("nope"));
    }

    #[test]
    fn test_sort_items_by_name_and_usage() {
        let mut m = make_manager(vec![
            make_item("b", "banana", "c1"),
            make_item("a", "Apple", "c1"),
            make_item("z", "zebra", "c2"), // 其他分类不受影响
        ]);
        m.sort_items_by_name("c1");
        let c1: Vec<&str> = m
            .config
            .data
            .items
            .iter()
            .filter(|i| i.category_id == "c1")
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(c1, vec!["a", "b"]);

        let mut it = m.config.data.items[0].clone(); // "a" 原为 Apple，现索引0
        it.run_count = 10;
        m.config.data.items[0] = it;
        m.sort_items_by_usage("c1");
        let c1_usage: Vec<&str> = m
            .config
            .data
            .items
            .iter()
            .filter(|i| i.category_id == "c1")
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(c1_usage, vec!["a", "b"]);
    }

    #[test]
    fn test_pinyin_search() {
        // "计算器" 拼音首字母 jsq / 全拼 jisuanqi
        let mut m = make_manager(vec![]);
        let mut it = make_item("py", "计算器", "c1");
        it.keywords = "math tool".into();
        m.config.data.items.push(it);
        // 首字母匹配
        let res = m.search_items("jsq");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].id, "py");
        // 全拼匹配
        let res = m.search_items("jisuanqi");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].id, "py");
        // 无匹配
        assert!(m.search_items("zzzz").is_empty());
    }

    #[test]
    fn test_pinyin_cache_populated_and_stable() {
        // P1-9：同一文本仅计算一次，重复搜索命中缓存且结果一致
        let mut m = make_manager(vec![]);
        let mut it = make_item("py", "计算器", "c1");
        it.keywords = "abc".into();
        m.config.data.items.push(it);
        let first = m.search_items("jsq");
        let second = m.search_items("jsq");
        assert_eq!(first, second);
        assert_eq!(second.len(), 1);
        let cache = m.pinyin_cache.borrow();
        // 名称拼音被计算并入缓存；搜索命中名称时 || 短路，keywords 不保证入缓存
        assert!(cache.contains_key("计算器"));
        assert!(cache.get("计算器").unwrap().is_some());
    }

    #[test]
    fn test_pinyin_initial_full() {
        assert_eq!(pinyin_initial("计算器"), "jsq");
        assert!(pinyin_full("计算器").contains("jisuanqi"));
        // 非汉字原样
        assert_eq!(pinyin_initial("Abc"), "abc");
        assert_eq!(pinyin_full("Abc"), "abc");
    }
}
