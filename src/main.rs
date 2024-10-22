use std::process::Command;
use tokio::time::{sleep, Duration};
use chrono::Local;
use sysinfo::System;
use chrono::Timelike;
use sysinfo::Cpu;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use sysinfo::Networks;
use std::sync::Arc;
use battery::Manager;
use tokio::sync::{Notify};
use weer_api::{*, chrono::{Utc, TimeZone}};
use std::collections::HashMap;

const BATTERY_STATE : [&str; 5] = ["", "", "", "", ""];
const CPU_STATE : [&str; 5] = ["", "", "", "", ""];
const WEATHER_STATE : [&str; 10] = [" ", " ", "", "", "", "", "", "", " ", ""];
								//   0    1    2    3    4    5    6    7    8    9  

fn weather_code_to_icon(weather_code: u32) -> String {
	let weather_states = vec![
        (1000, 0), (1003, 3), (1006, 3), (1009, 1), (1030, 4), (1063, 2), (1066, 2), (1069, 2),
        (1072, 2), (1087, 7), (1114, 8), (1117, 8), (1135, 4), (1147, 4), (1150, 2), (1153, 2),
        (1168, 5), (1171, 5), (1180, 2), (1183, 2), (1186, 5), (1189, 5), (1192, 6), (1195, 6),
        (1198, 5), (1201, 6), (1204, 1), (1207, 6), (1210, 8), (1213, 8), (1216, 8), (1219, 8),
        (1222, 8), (1225, 8), (1237, 8), (1240, 2), (1243, 5), (1246, 6), (1249, 2), (1252, 6),
        (1255, 8), (1258, 8), (1261, 8), (1264, 8), (1273, 7), (1276, 7), (1279, 7), (1282, 7)
    ];

    let weather_map: HashMap<u32, usize> = weather_states.into_iter().collect();
    
	let state = weather_map.get(&weather_code).unwrap_or(&(9 as usize));

	return WEATHER_STATE[*state].to_string()
}

#[derive(PartialEq)]
enum Connection {
	Wired,
	Wifi,
	None,
}

#[tokio::main]
async fn main() {
	let manager = Manager::new().expect("could not create battery manager");
	let mut sys = System::new();
	
	let mut networks = Networks::new();
	let mut battery = manager.batteries().expect("could not fetch battery").next().expect("there should be a battery").expect("the battery should be okay");
	
	let mut networks_2 = Networks::new();
    let mut battery_2 = manager.batteries().expect("could not fetch battery").next().expect("there should be a battery").expect("the battery should be okay");

	let weather_client = Client::new("2e6ff5190261446bbba80937241810", true);

	let weather_result = weather_client.forecast()
	    .query(Query::City("Toulouse".to_string()))
        .days(2)
        .dt(Utc.with_ymd_and_hms(2022, 08, 21, 0, 0, 0).unwrap())
        .lang(Language::French)
        .call();

    let weather_str : String = if let Ok(ref forecast) = weather_result {
    	println!("code : {}", forecast.current.condition.code);
    	weather_code_to_icon(forecast.current.condition.code)
    } else {
    	" ".into()
    };

	let notify = Arc::new(Notify::new());
	let notify_cloned: Arc<Notify> = Arc::clone(&notify);

	tokio::spawn(async move {	
		loop {
			let battery_charging = battery_2.time_to_empty().is_none();
			let connection_type = get_connection(&networks_2);
		
			if battery_2.state_of_charge().value == 1.0 && connection_type != Connection::None {
				sleep(Duration::from_secs(20)).await;
			} else {
				sleep(Duration::from_secs(2)).await;
			}
			
			battery_2.refresh().expect("could not refresh battery");
			if battery_2.time_to_empty().is_none() != battery_charging && battery_2.state_of_charge().value != 1.0 {
				notify_cloned.notify_one();
				continue
			}

			networks_2.refresh_list();
			if connection_type != get_connection(&networks_2) {
				notify_cloned.notify_one();
			}
		}
	});

	loop {
		let time_str = time_display();
		
		battery.refresh().expect("could not refresh battery");
		let battery_str = battery_display(&battery);
		
		sys.refresh_cpu();
		let cpu_str = cpu_display(sys.cpus());
		
		sys.refresh_memory();
		let mem_str = mem_display(sys.used_memory());
		
		networks.refresh_list();
		let internet_str = internet_display(&networks);
		
		display(format!("| {} | {} | {} | {} | {} | {} ", weather_str, internet_str, mem_str, cpu_str, battery_str, time_str));

		let sleep_or_notify = sleep(Duration::from_secs((60 - Local::now().second()).into()));
		tokio::select! {
			_ = sleep_or_notify => {}
			_ = notify.notified() => {}
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

fn internet_display(networks: &Networks) -> String {
	let connection_type = get_connection(networks);
	
	match connection_type {
		Connection::Wired => " ".to_string(),
		Connection::None =>  " ".to_string(),
		Connection::Wifi => {
			let strength = get_wifi_strength().unwrap_or(0.0);
			format!("  {:.0}%", strength)
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
