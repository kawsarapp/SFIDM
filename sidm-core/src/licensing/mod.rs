use chrono::{DateTime, Utc};
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const LICENSE_SALT: &[u8] = b"SIDM_SUPER_SECURE_SALT_2026";
const TRIAL_DAYS: i64 = 14;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum LicenseStatus {
    Trial {
        days_remaining: i64,
        is_active: bool,
    },
    Activated {
        license_key: String,
        plan: String,
        customer_email: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        is_active: bool,
    },
    Expired {
        message: String,
    },
    Invalid {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub hwid: String,
    pub status: LicenseStatus,
    pub is_pro: bool,
    pub max_connections: usize,
    pub support_8k: bool,
}

#[derive(Debug, Deserialize)]
pub struct ActivateRequest {
    pub license_key: String,
    pub customer_email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredLicense {
    license_key: String,
    plan: String,
    customer_email: Option<String>,
    hwid: String,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredTrial {
    first_run: i64,
    hwid: String,
    checksum: String,
}

pub struct LicenseManager {
    hwid: String,
    sidm_dir: PathBuf,
}

impl LicenseManager {
    pub fn new() -> Self {
        let hwid = Self::generate_hwid();
        let sidm_dir = Self::get_sidm_dir();
        let _ = fs::create_dir_all(&sidm_dir);

        Self { hwid, sidm_dir }
    }

    pub fn get_hwid(&self) -> &str {
        &self.hwid
    }

    /// Generate unique deterministic Hardware ID (HWID)
    pub fn generate_hwid() -> String {
        let mut raw_data = String::new();

        // 1. Linux machine-id / DBus ID
        if let Ok(id) = fs::read_to_string("/etc/machine-id") {
            raw_data.push_str(id.trim());
        } else if let Ok(id) = fs::read_to_string("/var/lib/dbus/machine-id") {
            raw_data.push_str(id.trim());
        }

        // 2. CPU info or hostname
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            raw_data.push_str(&hostname);
        }
        if let Ok(user) = std::env::var("USER") {
            raw_data.push_str(&user);
        }

        // Fallback if empty
        if raw_data.is_empty() {
            raw_data = "SIDM_DEFAULT_DEVICE_FINGERPRINT".to_string();
        }

        // Hash with SHA-256
        let hash = digest(&SHA256, raw_data.as_bytes());
        let hex = hash
            .as_ref()
            .iter()
            .take(8)
            .map(|b| format!("{:02X}", b))
            .collect::<String>();

        // Format: SIDM-XXXX-XXXX-XXXX-XXXX
        format!(
            "SIDM-{}-{}-{}-{}",
            &hex[0..4],
            &hex[4..8],
            &hex[8..12],
            &hex[12..16]
        )
    }

    fn get_sidm_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".sidm")
        } else {
            PathBuf::from("./.sidm")
        }
    }

    /// Check the current licensing status
    pub fn check_license(&self) -> LicenseInfo {
        // 1. Check for activated license file
        let lic_file = self.sidm_dir.join("license.key");
        if lic_file.exists() {
            if let Ok(content) = fs::read_to_string(&lic_file) {
                if let Ok(stored) = serde_json::from_str::<StoredLicense>(&content) {
                    // Verify HWID match
                    if stored.hwid == self.hwid {
                        let expected_sig = self.sign_license(&stored.license_key, &stored.plan);
                        if stored.signature == expected_sig {
                            return LicenseInfo {
                                hwid: self.hwid.clone(),
                                status: LicenseStatus::Activated {
                                    license_key: stored.license_key,
                                    plan: stored.plan,
                                    customer_email: stored.customer_email,
                                    expires_at: None, // Lifetime or remote verified
                                    is_active: true,
                                },
                                is_pro: true,
                                max_connections: 32,
                                support_8k: true,
                            };
                        }
                    }
                }
            }
        }

        // 2. Check Trial period
        let trial_file = self.sidm_dir.join("trial.dat");
        let now = Utc::now().timestamp();

        let first_run = if trial_file.exists() {
            if let Ok(content) = fs::read_to_string(&trial_file) {
                if let Ok(stored_trial) = serde_json::from_str::<StoredTrial>(&content) {
                    let expected = self.sign_trial(stored_trial.first_run);
                    if stored_trial.checksum == expected && stored_trial.hwid == self.hwid {
                        stored_trial.first_run
                    } else {
                        now
                    }
                } else {
                    now
                }
            } else {
                now
            }
        } else {
            // First time launch: initialize 14-day PRO trial
            let stored_trial = StoredTrial {
                first_run: now,
                hwid: self.hwid.clone(),
                checksum: self.sign_trial(now),
            };
            if let Ok(json) = serde_json::to_string(&stored_trial) {
                let _ = fs::write(&trial_file, json);
            }
            now
        };

        let elapsed_days = (now - first_run) / (24 * 3600);
        let days_remaining = TRIAL_DAYS - elapsed_days;

        if days_remaining > 0 {
            LicenseInfo {
                hwid: self.hwid.clone(),
                status: LicenseStatus::Trial {
                    days_remaining,
                    is_active: true,
                },
                is_pro: true,
                max_connections: 16,
                support_8k: true,
            }
        } else {
            LicenseInfo {
                hwid: self.hwid.clone(),
                status: LicenseStatus::Expired {
                    message: "Your 14-day PRO trial has ended. Activate license to unlock 32x speed & 8K.".to_string(),
                },
                is_pro: false,
                max_connections: 4,
                support_8k: false,
            }
        }
    }

    /// Activate a serial license key
    pub fn activate(&self, key: &str, email: Option<String>) -> Result<LicenseInfo, String> {
        let clean_key = key.trim().to_uppercase();

        // Key Validation Rules:
        // Format: SIDM-PRO-XXXX-XXXX or SIDM-LIFE-XXXX-XXXX
        if !clean_key.starts_with("SIDM-") {
            return Err("Invalid license key format. Key must start with 'SIDM-'".to_string());
        }

        let plan = if clean_key.contains("-LIFE-") {
            "Lifetime Commercial Pro".to_string()
        } else if clean_key.contains("-PRO-") {
            "Pro 1-Year Commercial".to_string()
        } else {
            "Commercial Standard".to_string()
        };

        let signature = self.sign_license(&clean_key, &plan);

        let stored = StoredLicense {
            license_key: clean_key,
            plan,
            customer_email: email,
            hwid: self.hwid.clone(),
            signature,
        };

        let json = serde_json::to_string_pretty(&stored)
            .map_err(|e| format!("Serialization error: {}", e))?;

        let lic_file = self.sidm_dir.join("license.key");
        fs::write(lic_file, json).map_err(|e| format!("Failed to save license: {}", e))?;

        Ok(self.check_license())
    }

    fn sign_license(&self, key: &str, plan: &str) -> String {
        let mut data = Vec::new();
        data.extend_from_slice(key.as_bytes());
        data.extend_from_slice(plan.as_bytes());
        data.extend_from_slice(self.hwid.as_bytes());
        data.extend_from_slice(LICENSE_SALT);

        let hash = digest(&SHA256, &data);
        hash.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    fn sign_trial(&self, first_run: i64) -> String {
        let mut data = Vec::new();
        data.extend_from_slice(&first_run.to_le_bytes());
        data.extend_from_slice(self.hwid.as_bytes());
        data.extend_from_slice(LICENSE_SALT);

        let hash = digest(&SHA256, &data);
        hash.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}
