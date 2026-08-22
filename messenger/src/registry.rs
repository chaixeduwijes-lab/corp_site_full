use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, RwLock};

use ed25519_dalek::VerifyingKey;
use rusqlite::Connection;

/// A registered client device. The relay knows nothing about it beyond an
/// opaque id and the Ed25519 public key used to authenticate its requests.
#[derive(Clone)]
pub struct Device {
    pub device_id: String,
    pub verifying_key: VerifyingKey,
    pub created_at: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegisterError {
    DuplicateKey,
    LimitReached,
}

/// Persistent directory of devices and their public identity keys.
///
/// SQLite holds only public data (ids + public keys); the in-memory map is
/// authoritative for reads so the auth hot path never touches the disk.
pub struct DeviceRegistry {
    conn: Mutex<Connection>,
    devices: RwLock<HashMap<String, Device>>,
}

impl DeviceRegistry {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS devices (
                 device_id   TEXT PRIMARY KEY,
                 identity_pk BLOB NOT NULL UNIQUE,
                 created_at  INTEGER NOT NULL
             );",
        )?;

        let mut devices = HashMap::new();
        {
            let mut stmt =
                conn.prepare("SELECT device_id, identity_pk, created_at FROM devices")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (device_id, pk_bytes, created_at) = row?;
                let pk: [u8; 32] = pk_bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("corrupt identity key for device {device_id}"))?;
                let verifying_key = VerifyingKey::from_bytes(&pk)
                    .map_err(|_| anyhow::anyhow!("invalid identity key for device {device_id}"))?;
                devices.insert(
                    device_id.clone(),
                    Device {
                        device_id,
                        verifying_key,
                        created_at: created_at as u64,
                    },
                );
            }
        }

        Ok(Self {
            conn: Mutex::new(conn),
            devices: RwLock::new(devices),
        })
    }

    pub fn get(&self, device_id: &str) -> Option<Device> {
        self.devices.read().unwrap().get(device_id).cloned()
    }

    pub fn list(&self) -> Vec<Device> {
        let mut all: Vec<Device> = self.devices.read().unwrap().values().cloned().collect();
        all.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        all
    }

    pub fn count(&self) -> usize {
        self.devices.read().unwrap().len()
    }

    pub fn register(
        &self,
        verifying_key: VerifyingKey,
        now: u64,
        max_devices: usize,
    ) -> Result<Device, RegisterError> {
        let mut devices = self.devices.write().unwrap();
        if devices.len() >= max_devices {
            return Err(RegisterError::LimitReached);
        }
        if devices
            .values()
            .any(|d| d.verifying_key.as_bytes() == verifying_key.as_bytes())
        {
            return Err(RegisterError::DuplicateKey);
        }

        let device = Device {
            device_id: uuid::Uuid::new_v4().to_string(),
            verifying_key,
            created_at: now,
        };

        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO devices (device_id, identity_pk, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    device.device_id,
                    device.verifying_key.as_bytes().as_slice(),
                    now as i64
                ],
            )
            .map_err(|_| RegisterError::DuplicateKey)?;

        devices.insert(device.device_id.clone(), device.clone());
        Ok(device)
    }
}
