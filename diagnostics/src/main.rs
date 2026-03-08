use std::env;
use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

enum HostType {
    Desktop,
    Laptop,
}

fn bluetooth_check_battery_low(mac: &str) -> bool {
    // upower -dump
    // Split on "Device"
    // Match mac to device
    // If not found/connected, return false (ignore unconnected)
    // Get "percentage"
    // extract value
    // Convert to int
    // Return val < 20
    true
}

// TODO: Special spaces like in resources

fn diag_desktop() -> Vec<String> {
    let mut warnings = Vec::new();

    // Headset
    if bluetooth_check_battery_low("80:C3:BA:65:D6:34") {
        warnings.push("󰋋 󱊡".to_string())
    }

    // Mouse
    if bluetooth_check_battery_low("D6:88:58:DA:5E:6D") {
        warnings.push(" 󱊡".to_string())
    }

    // Controller
    if bluetooth_check_battery_low("A4:AE:12:C1:CE:44") {
        warnings.push("󰖺 󱊡".to_string())
    }

    warnings
}

fn check_any_disk_full() -> bool {
    // df -h --output=source,pcent,target | grep '^/' | grep -v '/boot' | awk 'int($2) > 80' | grep -q .
    true
}

fn check_homelab_ping() -> bool {
    true
    // curl -s -o /dev/null -w "%{http_code}" https://rvdlserver.nl/}
}

fn diag_laptop() -> Vec<String> {
    let mut warnings = Vec::new();

    // Laptop battery < 20%

    warnings
}

fn diag_common() -> Vec<String> {
    // Check disk space
    let mut warnings = Vec::new();

    if check_any_disk_full() {
        warnings.push("󰇑 ".to_string());
    }
    // Homelab unresponsive to ping
    if !check_homelab_ping() {
        warnings.push("󰧠 ".to_string());
    }

    warnings
}

fn format_info(warnings: Vec<String>) -> String {
    format!(
        "{{ \"text\": \"{}\", \
         \"tooltip\": \"\", \
         \"class\": \"\" }} ",
        warnings.join(" | ")
    )
}

fn main() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let args = env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        2,
        "Wrong number of args (expected 2, got {}), 'resources <device>'",
        args.len()
    );

    let host_type = match args[1].as_str() {
        "desktop" => HostType::Desktop,
        "laptop" => HostType::Laptop,
        _ => panic!("Unsupported device, expected from (desktop, laptop)"),
    };

    loop {
        let warnings = match host_type {
            HostType::Desktop => diag_desktop(),
            HostType::Laptop => diag_laptop(),
        };

        writeln!(
            handle,
            "{}",
            format_info([warnings, diag_common()].concat())
        )
        .unwrap();

        let _ = handle.flush();
        sleep(Duration::from_millis(1000));
    }
}
