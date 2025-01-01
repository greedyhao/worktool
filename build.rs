use std::process::Command;
use std::io::Write;
use chrono::SecondsFormat;

fn main() {
    // 获取当前时间
    let compile_time = chrono::Local::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // 获取 git 提交哈希
    let git_hash = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .map(|output| String::from_utf8(output.stdout).unwrap())
        .unwrap_or_else(|_| "unknown".to_string());

    // 获取 git 提交时间
    let git_timestamp = Command::new("git")
        .args(&["log", "-1", "--format=%cd", "--date=iso"])
        .output()
        .map(|output| String::from_utf8(output.stdout).unwrap())
        .unwrap_or_else(|_| "unknown".to_string());

    // 将信息写入文件
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("info.rs");
    let mut f = std::fs::File::create(&dest_path).unwrap();

    writeln!(f, "pub const GIT_HASH: &str = \"{}\";", git_hash.trim()).unwrap();
    writeln!(f, "pub const GIT_TIMESTAMP: &str = \"{}\";", git_timestamp.trim()).unwrap();
    writeln!(f, "pub const COMPILE_TIME: &str = \"{}\";", compile_time).unwrap();
}
