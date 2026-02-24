use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host, SupportedBufferSize, SupportedStreamConfigRange};

fn device_name(dev: &Device) -> String {
    dev.description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "???".into())
}

pub fn host() -> Host {
    cpal::default_host()
}

pub fn input_device_list(host: &Host) -> Result<Vec<(usize, String, Device)>> {
    Ok(host
        .input_devices()?
        .enumerate()
        .map(|(i, d)| {
            let name = device_name(&d);
            (i, name, d)
        })
        .collect())
}

pub fn output_device_list(host: &Host) -> Result<Vec<(usize, String, Device)>> {
    Ok(host
        .output_devices()?
        .enumerate()
        .map(|(i, d)| {
            let name = device_name(&d);
            (i, name, d)
        })
        .collect())
}

pub fn negotiate_config(
    input: &Device,
    output: &Device,
) -> Result<(u16, u16)> {
    let in_cfg = input.default_input_config()?;
    let out_cfg = output.default_output_config()?;
    Ok((in_cfg.channels(), out_cfg.channels()))
}

/// Return the subset of `candidates` that both devices support as buffer sizes.
/// Falls back to full candidate list if device reports Unknown.
pub fn supported_buffer_sizes(
    input: &Device,
    output: &Device,
    candidates: &[u32],
) -> Vec<u32> {
    let range = |configs: Result<Vec<SupportedStreamConfigRange>, _>| -> Option<(u32, u32)> {
        let configs = configs.ok()?;
        let mut global_min = u32::MAX;
        let mut global_max = 0u32;
        for cfg in configs {
            match cfg.buffer_size() {
                SupportedBufferSize::Range { min, max } => {
                    global_min = global_min.min(*min);
                    global_max = global_max.max(*max);
                }
                SupportedBufferSize::Unknown => return None,
            }
        }
        if global_max > 0 { Some((global_min, global_max)) } else { None }
    };

    let in_range = range(input.supported_input_configs().map(|i| i.collect()));
    let out_range = range(output.supported_output_configs().map(|i| i.collect()));

    match (in_range, out_range) {
        (Some((in_min, in_max)), Some((out_min, out_max))) => {
            let lo = in_min.max(out_min);
            let hi = in_max.min(out_max);
            candidates.iter().copied().filter(|&s| s >= lo && s <= hi).collect()
        }
        (Some((min, max)), None) | (None, Some((min, max))) => {
            candidates.iter().copied().filter(|&s| s >= min && s <= max).collect()
        }
        (None, None) => candidates.to_vec(),
    }
}

/// Check whether the given buffer size and sample rate are supported by both devices.
pub fn validate_config(
    input: &Device,
    output: &Device,
    buffer_size: u32,
    sample_rate: u32,
) -> Result<(), String> {
    if supported_buffer_sizes(input, output, &[buffer_size]).is_empty() {
        return Err(format!(
            "Buffer size {buffer_size} not supported by selected devices"
        ));
    }
    if supported_sample_rates(input, output, &[sample_rate]).is_empty() {
        return Err(format!(
            "Sample rate {sample_rate} Hz not supported by selected devices"
        ));
    }
    Ok(())
}

/// Return the subset of `candidates` that both devices support as sample rates.
pub fn supported_sample_rates(
    input: &Device,
    output: &Device,
    candidates: &[u32],
) -> Vec<u32> {
    let ranges = |configs: Result<Vec<SupportedStreamConfigRange>, _>| -> Option<Vec<(u32, u32)>> {
        let configs = configs.ok()?;
        Some(
            configs
                .into_iter()
                .map(|c| (c.min_sample_rate(), c.max_sample_rate()))
                .collect(),
        )
    };

    let in_ranges = ranges(input.supported_input_configs().map(|i| i.collect()));
    let out_ranges = ranges(output.supported_output_configs().map(|i| i.collect()));

    let rate_in_ranges = |rate: u32, rs: &[(u32, u32)]| -> bool {
        rs.iter().any(|&(lo, hi)| rate >= lo && rate <= hi)
    };

    candidates
        .iter()
        .copied()
        .filter(|&rate| {
            let in_ok = in_ranges.as_ref().is_none_or(|r| rate_in_ranges(rate, r));
            let out_ok = out_ranges.as_ref().is_none_or(|r| rate_in_ranges(rate, r));
            in_ok && out_ok
        })
        .collect()
}
