//! Read-only Codex App Server integration. Never starts a model turn.
use serde::Serialize;
use serde_json::{json, Value};
use std::{path::PathBuf, process::Stdio, time::{Duration, SystemTime, UNIX_EPOCH}};
use tokio::{io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader, Lines}, process::{ChildStdin, Command}, sync::Mutex, time::{timeout, Instant}};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub fetched_at: u64,
    pub source: String,
    pub account: Value,
    pub rate_limits: Value,
    pub usage: Value,
}
#[derive(Default)]
pub struct UsageCache(Mutex<Option<(Instant, UsageSnapshot)>>);
impl UsageCache {
    pub async fn read(&self) -> Result<UsageSnapshot, String> {
        let mut cached = self.0.lock().await;
        if let Some((at, snapshot)) = cached.as_ref() {
            if at.elapsed() < Duration::from_secs(60) { return Ok(snapshot.clone()); }
        }
        let snapshot = fetch().await?;
        *cached = Some((Instant::now(), snapshot.clone()));
        Ok(snapshot)
    }
}
fn executable() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CODEX_BIN") { return Ok(path.into()); }
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).map(|p| p.join(if cfg!(windows) { "codex.exe" } else { "codex" })).collect()).unwrap_or_default();
    paths.extend(["/opt/homebrew/bin/codex", "/usr/local/bin/codex", "/Applications/Codex.app/Contents/Resources/codex", "/Applications/ChatGPT.app/Contents/Resources/codex"].map(PathBuf::from));
    paths.into_iter().find(|p| p.is_file()).ok_or_else(|| "找不到 Codex CLI；請安裝 Codex 或設定 CODEX_BIN".into())
}
async fn send(input: &mut ChildStdin, message: Value) -> Result<(), String> {
    input.write_all(format!("{message}\n").as_bytes()).await.map_err(|e| e.to_string())?;
    input.flush().await.map_err(|e| e.to_string())
}
async fn response<R: AsyncBufRead + Unpin>(lines: &mut Lines<R>, id: u64) -> Result<Value, String> {
    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        let message: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if message.get("id").and_then(Value::as_u64) == Some(id) { return Ok(message); }
    }
    Err("Codex App Server 已結束，尚未回傳資料".into())
}
fn result(message: Value) -> Result<Value, String> {
    if let Some(error) = message.get("error") { return Err(error.to_string()); }
    message.get("result").cloned().ok_or_else(|| "Codex 回應缺少 result".into())
}
pub async fn fetch() -> Result<UsageSnapshot, String> {
    let executable = executable()?;
    let mut child = Command::new(&executable).arg("app-server")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true)
        .spawn().map_err(|e| format!("無法啟動 Codex App Server: {e}"))?;
    let mut input = child.stdin.take().ok_or("Missing Codex stdin")?;
    let mut lines = BufReader::new(child.stdout.take().ok_or("Missing Codex stdout")?).lines();
    let exchange = async {
        send(&mut input, json!({"id":0,"method":"initialize","params":{"clientInfo":{"name":"desktop_pet","version":"0.1.0"}}})).await?;
        result(response(&mut lines, 0).await?)?;
        send(&mut input, json!({"method":"initialized","params":{}})).await?;
        let mut results = Vec::new();
        for (index, method) in ["account/rateLimits/read", "account/read", "account/usage/read"].iter().enumerate() {
            let id = index as u64 + 1;
            let params = if *method == "account/read" { json!({"refreshToken":false}) } else { json!({}) };
            send(&mut input, json!({"id":id,"method":method,"params":params})).await?;
            let reply = response(&mut lines, id).await?;
            // Quota is required; optional account/activity APIs may be absent on older CLIs.
            let value = if index == 0 { result(reply)? } else { result(reply).unwrap_or_else(|e| json!({"error":e})) };
            results.push(value);
        }
        Ok(UsageSnapshot {
            fetched_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            source: executable.to_string_lossy().into_owned(),
            rate_limits: results[0].clone(), account: results[1].clone(), usage: results[2].clone(),
        })
    };
    let fetched = timeout(Duration::from_secs(25), exchange).await.unwrap_or_else(|_| Err("Codex 用量查詢逾時（25 秒）".into()));
    drop(input);
    let _ = child.kill().await;
    let _ = child.wait().await;
    fetched
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ignores_notifications_and_unrelated_responses() {
        let input = b"{\"method\":\"account/updated\",\"params\":{}}\n{\"id\":9,\"result\":{}}\n{\"id\":1,\"result\":{\"primary\":null}}\n";
        let mut lines = BufReader::new(&input[..]).lines();
        assert_eq!(result(response(&mut lines, 1).await.unwrap()).unwrap(), json!({"primary":null}));
        assert!(response(&mut lines, 2).await.is_err());
    }
    #[test]
    fn preserves_protocol_errors_instead_of_treating_them_as_empty_usage() {
        assert!(result(json!({"id":1,"error":{"code":-32601,"message":"Unknown method"}})).unwrap_err().contains("Unknown method"));
        assert!(result(json!({"id":1})).is_err());
    }
}
