use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;

/// 从 Cargo.toml 读取项目名称
pub fn get_project_name() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 环境变量未设置");
    let cargo_toml_path = PathBuf::from(&manifest_dir).join("Cargo.toml");
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path).expect("无法读取 Cargo.toml");

    cargo_toml_content
        .lines()
        .find(|line| line.trim().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .expect("无法从 Cargo.toml 获取项目名称")
}

/// 获取 target 目录路径
pub fn get_target_dir() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR 环境变量未设置");

    env::var("CARGO_TARGET_DIR")
        .or_else(|_| env::var("CARGO_BUILD_TARGET_DIR"))
        .unwrap_or_else(|_| {
            PathBuf::from(&manifest_dir)
                .join("target")
                .to_string_lossy()
                .to_string()
        })
}

/// 根据项目名称和平台生成可执行文件路径
pub fn get_binary_path(project_name: &str, target_dir: &str) -> PathBuf {
    let exe_name = if cfg!(windows) {
        format!("{}.exe", project_name)
    } else {
        project_name.to_string()
    };

    PathBuf::from(target_dir).join("debug").join(&exe_name)
}

/// 查找一个可用的端口
pub fn find_available_port() -> u16 {
    // 绑定到端口 0 让操作系统自动分配空闲端口
    let listener = TcpListener::bind("127.0.0.1:0").expect("无法绑定到任意端口");

    let port = listener.local_addr().expect("无法获取本地地址").port();

    drop(listener); // 释放端口
    port
}

/// 读取日志文件的最后 N 行
pub fn read_last_n_lines(file_path: &str, n: usize) -> String {
    match fs::read_to_string(file_path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();

            let last_n_lines = if lines.len() > n {
                &lines[lines.len() - n..]
            } else {
                &lines[..]
            };

            last_n_lines.join("\n")
        }
        Err(_) => String::new(),
    }
}
