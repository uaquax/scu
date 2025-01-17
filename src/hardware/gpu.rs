use crate::pci_ids::PciIdentifiers;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct GPUInfo {
    pub model: String,
    pub driver: String,
    pub temperature: f32,
}

fn lower(_str: &str) -> String {
    let mut result = String::from(_str);
    result.make_ascii_lowercase();
    result
}

pub fn get_info() -> Option<BTreeMap<u8, GPUInfo>> {
    let mut result: BTreeMap<u8, GPUInfo> = BTreeMap::new();

    if !Path::new("/sys/bus/pci/devices").exists() {
        return None;
    }

    let drm_content = fs::read_dir("/sys/bus/pci/devices");
    if drm_content.is_err() {
        return None;
    }

    for entry in drm_content.unwrap() {
        let entry = entry.unwrap().path();
        let entry = entry.to_str().unwrap();
        if fs::metadata(format!("{}/class", entry)).is_err()
            || !fs::read_to_string(format!("{}/class", entry))
                .unwrap()
                .starts_with("0x03")
        {
            continue;
        }
        let uevent_path = format!("{}/uevent", entry);
        let mut hwmon_path = format!("{}/hwmon", entry);
        if fs::metadata(hwmon_path.clone()).is_ok() {
            fs::read_dir(hwmon_path.clone())
                .unwrap()
                .for_each(|hwentry| {
                    let hwentry = hwentry.unwrap().file_name();
                    let hwentry = hwentry.to_str().unwrap();
                    if regex::Regex::new("hwmon[[:digit:]]")
                        .unwrap()
                        .is_match(hwentry)
                    {
                        hwmon_path = format!("{}/hwmon/{}", entry, hwentry);
                    }
                });
        }
        let temperature_path = format!("{}/temp1_input", hwmon_path);
        if Path::new(uevent_path.as_str()).exists() {
            let mut vendor = String::new();
            let mut model = String::new();
            let mut driver = String::from("Unknown");
            let mut temperature: f32 = 0.0;
            if Path::new(&temperature_path).exists() {
                temperature = match fs::read_to_string(temperature_path) {
                    Ok(content) => content.trim().parse::<u32>().unwrap() as f32 / 1000.0,
                    Err(_) => 0.0,
                };
            }
            for line in fs::read_to_string(uevent_path).unwrap().split('\n') {
                if line.starts_with("DRIVER") {
                    driver = line.split("DRIVER=").nth(1).unwrap().to_string();
                } else if line.starts_with("PCI_ID") {
                    let pci_id = line.split("PCI_ID=").nth(1).unwrap().to_string();
                    vendor = String::from(match pci_id.split(':').next().unwrap() {
                        "10DE" => "NVIDIA",
                        "1002" => "AMD",
                        "8086" => "Intel",
                        "1a03" => "ASPEED",
                        _ => "Unknown",
                    });
                    model = pci_id.to_string().to_ascii_lowercase();
                } else if line.starts_with("PCI_SUBSYS_ID") {
                    if !line.is_empty() {
                        model.push(' ');
                    }
                    model.push_str(
                        line.split("PCI_SUBSYS_ID=")
                            .nth(1)
                            .unwrap()
                            .to_string()
                            .as_str(),
                    );
                }
            }
            if !model.is_empty() {
                if PciIdentifiers::contains_key(lower(model.as_str()).as_str()) {
                    let id = lower(model.as_str());
                    let name = PciIdentifiers::get(id.as_str());
                    if name.is_some() {
                        model = name.unwrap().to_string();
                    }
                } else if model.contains(' ')
                    && PciIdentifiers::contains_key(
                        lower(model.split(' ').next().unwrap()).as_str(),
                    )
                {
                    let id = lower(model.split(' ').next().unwrap());
                    let name = PciIdentifiers::get(id.as_str());
                    if name.is_some() {
                        model = name.unwrap().to_string();
                    }
                }
                if model.contains('[') && model.contains(']') {
                    model = model
                        .split('[')
                        .nth(1)
                        .unwrap()
                        .split(']')
                        .next()
                        .unwrap()
                        .to_string();
                }
                if model.contains(&vendor) {
                    model = model.replace(&vendor, "");
                }
                result.insert(
                    result.len() as u8 + 1,
                    GPUInfo {
                        model: if !vendor.is_empty() {
                            format!("{} ", vendor)
                        } else {
                            "".to_string()
                        } + model.trim(),
                        driver,
                        temperature,
                    },
                );
            }
        }
    }

    if !result.is_empty() {
        return Some(result);
    }

    None
}
