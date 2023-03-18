macro_rules! auto_increment {
    () => {{
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }};
}

pub(crate) mod client;
pub(crate) mod server;
