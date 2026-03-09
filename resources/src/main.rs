use std::env;
use std::io::{self, Write};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use sysinfo::System;

enum Device {
    CPU,
    GPU,
}

fn format_info(device_type: &Device, usage: i32, temp: i32, mem: i32) -> String {
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

fn jq_select(mut process: Child, jq_path: &str) -> String {
    let stdout = process
        .stdout
        .take()
        .expect("Failed to get stdout from prev.");

    let jq_child = Command::new("jq")
        .args([jq_path])
        .stdin(Stdio::from(stdout))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn jq command.");

    let _ = process.wait().expect("Failed to wait for prev.");

    let result = String::from_utf8(
        jq_child
            .wait_with_output()
            .expect("Failed to get jq output")
            .stdout,
    )
    .expect("Failed to cast stdout to string.");

    String::from(result.trim())
}

fn get_device_temp(jq_path: &str) -> i32 {
    let temp = Command::new("sensors")
        .args(["-j"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lm_sensors command.");

    jq_select(temp, jq_path)
        .parse::<f32>()
        .expect("Failed parsing device temp to float.") as i32
}

fn get_cpu_info(sys: &mut System) -> (i32, i32, i32) {
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu_use = sys
        .cpus()
        .iter()
        .fold(0.0, |acc, cpu| acc + cpu.cpu_usage())
        / sys.cpus().len() as f32;

    (
        cpu_use as i32,
        // "with_entries(select(.key | contains(\"kraken\"))) | .[] | .Coolant.temp1_input",
        get_device_temp(
            "with_entries(select(.key | contains(\"k10temp\"))) | .[] | .Tctl.temp1_input",
        ),
        (100 * sys.used_memory() as i64 / sys.total_memory() as i64) as i32,
    )
}

fn get_gpu_info() -> (i32, i32, i32) {
    let rocm = Command::new("rocm-smi")
        .args(["--showuse", "--showmemuse", "--json"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn rocm-smi command.");
    let info = jq_select(
        rocm,
        ".\"card0\"| .\"GPU use (%)\", .\"GPU Memory Allocated (VRAM%)\"",
    )
    .split("\n")
    .map(|x| {
        x.trim()
            .replace("\"", "")
            .parse::<i32>()
            .expect("Failed casting rocm-smi info to ints.")
    })
    .collect::<Vec<i32>>();

    let (usage, vram) = (info[0], info[1]);

    (
        usage as i32,
        get_device_temp(".\"nvme-pci-0900\".\"Composite\".\"temp1_input\""),
        vram as i32,
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
        sleep(Duration::from_millis(1000));
    }
}
