//! 系统/调试工具：component_counts、console_messages、evaluate_script

use crossbeam_channel::Sender;
use serde_json::{json, Value};

use crate::test_system::channel::{LogEntryData, TestMessage};

use super::dispatch_shared::{arg_str, send, TIMEOUT};

macro_rules! try_ok {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        }
    };
}

pub async fn handle(
    sender: &Sender<TestMessage>,
    name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    Some(match name {
        "component_counts" => {
            let mut list: Vec<Value> = match send(
                sender,
                |tx| TestMessage::QueryComponents { response: tx },
                TIMEOUT,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            }
            .into_iter()
            .map(|(n, c)| json!({ "name": n, "count": c }))
            .collect();
            list.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            Ok(list.into())
        }

        "console_messages" => {
            let lines = args["lines"].as_u64().unwrap_or(50) as u32;
            let log_file = args["log_file"].as_str().map(String::from);

            let (tx, rx) = tokio::sync::oneshot::channel::<Vec<LogEntryData>>();
            std::thread::spawn(move || {
                let file_path = log_file.unwrap_or_else(|| {
                    std::env::var("TEST_LOG_FILE").unwrap_or_else(|_| "logs/game.log".to_string())
                });
                let entries = read_log_file(&file_path, lines);
                let _ = tx.send(entries);
            });

            let entries =
                match tokio::time::timeout(tokio::time::Duration::from_secs(TIMEOUT), rx).await {
                    Ok(Ok(v)) => v,
                    Ok(Err(_)) => return Some(Err("读取日志失败".to_string())),
                    Err(_) => return Some(Err("读取日志超时".to_string())),
                };

            Ok(entries
                .into_iter()
                .map(
                    |e| json!({ "timestamp": e.timestamp, "level": e.level, "message": e.message }),
                )
                .collect::<Vec<_>>()
                .into())
        }

        "evaluate_script" => {
            let script = try_ok!(arg_str(args, "script"));
            let result = try_ok!(
                send(
                    sender,
                    |tx| TestMessage::EvaluateScript {
                        script,
                        response: tx
                    },
                    TIMEOUT
                )
                .await
            );
            Ok(json!({ "result": result }))
        }

        _ => return None,
    })
}

fn read_log_file(file_path: &str, lines: u32) -> Vec<LogEntryData> {
    match std::fs::read_to_string(file_path) {
        Ok(content) => {
            let all_lines: Vec<&str> = content.lines().collect();
            let start = all_lines.len().saturating_sub(lines as usize);
            all_lines[start..]
                .iter()
                .map(|l| parse_log_line(l))
                .collect()
        }
        Err(_) => vec![],
    }
}

fn parse_log_line(line: &str) -> LogEntryData {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() >= 3 {
        let timestamp = parts[0].to_string();
        if let (Some(start), Some(end)) = (line.find('['), line.find(']')) {
            let level = line[start + 1..end].to_string();
            let message = line[end + 1..]
                .trim_start_matches([' ', '-'])
                .trim()
                .to_string();
            return LogEntryData {
                timestamp,
                level,
                message,
            };
        }
    }
    LogEntryData {
        timestamp: String::new(),
        level: "INFO".to_string(),
        message: line.to_string(),
    }
}
