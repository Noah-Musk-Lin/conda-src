use crate::models::{MirrorProfile, SpeedTestResult};
use futures::future::join_all;
use reqwest::Client;
use std::time::{Duration, Instant};

async fn probe_single(client: &Client, mirror: &MirrorProfile) -> SpeedTestResult {
    let url = mirror.probe_url.trim();
    if url.is_empty() {
        return SpeedTestResult {
            mirror_id: mirror.id.clone(),
            ok: false,
            latency_ms: None,
            status_code: None,
            error: Some("未配置测速地址".to_string()),
        };
    }

    let start = Instant::now();
    let head_resp = client.head(url).send().await;

    let resp = match head_resp {
        Ok(r) if r.status().as_u16() == 405 => {
            // 部分镜像站不支持 HEAD，降级为 1 字节 Range GET
            client
                .get(url)
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await
        }
        other => other,
    };

    let elapsed = start.elapsed().as_millis();

    match resp {
        Ok(r) => {
            let status = r.status().as_u16();
            let ok = status < 400;
            SpeedTestResult {
                mirror_id: mirror.id.clone(),
                ok,
                latency_ms: Some(elapsed),
                status_code: Some(status),
                error: if ok {
                    None
                } else {
                    Some(format!("HTTP {status}"))
                },
            }
        }
        Err(e) => {
            let msg = if e.is_timeout() {
                "连接超时 (>6s)".to_string()
            } else if e.is_connect() {
                "无法连接服务器".to_string()
            } else {
                format!("请求失败: {e}")
            };
            SpeedTestResult {
                mirror_id: mirror.id.clone(),
                ok: false,
                latency_ms: None,
                status_code: None,
                error: Some(msg),
            }
        }
    }
}

pub async fn run_all_speedtests(mirrors: Vec<MirrorProfile>) -> Result<Vec<SpeedTestResult>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(6))
        .connect_timeout(Duration::from_secs(5))
        .user_agent("CondaSrc/0.1.0")
        .build()
        .map_err(|e| format!("初始化 HTTP 客户端失败: {e}"))?;

    let futures = mirrors.iter().map(|m| probe_single(&client, m));
    Ok(join_all(futures).await)
}
