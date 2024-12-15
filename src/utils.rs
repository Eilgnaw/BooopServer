use chrono::Local;

/// 带时间戳的 eprintln 宏
#[macro_export]
macro_rules! eprintln_with_time {
    ($($arg:tt)*) => {{
        eprintln!(
            "[{}] {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            format!($($arg)*)
        );
    }};
}