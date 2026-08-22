use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc::UnboundedSender;

use crate::auth::ReplayCache;
use crate::config::Config;
use crate::queue::MessageQueue;
use crate::registry::DeviceRegistry;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub config: Config,
    pub registry: DeviceRegistry,
    pub queue: MessageQueue,
    pub online: Online,
    pub replay: ReplayCache,
}

impl AppState {
    pub fn new(config: Config, registry: DeviceRegistry) -> SharedState {
        let queue = MessageQueue::new(
            config.message_ttl,
            config.max_queue_per_device,
            config.max_total_queue_bytes,
        );
        Arc::new(Self {
            config,
            registry,
            queue,
            online: Online::default(),
            replay: ReplayCache::default(),
        })
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}

#[derive(Debug)]
pub enum WakeEvent {
    /// A new message was queued for this device.
    NewMessage,
    /// Another connection authenticated as the same device; this one must go.
    Replaced,
}

/// Live WebSocket connections, one per device. A newer connection for the
/// same device replaces the older one.
#[derive(Default)]
pub struct Online {
    map: Mutex<HashMap<String, (String, UnboundedSender<WakeEvent>)>>,
}

impl Online {
    pub fn connect(&self, device_id: &str, conn_id: &str, sender: UnboundedSender<WakeEvent>) {
        let mut map = self.map.lock().unwrap();
        if let Some((_, old)) = map.insert(device_id.to_string(), (conn_id.to_string(), sender)) {
            let _ = old.send(WakeEvent::Replaced);
        }
    }

    /// Remove the mapping only if it still belongs to this connection.
    pub fn disconnect(&self, device_id: &str, conn_id: &str) {
        let mut map = self.map.lock().unwrap();
        if map.get(device_id).is_some_and(|(id, _)| id == conn_id) {
            map.remove(device_id);
        }
    }

    /// Wake the device's delivery loop. Returns false if it is offline.
    pub fn notify(&self, device_id: &str) -> bool {
        self.map
            .lock()
            .unwrap()
            .get(device_id)
            .is_some_and(|(_, tx)| tx.send(WakeEvent::NewMessage).is_ok())
    }
}
