use serde::Deserialize;
use std::{env, error::Error};

#[derive(Debug, Deserialize)]
pub struct SysEntry {
    pub cpu: String,
    pub model: String,
    pub os: String,
    pub uptime: String,
    pub ram_total_mb: u64,
    pub metrics: Metrics,
}

#[derive(Debug, Deserialize)]
pub struct Metrics {
    pub cpu: CpuMetrics,
    pub memory: MemMetrics,
}

#[derive(Debug, Deserialize)]
pub struct CpuMetrics {
    pub temp_c: f32,
}

#[derive(Debug, Deserialize)]
pub struct MemMetrics {
    pub available_mb: u64,
}

impl SysEntry {
    pub fn used_mb(&self) -> u64 {
        self.ram_total_mb
            .saturating_sub(self.metrics.memory.available_mb)
    }
}

pub async fn get_sys_info() -> Result<SysEntry, Box<dyn Error>> {
    let url = env::var("API_URL")? + "/system";
    Ok(reqwest::get(url).await?.json::<SysEntry>().await?)
}