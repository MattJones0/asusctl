use log::{error, warn};
use rog_profiles::{FanCurveProfiles, Profile};
use serde_derive::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[derive(Deserialize, Serialize, Debug)]
pub struct ProfileConfig {
    #[serde(skip)]
    config_path: String,
    /// For restore on boot
    pub active_profile: Profile,
    /// States to restore
    pub fan_curves: Option<FanCurveProfiles>,
}

impl ProfileConfig {
    fn new(config_path: String) -> Self {
        Self {
            config_path,
            active_profile: Profile::Balanced,
            fan_curves: None,
        }
    }

    pub fn set_defaults_and_save(&mut self) {
        self.active_profile = Profile::get_active_profile().unwrap_or(Profile::Balanced);
        if let Ok(res) = FanCurveProfiles::is_supported() {
            if res {
                let curves = FanCurveProfiles::default();
                self.fan_curves = Some(curves);
            }
        }
        self.write();
    }

    pub fn load(config_path: String) -> Self {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_path)
            .unwrap_or_else(|_| panic!("The directory /etc/asusd/ is missing")); // okay to cause panic here
        let mut buf = String::new();
        let mut config;
        if let Ok(read_len) = file.read_to_string(&mut buf) {
            if read_len == 0 {
                config = Self::new(config_path);
                config.set_defaults_and_save();
            } else if let Ok(data) = toml::from_str(&buf) {
                config = data;
                config.config_path = config_path;
            } else {
                warn!(
                    "Could not deserialise {}.\nWill rename to {}-old and recreate config",
                    config_path, config_path
                );
                let cfg_old = config_path.clone() + "-old";
                std::fs::rename(config_path.clone(), cfg_old).unwrap_or_else(|err| {
                    panic!(
                        "Could not rename. Please remove {} then restart service: Error {}",
                        config_path, err
                    )
                });
                config = Self::new(config_path);
                config.set_defaults_and_save();
            }
        } else {
            config = Self::new(config_path);
            config.set_defaults_and_save();
        }
        config
    }

    pub fn read(&mut self) {
        let mut file = OpenOptions::new()
            .read(true)
            .open(&self.config_path)
            .unwrap_or_else(|err| panic!("Error reading {}: {}", self.config_path, err));

        let mut buf = String::new();
        if let Ok(l) = file.read_to_string(&mut buf) {
            if l == 0 {
                warn!("File is empty {}", self.config_path);
            } else {
                let mut data: ProfileConfig = toml::from_str(&buf)
                    .unwrap_or_else(|_| panic!("Could not deserialise {}", self.config_path));
                // copy over serde skipped values
                data.config_path = self.config_path.clone();
                *self = data;
            }
        }
    }

    pub fn write(&self) {
        let mut file = File::create(&self.config_path).expect("Couldn't overwrite config");
        let data = toml::to_string(self).expect("Parse config to toml failed");
        file.write_all(data.as_bytes())
            .unwrap_or_else(|err| error!("Could not write config: {}", err));
    }
}
