use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use sysinfo::System;
use chrono::Timelike;
use sysinfo::Cpu;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use sysinfo::Networks;

const BATTERY_STATE : [&str; 5] = ["", "", "", "", ""];
const CPU_STATE : [&str; 5] = ["", "", "", "", ""];

#[derive(PartialEq)]
enum Connection {
	Wired,
	Wifi,
	None,
}

fn main() {
	let manager = battery::Manager::new().expect("could not create battery manager");
	let mut sys = System::new();
	let mut networks = Networks::new();
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
		
		networks.refresh_list();
		let (internet_str, connection_type) = internet_display(&networks);
		
		display(format!("| {} | {} | {} | {} | {} ", internet_str, mem_str, cpu_str, battery_str, time_str));
		
		let mut event = false;
		sleep(Duration::from_secs(1));
		
		while Local::now().second() != 0 && !event {
			sleep(Duration::from_secs(1));
			
			battery.refresh().expect("could not refresh battery");
			if battery.time_to_empty().is_none() != battery_charging && battery.state_of_charge().value != 1.0 {
				event = true;
			}
			networks.refresh_list();
			if connection_type != get_connection(&networks) {
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
		return "".to_string()
	}
	
	let state = BATTERY_STATE[(battery.state_of_charge().value * 5.0) as usize];
	let charge_status = if battery.time_to_empty().is_none() { " " } else { "" };
	format!("{}{} ", charge_status, state)
}


fn cpu_display(cpus: &[Cpu]) -> String {
	let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
	let cpu_usage = (total_usage / cpus.len() as f32) as usize;

	let components = sysinfo::Components::new_with_refreshed_list();
	let cpu_temp = components.iter()
		.find(|c| c.label() == "k10temp Tctl")
		.map_or(0.0, |c| c.temperature());

	
	let cpu_state = match cpu_temp {
		t if t < 45.0 => CPU_STATE[0], 
		t if t < 55.0 => CPU_STATE[1], 
		t if t < 65.0 => CPU_STATE[2], 
		t if t < 75.0 => CPU_STATE[3], 
		_ => CPU_STATE[4],
	};
	
	return format!("{} {}", cpu_usage, cpu_state);
}


fn mem_display(mem_usage: u64) -> String {
	let used_memory_gb = mem_usage as f64 / 1024.0 / 1024.0 / 1024.0;
	
	return format!("{:.1} ", used_memory_gb);
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

fn internet_display(networks: &Networks) -> (String, Connection) {
	let connection_type = get_connection(networks);
	
	match connection_type {
		Connection::Wired => (" ".to_string(), Connection::Wired),
		Connection::None =>  (" ".to_string(), Connection::None),
		Connection::Wifi => {
			let strength = get_wifi_strength().unwrap_or(0.0);
			(format!("  {:.0}%", strength), Connection::Wifi)
		}
	}
}

fn get_connection(networks: &Networks) -> Connection {
	if networks.len() <= 1 {
		return Connection::None
	}
	
	for (interface_name, _network) in networks {
		if interface_name == "wlp1s0" {
			return Connection::Wifi
		}
	}
	
	return Connection::Wired
}
