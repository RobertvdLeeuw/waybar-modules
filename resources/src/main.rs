use std::env;
use std::fs;
use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

use sensors::Sensors;
use sysinfo::System;

enum Device {
    CPU,
    GPU,
}

fn format_info(device_type: &Device, usage: u8, temp: u8, mem: u8) -> String {
    let icon = match device_type {
        Device::CPU => "",
        Device::GPU => "󰢮",
    };

    // Careful with the spaces, they're non-standard length spacers but this isn't displayed in
    // mono.
    let body = format!(
        "{icon}  <b>{usage}%</b> {spacer}{temp}°C <b>|</b> {mem}%   ",
        icon = icon,
        usage = usage,
        temp = temp,
        mem = mem,
        spacer = if usage >= 10 { "" } else { " " }
    );

    format!(
        "{{ \"text\": \"{}\", \
         \"tooltip\": \"\", \
         \"class\": \"\" }} ",
        body
    )
}

fn get_device_temp(chip: &str, feature: &str, subfeature: &str) -> u8 {
    Sensors::new()
        .into_iter()
        .find(|c| {
            c.get_name()
                .expect("Unable to get sensor name")
                .starts_with(chip)
        })
        .unwrap_or_else(|| panic!("Unable to find device: '{}'", chip))
        .into_iter()
        .find(|f| f.get_label().expect("Unable to get feature name") == feature)
        .unwrap_or_else(|| panic!("Unable to find device feature: '{}'", feature))
        .into_iter()
        .find(|sf| sf.name() == subfeature)
        .unwrap_or_else(|| panic!("Unable to find device subfeature: '{}'", subfeature))
        .get_value()
        .expect("Unable to retrieve device temperature value.") as u8
}

fn get_cpu_info(sys: &mut System) -> (u8, u8, u8) {
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu_use = sys
        .cpus()
        .iter()
        .fold(0.0, |acc, cpu| acc + cpu.cpu_usage())
        / sys.cpus().len() as f32;

    (
        cpu_use as u8,
        get_device_temp("k10temp", "Tctl", "temp1_input"),
        (100 * sys.used_memory() / sys.total_memory()) as u8,
    )
}

fn get_gpu_info() -> (u8, u8, u8) {
    let usage = fs::read_to_string("/sys/class/drm/card1/device/gpu_busy_percent")
        .expect("Failed to read GPU usage.")
        .strip_suffix("\n")
        .expect("Failed to remove newline (GPU usage).")
        .parse::<u8>()
        .expect("Failed to cast GPU usage to int.");

    let vram_total = fs::read_to_string("/sys/class/drm/card1/device/mem_info_vram_total")
        .expect("Failed to read GPU VRAM total.")
        .strip_suffix("\n")
        .expect("Failed to remove newline (GPU total VRAM).")
        .parse::<u64>()
        .expect("Failed to cast GPU VRAM total to int.");

    let vram_used = fs::read_to_string("/sys/class/drm/card1/device/mem_info_vram_used")
        .expect("Failed to read GPU VRAM usage.")
        .strip_suffix("\n")
        .expect("Failed to remove newline (GPU used VRAM).")
        .parse::<u64>()
        .expect("Failed to cast GPU VRAM usage to int.");

    (
        usage,
        get_device_temp("nvme-pci-0900", "Composite", "temp1_input"),
        (vram_used * 100 / vram_total) as u8,
    )
}

fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let args = env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        2,
        "Wrong number of args (expected 2, got {}), 'resources <device>'",
        args.len()
    );

    let device_type = match args[1].as_str() {
        "CPU" => Device::CPU,
        "GPU" => Device::GPU,
        _ => panic!("Unsupported device, expected from (CPU, GPU)"),
    };

    loop {
        let (usage, temp, mem) = match device_type {
            Device::CPU => get_cpu_info(&mut sys),
            Device::GPU => get_gpu_info(),
        };

        writeln!(handle, "{}", format_info(&device_type, usage, temp, mem)).unwrap();

        let _ = handle.flush();
        sleep(Duration::from_millis(500));
    }
}
