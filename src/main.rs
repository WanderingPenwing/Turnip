use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use sysinfo::System;
use chrono::Timelike;
use sysinfo::Cpu;
use std::net::TcpStream;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

const BATTERY_STATE : [&str; 5] = [" ", " ", " ", " ", " "];
const CPU_STATE : [&str; 5] = [" ", " ", " ", " ", " "];

//enum Connection {
//	Wired,
//	Wifi,
//	None,
//}

fn main() {
	let manager = battery::Manager::new().expect("could not create battery manager");
	let mut sys = System::new();
	let mut battery = manager.batteries().expect("could not fetch battery").next().expect("there should be a battery").expect("the battery should be okay");
		
	loop {
		let time_str = time_display();

		battery.refresh().expect("could not refresh battery");
		let battery_charging = battery.time_to_empty().is_none();
		let battery_str = battery_display(&battery);
		
		sys.refresh_cpu();
		let cpu_str = cpu_display(sys.cpus());
		
		sys.refresh_memory();
		let mem_str = mem_display(sys.used_memory());
		
		let internet_str = internet_display();
		
		display(format!("| {} | {} | {} | {} | {} ", internet_str, mem_str, cpu_str, battery_str, time_str));
		
		
		let mut event = false;
		sleep(Duration::from_secs(1));
		
		while Local::now().second() != 0 && !event {
			sleep(Duration::from_secs(1));
			
			battery.refresh().expect("could not refresh battery");
			if battery.time_to_empty().is_none() != battery_charging && battery.state_of_charge().value != 1.0 {
				event = true;
			}
		}
	}
}

fn display(status: String) {
	let output = Command::new("xsetroot")
		.arg("-name")
		.arg(&status)
		.output()
		.expect("Failed to execute command");

	if !output.status.success() {
		eprintln!(
			"Command failed with error: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}
}


fn time_display() -> String {
	let now = Local::now();
	now.format("%H:%M").to_string()
}


fn battery_display(battery: &battery::Battery) -> String {
	if battery.state_of_charge().value == 1.0 {
		return "   ".to_string()
	}
	
	let state = BATTERY_STATE[(battery.state_of_charge().value * 5.0) as usize];
	
	if battery.time_to_empty().is_none() {
		return format!(" {}", state)
	}
	return format!(" {} ", state)
}


fn cpu_display(cpus : &[Cpu]) -> String {
	let mut total_usage: f32 = 0.0;
	let n_cpus = cpus.len() as f32;
	
	for cpu in cpus {
		total_usage += cpu.cpu_usage();
	}
	
	let cpu_usage = (total_usage / n_cpus) as usize;
	
	let mut cpu_temp : f32 = 0.0;
	
	let components = sysinfo::Components::new_with_refreshed_list();
	for component in &components {
		if component.label() == "k10temp Tctl" {
			cpu_temp = component.temperature();
		}
	}
	
	let cpu_state = match cpu_temp {
	    t if t < 45.0 => CPU_STATE[0], 
	    t if t < 55.0 => CPU_STATE[1], 
	    t if t < 65.0 => CPU_STATE[2], 
	    t if t < 75.0 => CPU_STATE[3], 
	    _ => CPU_STATE[4],
	};
	
	let cpu_space = if cpu_usage < 10 {
		" "
	} else {
		""
	};
	return format!("{}{} {}", cpu_space, cpu_usage, cpu_state);
}


fn mem_display(mem_usage: u64) -> String {
	let used_memory_gb = mem_usage as f64 / 1024.0 / 1024.0 / 1024.0;
	
	return format!(" {:.1} ", used_memory_gb);
}

fn is_connected_to_internet() -> bool {
    match TcpStream::connect_timeout(&"8.8.8.8:53".parse().unwrap(), Duration::from_secs(2)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

// Check if connected via Ethernet
fn is_ethernet() -> bool {
    let output = Command::new("nmcli")
        .arg("device")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    let status = String::from_utf8_lossy(&output.stdout);
    status.contains("ethernet")
}

// Get Wi-Fi signal strength
fn get_wifi_strength() -> Option<f32> {
    let file_path = "/proc/net/wireless";

    // Check if the file exists
    if !Path::new(file_path).exists() {
        return None;
    }

    // Read the file line by line
    let file = fs::File::open(file_path).expect("Failed to open file");
    let reader = io::BufReader::new(file);
    
    // Parse the file content
    for (index, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line");
        if index == 2 { // The third line in the file
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() > 2 {
                if let Ok(signal_dbm) = fields[2].parse::<f32>() {
                    // Convert dBm to percentage using the same formula as awk
                    let signal_strength_percentage = (signal_dbm * 10.0 / 7.0).clamp(0.0, 100.0);
                    return Some(signal_strength_percentage);
                }
            }
        }
    }

    None
} 

fn internet_display() -> String {
	if is_connected_to_internet() {
        if is_ethernet() {
            return " ".to_string();
        } else {
            match get_wifi_strength() {
                Some(strength) => return format!("  {:.0}%", strength),
                None => return " ? ".to_string(),
            }
        }
    } else {
        return " ".to_string();
    }
}