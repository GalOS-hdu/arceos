//! Hardware Abstraction Layer for lwext4_core
//!
//! 为 lwext4_core 提供系统时间接口

use core::time::Duration;
use lwext4_core::SystemHal;

/// ArceOS Hardware Abstraction Layer implementation
pub struct ArceOsHal;

impl SystemHal for ArceOsHal {
    fn now() -> Option<Duration> {
        // TODO: 集成 ArceOS 的时间接口
        // 暂时返回 None，表示不提供时间戳
        // 这不影响核心文件系统功能，只会导致文件时间戳为 0
        None
    }
}
