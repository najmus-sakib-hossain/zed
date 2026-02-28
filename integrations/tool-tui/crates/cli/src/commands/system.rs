//! System information display command

use anyhow::Result;
use owo_colors::OwoColorize;
use std::env;
use sysinfo::System;

use crate::ui::theme::Theme;

pub fn run_system(_theme: &Theme) -> Result<()> {
    // Get system information
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let family = env::consts::FAMILY;

    // Get hostname
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Get username
    let username = whoami::username();

    // Get CPU info
    let cpu_count = num_cpus::get();

    // Get memory info
    let mut sys = System::new_all();
    sys.refresh_all();
    let total_memory = sys.total_memory() / 1024 / 1024; // MB
    let used_memory = sys.used_memory() / 1024 / 1024; // MB

    // Get uptime
    let uptime_secs = System::uptime();
    let uptime_hours = uptime_secs / 3600;
    let uptime_mins = (uptime_secs % 3600) / 60;

    // Select ASCII art based on OS
    let ascii_art = get_os_logo(os);

    println!();
    println!(
        "  {} {}",
        "◆".cyan().bold(),
        format!("DX v{}", env!("CARGO_PKG_VERSION")).white().bold()
    );
    println!();

    // Print user@hostname header
    println!("  {}", format!("{}@{}", username, hostname).cyan().bold());

    // Print ASCII art and info side by side
    let info_lines = vec![
        "─".repeat(username.len() + hostname.len() + 1),
        format!("OS: {} {}", os, arch),
        format!("Family: {}", family),
        format!("CPU: {} cores", cpu_count),
        format!("Memory: {} MB / {} MB", used_memory, total_memory),
        format!("Uptime: {}h {}m", uptime_hours, uptime_mins),
    ];

    let art_lines: Vec<&str> = ascii_art.lines().filter(|l| !l.is_empty()).collect();
    let max_lines = art_lines.len().max(info_lines.len());
    let empty_string = String::new();

    // Calculate max width of ASCII art for alignment
    let max_art_width = art_lines.iter().map(|l| l.len()).max().unwrap_or(0);

    for i in 0..max_lines {
        let art_line = art_lines.get(i).unwrap_or(&"");
        let info_line = info_lines.get(i).unwrap_or(&empty_string);

        if i < art_lines.len() {
            let padding = max_art_width - art_line.len();
            print!("  {}{}", art_line.cyan(), " ".repeat(padding + 2));
        } else {
            print!("  {}", " ".repeat(max_art_width + 2));
        }

        println!("{}", info_line);
    }

    println!();
    Ok(())
}

fn get_os_logo(os: &str) -> &'static str {
    match os {
        "windows" => {
            r#"
        ,.=:^!^!t3Z3z.,
       :tt:::tt333EE3
       Et:::ztt33EEE  @Ee.,      ..,
      ;tt:::tt333EE7 ;EEEEEEttttt33#
     :Et:::zt333EEQ. SEEEEEttttt33QL
     it::::tt333EEF @EEEEEEttttt33F
    ;3=*^```"*4EEV :EEEEEEttttt33@.
    ,.=::::it=., ` @EEEEEEtttz33QF
   ;::::::::zt33)   "4EEEtttji3P*
  :t::::::::tt33.:Z3z..  `` ,..g.
  i::::::::zt33F AEEEtttt::::ztF
 ;:::::::::t33V ;EEEttttt::::t3
 E::::::::zt33L @EEEtttt::::z3F
{3=*^```"*4E3) ;EEEtttt:::::tZ`
             ` :EEEEtttt::::z7
                 "VEzjt:;;z>*`
"#
        }
        "linux" => {
            r#"
        #####
       #######
       ##O#O##
       #######
     ###########
    #############
   ###############
   ################
  #################
#####################
#####################
  #################
"#
        }
        "macos" => {
            r#"
                    'c.
                 ,xNMM.
               .OMMMMo
               OMMM0,
     .;loddo:' loolloddol;.
   cKMMMMMMMMMMNWMMMMMMMMMM0:
 .KMMMMMMMMMMMMMMMMMMMMMMMWd.
 XMMMMMMMMMMMMMMMMMMMMMMMX.
;MMMMMMMMMMMMMMMMMMMMMMMM:
:MMMMMMMMMMMMMMMMMMMMMMMM:
.MMMMMMMMMMMMMMMMMMMMMMMMX.
 kMMMMMMMMMMMMMMMMMMMMMMMMWd.
 .XMMMMMMMMMMMMMMMMMMMMMMMMMMk
  .XMMMMMMMMMMMMMMMMMMMMMMMMK.
    kMMMMMMMMMMMMMMMMMMMMMMd
     ;KMMMMMMMWXXWMMMMMMMk.
       .cooc,.    .,coo:.
"#
        }
        _ => {
            r#"
     _____  __   __
    |  __ \ \ \ / /
    | |  | | \ V / 
    | |  | |  > <  
    | |__| | / . \ 
    |_____/ /_/ \_\
"#
        }
    }
}
