// thread 'main' panicked at src/main.rs:76:18:
// : Os { code: 11, kind: WouldBlock, message: "Resource temporarily unavailable" }
// note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::process::{exit, Command, Stdio};
use std::string::String;
use std::thread::sleep;
use std::time::Duration;
use std::vec;

fn build_workspace_reprs() -> HashMap<String, String> {
    let mut data = HashMap::new();
    // Nonempty first
    // let open_workspaces: Vec<(String, bool, String)>;
    let open_workspaces = get_open_workspaces();

    for (open_ws_id, visible, repr) in open_workspaces {
        data.insert(open_ws_id, format_as_icon(&repr, visible));
    }

    // Then pad with empty
    for ws_id in (1..=4).chain(11..=14).chain(20..=24) {
        data.entry(ws_id.to_string())
            .or_insert_with(|| "".to_string());
    }

    data
}

fn format_as_icon(repr: &str, visible: bool) -> String {
    if visible {
        return " ".to_string();
    }

    let formatted = repr
        .replace("V[", "")
        .replace("H[", "")
        .replace("]", "")
        .split(" ")
        .map(|app| {
            match app.to_lowercase().as_str() {
                "spotify" => " ",
                "alacritty" => " ",
                "brave-browser" => " ",
                "firefox" => " ",
                "libreoffice-calc" => "󰧷 ",
                "steam" => " ",
                "wasistlos" => " ",
                _ if app.contains("steam_app") => "󰊗 ", // TODO: This only works for a handful of games.
                // TEST NAMES FROM BELOW
                "teams" => "󰊻 ",
                // "Blender" => "",
                // "Discord" => "",
                _ => "󱗼 ",
            }
            .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!("❬  {}❭", formatted)
}

fn get_open_workspaces() -> Vec<(String, bool, String)> {
    let mut open_workspaces = Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn swaymsg command");

    // Take ownership of stdout before spawning jq
    let stdout = open_workspaces
        .stdout
        .take()
        .expect("Failed to get stdout from swaymsg");

    // Use the taken stdout for jq's stdin
    let jq_child = Command::new("jq")
        .args(["-r", ".[] | \"\\(.name) \\(.visible) \\(.representation)\""])
        .stdin(Stdio::from(stdout))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn jq command");

    // Now we can still wait on open_workspaces
    let _ = open_workspaces.wait().expect("Failed to wait for swaymsg");

    // Then get output from jq
    let open_workspace_ids = jq_child
        .wait_with_output()
        .expect("Failed to get jq output");

    String::from_utf8(open_workspace_ids.stdout)
        .unwrap()
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let parts: Vec<&str> = s.split(' ').collect();
            (
                parts[0].to_string(),
                parts[1] == "true",
                parts[2..parts.len()].join(" ").to_string(),
            )
        })
        .collect::<Vec<(String, bool, String)>>()
}

fn get_corresponding_workspaces(screen_name: &str) -> Vec<&str> {
    match screen_name {
        "HDMI-A-1" => vec!["1", "2", "3", "4"],
        "DP-1" => vec!["11", "12", "13", "14"],
        "DP-3" => vec!["21", "22", "23", "24"],
        _ => panic!("Unrecognized screen name: {}", screen_name),
    }
}

fn build_json_repr(screen_name: &str) -> String {
    let workspace_reprs = build_workspace_reprs();

    let text = get_corresponding_workspaces(screen_name)
        .into_iter()
        .map(|ws| workspace_reprs[ws].clone())
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "{{ \"text\": \"{}\", \
         \"tooltip\": \"\", \
         \"class\": \"\" }} ",
        text
    )
}

fn main() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let args = env::args().collect::<Vec<_>>();

    if args.len() != 2 {
        println!(
            "Wrong number of args (expected 2, got {}), 'workspaces <filename>'",
            args.len()
        );
        exit(1)
    }

    let output_screen = args[1].clone();

    loop {
        writeln!(handle, "{}", build_json_repr(&output_screen)).expect("Error constructing JSON.");
        let _ = handle.flush();

        sleep(Duration::from_millis(100));
    }
}
