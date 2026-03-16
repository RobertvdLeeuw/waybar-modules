use std::env;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

enum HostType {
    Desktop,
    Laptop,
}

fn bluetooth_check_battery_low(mac: &str) -> bool {
    let upower = Command::new("upower")
        .args(["-d"])
        .stdout(Stdio::piped())
        .spawn();

    let Ok(mut upower) = upower else {
        return false; // If upower fails, assume no warning needed
    };

    let stdout = upower
        .stdout
        .take()
        .expect("Failed to get stdout from upower");

    // Search for the MAC address and extract percentage
    let grep = Command::new("grep")
        .args(["-A", "20", mac])
        .stdin(Stdio::from(stdout))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn grep");

    let _ = upower.wait().expect("Failed to wait for upower");

    let grep_out = grep.wait_with_output().expect("Failed to get grep output");
    let output = String::from_utf8_lossy(&grep_out.stdout);

    // Parse percentage line
    for line in output.lines() {
        if !line.contains("percentage:") {
            continue;
        }

        let Some(pct_str) = line.split_whitespace().nth(1) else {
            continue;
        };
        let Ok(pct) = pct_str.trim_end_matches('%').parse::<i32>() else {
            continue;
        };

        return pct < 20;
    }

    false // Device not found or not connected
}

fn check_lekkerspelen_live() -> bool {
    let yt_dlp = Command::new("yt-dlp")
        .args([
            "--print",
            "%(is_live)s",
            "https://www.youtube.com/@lekkerspelen/live",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let Ok(yt_dlp) = yt_dlp else {
        return false;
    };

    let output = yt_dlp
        .wait_with_output()
        .expect("Failed to get yt-dlp output");
    output.status.success()
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
    let pct_threshold = 80;

    let mut df = Command::new("df")
        .args(["-h", "--output=source,pcent,target"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn df");

    let stdout = df.stdout.take().expect("Failed to get df stdout");

    let mut grep1 = Command::new("grep")
        .args(["^/"])
        .stdin(Stdio::from(stdout))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn grep");

    let _ = df.wait().expect("Failed to wait for df");

    let grep1_out = grep1.stdout.take().expect("Failed to get grep1 stdout");

    let grep2 = Command::new("grep")
        .args(["-v", "/boot"])
        .stdin(Stdio::from(grep1_out))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn grep2");

    let _ = grep1.wait().expect("Failed to wait for grep1");

    let grep2_out = grep2
        .wait_with_output()
        .expect("Failed to get grep2 output");
    let output = String::from_utf8_lossy(&grep2_out.stdout);

    // Parse percentage from each line
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let Ok(pct) = parts[1].trim_end_matches('%').parse::<i32>() else {
            continue;
        };

        if pct > pct_threshold {
            return true;
        }
    }

    false
}

fn check_internet_connection() -> bool {
    let ping = Command::new("ping")
        .args(["-c", "1", "-W", "3", "google.com"])
        .stdout(Stdio::piped())
        .spawn();

    let Ok(ping) = ping else {
        return false; // Network issues, assume down
    };

    let output = ping.wait_with_output().expect("Failed to get ping output");
    output.status.success()
}

fn check_homelab_ping() -> bool {
    let curl = Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "https://rvdlserver.nl/",
        ])
        .stdout(Stdio::piped())
        .spawn();

    let Ok(curl) = curl else {
        return false; // Network issues, assume down
    };

    let output = curl.wait_with_output().expect("Failed to get curl output");
    let status_code = String::from_utf8_lossy(&output.stdout);

    status_code.trim() == "200"
}

fn diag_laptop() -> Vec<String> {
    let mut warnings = Vec::new();

    warnings
}

fn diag_common() -> Vec<String> {
    // Check disk space
    let mut warnings = Vec::new();

    if check_any_disk_full() {
        warnings.push(" ".to_string());
    }
    // Homelab unresponsive to ping
    if !check_internet_connection() {
        warnings.push("󰖩 ".to_string());
    } else if !check_homelab_ping() {
        // warnings.push("󰧠 ".to_string());
    }

    if check_lekkerspelen_live() {
        warnings.push("󱈔 ".to_string());
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
        "Wrong number of args (expected 2, got {}), 'diagnostics <device>'",
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
