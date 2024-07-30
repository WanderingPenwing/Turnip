use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use chrono::Local;
use sysinfo::System;
use chrono::Timelike;
use sysinfo::Cpu;
//use reqwest::Client;

const BATTERY_STATE : [&str; 5] = [" ", " ", " ", " ", " "];
const CPU_STATE : [&str; 5] = [" ", " ", " ", " ", " "];

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
		
		display(format!("| {} | {} | {} | {} ", mem_str, cpu_str, battery_str, time_str));
		
		
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
	
	return format!(" {:.1}  ", used_memory_gb);
}


//fn measure_bandwidth(url: &str, duration_secs: u64) -> reqwest::Result<f64> {
//    let client = Client::new();
//    let mut response = client.get(url).send().await?;
//    
//    let start = Instant::now();
//    let mut total_bytes = 0;
//    let mut buffer = vec![0; 64 * 1024]; // 64 KB buffer
//
//    while start.elapsed().as_secs() < duration_secs {
//        match response.copy_to(&mut buffer).await {
//            Ok(n) => total_bytes += n,
//            Err(e) => return Err(e.into()),
//        }
//    }
//
//    let elapsed = start.elapsed().as_secs_f64();
//    let bandwidth = total_bytes as f64 / elapsed; // Bytes per second
//    println!("Downloaded {:.2} Mb/s measured in {:.2} seconds", bandwidth / (1024.0 * 1024.0), elapsed);
//
//    Ok(bandwidth / (1024.0 * 1024.0))
//}
//
//fn internet_display() -> String {
//	match measure_bandwidth("https://example.com", 5).await {
//		Ok(bandwidth) => {
//			return format!("{} Mb/s", bandwidth)
//		}
//        Err(e) => {
//			eprintln!("Error measuring bandwidth: {}", e);
//			return " ".to_string()
//		}
//    }
//}