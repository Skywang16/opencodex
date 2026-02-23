//! 单例模块测试
//!
//! 测试全局 TerminalMux 单例的行为

use std::sync::Arc;
use terminal_lib::mux::{get_mux, PtySize};

#[test]
fn test_singleton_behavior() {
    // 第一次获取应该创建实例
    let mux1 = get_mux();

    // 第二次获取应该返回同一个实例
    let mux2 = get_mux();

    // 验证是同一个实例（通过Arc的指针比较）
    assert!(Arc::ptr_eq(&mux1, &mux2));
}

#[tokio::test]
async fn test_mux_functionality_through_singleton() {
    let mux = get_mux();
    let initial_count = mux.pane_count();

    // 测试通过单例创建面板
    let pane_id = mux.create_pane(PtySize::default()).await.unwrap();
    assert_eq!(mux.pane_count(), initial_count + 1);

    // 清理
    mux.remove_pane(pane_id).unwrap();

    // 验证清理后面板数量回到初始状态
    assert_eq!(mux.pane_count(), initial_count);
}
