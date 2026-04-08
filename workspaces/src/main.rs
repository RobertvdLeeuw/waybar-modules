use swayipc::{Connection, Workspace};

use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

fn build_workspace_repr(workspace_ids: &[u8], conn: &mut Connection) -> String {
    // num => "<x x>    "

    let mut data = workspace_ids
        .into_iter()
        .map(|&ws| (ws, "".to_string()))
        .collect::<HashMap<u8, String>>();

    for ws in conn
        .get_workspaces()
        .expect("Unable to get Sway workspaces")
    {
        let ws_id = ws.num as u8;
        if !workspace_ids.contains(&ws_id) {
            continue;
        }

        if ws.visible {
            data.insert(ws_id, " ".to_string());
        } else {
            data.insert(ws_id, format_as_icons(&ws));
        }
    }

    workspace_ids
        .into_iter()
        .map(|ws_id| data[ws_id].clone())
        .collect::<Vec<String>>()
        .join(" ")
}

fn format_as_icons(workspace: &Workspace) -> String {
    let iconized = workspace
        .representation
        .clone()
        .expect("Failed to get workspace repr.")
        .replace("V[", "")
        .replace("H[", "")
        .replace("]", "")
        .split(" ")
        .map(|app: &str| {
            match app.to_lowercase().as_str() {
                "spotify" => " ",
                "alacritty" => " ",
                "brave-browser" => " ",
                "firefox" => " ",
                "libreoffice-calc" => "󰧷 ",
                "steam" => " ",
                "com.ktechpit.whatsie" => " ",
                "chimp" => " ",
                "discord" => " ",
                "gimp" | "com.github.PintaProject.Pinta" => "󰽉 ",
                "org.pulseaudio.pavucontrol" => " ",

                _ if app.contains("steam_app") => "󰊗 ", // NOTE: This only works for a handful of games.
                _ => "󱗼 ",
            }
            .to_string()
        })
        .collect::<Vec<String>>()
        .join(" ");

    format!("❬ {}❭", iconized)
}

fn build_json_repr(workspace_ids: &[u8], conn: &mut Connection) -> String {
    format!(
        "{{ \"text\": \"{}\", \
         \"tooltip\": \"\", \
         \"class\": \"\" }} ",
        build_workspace_repr(workspace_ids, conn)
    )
}

fn validate_workspace_args(workspace_args: &[String]) -> Vec<u8> {
    for ws in workspace_args {
        if !ws.parse::<u8>().is_ok() {
            eprintln!("Invalid workspace arg (expected number): '{}'", ws);
            exit(1);
        }
    }

    workspace_args
        .into_iter()
        .map(|ws| ws.parse::<u8>().expect("Invalid arg format, expected u8."))
        .collect()
}

fn main() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let mut conn =
        Connection::new().expect("Failed to connect to Sway IPC - are you running Sway?");

    let args = env::args().collect::<Vec<String>>();

    if args.len() == 1 {
        eprintln!("Missing workspace numbers (what workspaces should this widget watch?)");
        exit(1)
    }

    let workspace_ids = validate_workspace_args(&args[1..]);

    loop {
        writeln!(handle, "{}", build_json_repr(&workspace_ids, &mut conn))
            .expect("Error sending JSON.");
        let _ = handle.flush();

        sleep(Duration::from_millis(100));
    }
}
