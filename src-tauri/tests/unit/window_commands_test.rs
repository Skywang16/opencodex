/*!
 * Window Commands 单元测试
 *
 * 测试窗口命令模块的状态管理和配置功能
 */

use terminal_lib::window::commands::{WindowConfigManager, WindowStateManager};

#[test]
fn test_window_state_manager_creation() {
    let manager = WindowStateManager::new();

    assert!(!manager.is_always_on_top());
}

#[test]
fn test_always_on_top_toggle() {
    let mut manager = WindowStateManager::new();

    // 初始状态为false
    assert!(!manager.is_always_on_top());

    // 切换为true
    let new_value = manager.toggle_always_on_top();
    assert!(new_value);
    assert!(manager.is_always_on_top());

    // 再次切换回false
    let new_value = manager.toggle_always_on_top();
    assert!(!new_value);
    assert!(!manager.is_always_on_top());
}

#[test]
fn test_always_on_top_set() {
    let mut manager = WindowStateManager::new();

    manager.set_always_on_top(true);
    assert!(manager.is_always_on_top());

    manager.set_always_on_top(false);
    assert!(!manager.is_always_on_top());
}

#[test]
fn test_cache_clear() {
    let mut manager = WindowStateManager::new();

    manager.set_always_on_top(true);

    // 清除缓存
    manager.clear_cache();

    // always_on_top不受影响
    assert!(manager.is_always_on_top());
}

#[test]
fn test_state_reset() {
    let mut manager = WindowStateManager::new();

    manager.set_always_on_top(true);

    // 重置
    manager.reset();

    // 验证所有状态已重置
    assert!(!manager.is_always_on_top());
}

#[test]
fn test_window_config_manager_creation() {
    let manager = WindowConfigManager::new();

    assert!(manager.platform_info().is_none());
    assert_eq!(manager.default_window_id(), "main");
}

#[test]
fn test_concurrent_state_access() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let manager: Arc<Mutex<WindowStateManager>> = Arc::new(Mutex::new(WindowStateManager::new()));
    let mut handles = Vec::new();

    // 多线程并发访问
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let mut mgr = manager_clone.lock().unwrap();
            mgr.set_always_on_top(i % 2 == 0);
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 并发访问完成后依然可写且可读（不死锁/不崩溃）
    manager.lock().unwrap().set_always_on_top(true);
    assert!(manager.lock().unwrap().is_always_on_top());
}

#[test]
fn test_atomic_always_on_top() {
    use std::sync::Arc;

    let manager = Arc::new(WindowStateManager::new());

    // 测试原子操作的并发安全性
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let mgr = Arc::clone(&manager);
            std::thread::spawn(move || {
                for _ in 0..10 {
                    if i % 2 == 0 {
                        mgr.is_always_on_top();
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // 无写入时，默认值应保持不变
    assert!(!manager.is_always_on_top());
}
