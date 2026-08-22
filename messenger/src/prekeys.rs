use std::collections::HashMap;
use std::sync::Mutex;

/// A one-time prekey: an opaque id chosen by the owner and a base64 Curve25519
/// public key. Consumed at most once by an initiator starting a session.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct OneTimeKey {
    pub id: String,
    pub key: String,
}

/// A device's published prekey bundle. Everything here is *public* key
/// material — the directory never holds a secret. Bundles live only in RAM
/// (like the message queue): clients republish on reconnect, so nothing about
/// group membership or key rotation is persisted to disk (audit F5).
#[derive(Clone, Default)]
pub struct PrekeyBundle {
    pub identity_key: String,
    pub one_time_keys: Vec<OneTimeKey>,
    /// Reusable last-resort key used when the one-time keys are exhausted.
    pub fallback_key: Option<String>,
}

/// What an initiator receives when claiming a peer's bundle: the peer's
/// identity key plus a single one-time key (or, if none remain, the fallback).
#[derive(Clone, serde::Serialize)]
pub struct ClaimedBundle {
    pub identity_key: String,
    pub one_time_key: Option<OneTimeKey>,
    pub fallback_key: Option<String>,
}

pub struct PrekeyStore {
    inner: Mutex<HashMap<String, PrekeyBundle>>,
}

impl PrekeyStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Publish (or refresh) a device's bundle. The identity and fallback keys
    /// are replaced; new one-time keys are appended, de-duplicated by id, so a
    /// client can top up its supply without dropping unclaimed keys.
    pub fn publish(
        &self,
        device_id: &str,
        identity_key: String,
        fallback_key: Option<String>,
        one_time_keys: Vec<OneTimeKey>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let bundle = inner.entry(device_id.to_string()).or_default();
        bundle.identity_key = identity_key;
        bundle.fallback_key = fallback_key;
        for otk in one_time_keys {
            if !bundle.one_time_keys.iter().any(|k| k.id == otk.id) {
                bundle.one_time_keys.push(otk);
            }
        }
    }

    /// Claim a bundle for a peer, consuming one one-time key. Returns `None`
    /// if the peer has never published. When one-time keys are exhausted the
    /// reusable fallback is returned instead (`one_time_key` is `None`).
    pub fn claim(&self, device_id: &str) -> Option<ClaimedBundle> {
        let mut inner = self.inner.lock().unwrap();
        let bundle = inner.get_mut(device_id)?;
        let one_time_key = bundle.one_time_keys.pop();
        Some(ClaimedBundle {
            identity_key: bundle.identity_key.clone(),
            one_time_key,
            fallback_key: bundle.fallback_key.clone(),
        })
    }

    /// Number of unclaimed one-time keys a device still has (for top-up hints).
    pub fn available_one_time_keys(&self, device_id: &str) -> usize {
        self.inner
            .lock()
            .unwrap()
            .get(device_id)
            .map_or(0, |b| b.one_time_keys.len())
    }

    /// Forget a device's bundle (e.g. on removal). Public keys only, but keeps
    /// the directory tidy.
    pub fn remove(&self, device_id: &str) {
        self.inner.lock().unwrap().remove(device_id);
    }
}

impl Default for PrekeyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn otk(id: &str) -> OneTimeKey {
        OneTimeKey {
            id: id.to_string(),
            key: format!("key-{id}"),
        }
    }

    #[test]
    fn publish_then_claim_consumes_one_time_keys() {
        let store = PrekeyStore::new();
        store.publish(
            "bob",
            "id-bob".into(),
            Some("fb".into()),
            vec![otk("1"), otk("2")],
        );
        assert_eq!(store.available_one_time_keys("bob"), 2);

        let first = store.claim("bob").unwrap();
        assert_eq!(first.identity_key, "id-bob");
        assert!(first.one_time_key.is_some());
        assert_eq!(store.available_one_time_keys("bob"), 1);

        store.claim("bob").unwrap();
        assert_eq!(store.available_one_time_keys("bob"), 0);

        // Exhausted: fall back to the reusable key, no one-time key.
        let fallback = store.claim("bob").unwrap();
        assert!(fallback.one_time_key.is_none());
        assert_eq!(fallback.fallback_key.as_deref(), Some("fb"));
    }

    #[test]
    fn publish_appends_and_dedups_one_time_keys() {
        let store = PrekeyStore::new();
        store.publish("a", "id".into(), None, vec![otk("1")]);
        store.publish("a", "id".into(), None, vec![otk("1"), otk("2")]);
        assert_eq!(store.available_one_time_keys("a"), 2);
    }

    #[test]
    fn claim_unknown_device_is_none() {
        let store = PrekeyStore::new();
        assert!(store.claim("nobody").is_none());
    }
}
