use crate::app::monitoring::RawSample;
use bollard::{Docker, query_parameters::StatsOptionsBuilder};
use futures_util::StreamExt;

pub async fn container_stats(client: &Docker, id: &str) -> Result<RawSample, String> {
    let options = StatsOptionsBuilder::new()
        .stream(false)
        .one_shot(true)
        .build();
    let mut stream = client.stats(id, Some(options));
    let stats = stream
        .next()
        .await
        .ok_or_else(|| "stats unavailable".to_string())?
        .map_err(|error| error.to_string())?;
    let cpu = stats.cpu_stats.as_ref();
    let usage = cpu.and_then(|stats| stats.cpu_usage.as_ref());
    let memory = stats.memory_stats.as_ref();
    let (network_rx, network_tx) = stats
        .networks
        .as_ref()
        .map(|networks| {
            networks.values().fold((0, 0), |(rx, tx), network| {
                (
                    rx + network.rx_bytes.unwrap_or(0),
                    tx + network.tx_bytes.unwrap_or(0),
                )
            })
        })
        .unwrap_or((0, 0));
    let (block_read, block_write) = stats
        .blkio_stats
        .as_ref()
        .and_then(|stats| stats.io_service_bytes_recursive.as_ref())
        .map(|entries| {
            entries.iter().fold((0, 0), |(read, write), entry| {
                let value = entry.value.unwrap_or(0);
                match entry
                    .op
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "read" => (read + value, write),
                    "write" => (read, write + value),
                    _ => (read, write),
                }
            })
        })
        .unwrap_or((0, 0));
    Ok(RawSample {
        cpu_total: usage.and_then(|usage| usage.total_usage).unwrap_or(0),
        system_total: cpu.and_then(|stats| stats.system_cpu_usage).unwrap_or(0),
        online_cpus: u64::from(cpu.and_then(|stats| stats.online_cpus).unwrap_or(1)),
        memory_usage: memory.and_then(|stats| stats.usage).unwrap_or(0),
        memory_limit: memory.and_then(|stats| stats.limit).unwrap_or(0),
        network_rx,
        network_tx,
        block_read,
        block_write,
    })
}
