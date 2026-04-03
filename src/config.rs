pub mod config {
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;
    use directories::ProjectDirs;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct AppConfig {
        pub host_sam: String,
        pub port_sam: u16,
    }
    use clap::Parser;
    #[derive(Parser)]
    struct Args {
        #[arg(short, long, default_value = "config.yaml")]
        config: String,
    }
    impl AppConfig {
        pub fn is_exists(path: &str) -> bool {
            return fs::metadata(path).is_ok();
        }
        pub fn load() -> Self {
            let args = Args::parse();
            return Self::new(&args.config);            
        }
        pub fn new(path: &str) -> Self {
            if !fs::metadata(path).is_ok() {
                let default_config = Self {
                    host_sam: "127.0.0.1".to_string(),
                    port_sam: 7656,
                };
                let yaml = serde_yaml::to_string(&default_config).expect("Can't create default YAML");
                fs::write(path, yaml).expect("Can't write default config file");
            }

            let yaml = fs::read_to_string(path).expect("Can't read config_file");
            let config: AppConfig = serde_yaml::from_str(&yaml).expect("Error parsing YAML");
            config
        }

        pub fn get_app_dir() -> PathBuf {
            if let Some(proj_dirs) = ProjectDirs::from("com", "wipedlifepotato", "Kekulen") {
                let config_dir = proj_dirs.config_dir();
                fs::create_dir_all(config_dir).expect("Can't create directory");
                return config_dir.to_path_buf();
            }
            PathBuf::from(".")
        }
    }
}