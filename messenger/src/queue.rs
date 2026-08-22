use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

/// A queued encrypted envelope. The relay never inspects `ciphertext`; it is
/// opaque bytes produced by the sender's E2EE layer.
#[derive(Clone)]
pub struct QueuedMessage {
    pub id: String,
    pub ciphertext: Vec<u8>,
    pub queued_at: u64,
    pub expires_at: u64,
    /// True once the message was pushed over an open WebSocket connection.
    /// Cleared on disconnect so unacknowledged messages are redelivered.
    pub delivered: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnqueueError {
    QueueFull,
}

/// In-memory, per-recipient message queue.
///
/// Messages live only in RAM by design: nothing is ever written to disk, so a
/// compromise of the host's storage yields no message data at all, and a
/// restart is an implicit crypto-erasure of the whole queue. A message is
/// removed on ACK or when its TTL expires, whichever comes first.
pub struct MessageQueue {
    inner: Mutex<HashMap<String, VecDeque<QueuedMessage>>>,
    ttl: Duration,
    max_per_device: usize,
}

impl MessageQueue {
    pub fn new(ttl: Duration, max_per_device: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
            max_per_device,
        }
    }

    /// Queue a ciphertext for `to`. Returns the message id and expiry time.
    pub fn enqueue(
        &self,
        to: &str,
        ciphertext: Vec<u8>,
        now: u64,
    ) -> Result<(String, u64), EnqueueError> {
        let mut inner = self.inner.lock().unwrap();
        let queue = inner.entry(to.to_string()).or_default();
        if queue.len() >= self.max_per_device {
            return Err(EnqueueError::QueueFull);
        }
        let msg = QueuedMessage {
            id: uuid::Uuid::new_v4().to_string(),
            ciphertext,
            queued_at: now,
            expires_at: now + self.ttl.as_secs(),
            delivered: false,
        };
        let result = (msg.id.clone(), msg.expires_at);
        queue.push_back(msg);
        Ok(result)
    }

    /// Return all messages not yet pushed on the current connection, marking
    /// them as delivered. They stay queued until ACKed or expired.
    pub fn take_undelivered(&self, device: &str) -> Vec<QueuedMessage> {
        let mut inner = self.inner.lock().unwrap();
        let Some(queue) = inner.get_mut(device) else {
            return Vec::new();
        };
        queue
            .iter_mut()
            .filter(|m| !m.delivered)
            .map(|m| {
                m.delivered = true;
                m.clone()
            })
            .collect()
    }

    /// Drop a message the recipient has acknowledged. Returns whether the
    /// message was still present.
    pub fn ack(&self, device: &str, id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let Some(queue) = inner.get_mut(device) else {
            return false;
        };
        let before = queue.len();
        queue.retain(|m| m.id != id);
        let removed = queue.len() < before;
        if queue.is_empty() {
            inner.remove(device);
        }
        removed
    }

    /// Called when a device's connection closes: everything unACKed becomes
    /// eligible for redelivery on the next connection.
    pub fn reset_delivered(&self, device: &str) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(queue) = inner.get_mut(device) {
            for m in queue.iter_mut() {
                m.delivered = false;
            }
        }
    }

    /// Drop every message past its TTL. Returns how many were dropped.
    pub fn sweep_expired(&self, now: u64) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let mut dropped = 0;
        inner.retain(|_, queue| {
            let before = queue.len();
            queue.retain(|m| m.expires_at > now);
            dropped += before - queue.len();
            !queue.is_empty()
        });
        dropped
    }

    pub fn pending_count(&self, device: &str) -> usize {
        self.inner
            .lock()
            .unwrap()
            .get(device)
            .map_or(0, VecDeque::len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue() -> MessageQueue {
        MessageQueue::new(Duration::from_secs(60), 3)
    }

    #[test]
    fn enqueue_deliver_ack_removes_message() {
        let q = queue();
        let (id, expires_at) = q.enqueue("bob", b"ct".to_vec(), 100).unwrap();
        assert_eq!(expires_at, 160);
        assert_eq!(q.pending_count("bob"), 1);

        let delivered = q.take_undelivered("bob");
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].id, id);
        // Delivered but not ACKed: still queued, not re-pushed.
        assert_eq!(q.pending_count("bob"), 1);
        assert!(q.take_undelivered("bob").is_empty());

        assert!(q.ack("bob", &id));
        assert_eq!(q.pending_count("bob"), 0);
        assert!(!q.ack("bob", &id));
    }

    #[test]
    fn reconnect_redelivers_unacked_messages() {
        let q = queue();
        q.enqueue("bob", b"ct".to_vec(), 100).unwrap();
        assert_eq!(q.take_undelivered("bob").len(), 1);
        assert!(q.take_undelivered("bob").is_empty());

        q.reset_delivered("bob");
        assert_eq!(q.take_undelivered("bob").len(), 1);
    }

    #[test]
    fn sweep_drops_only_expired_messages() {
        let q = queue();
        q.enqueue("bob", b"old".to_vec(), 100).unwrap();
        q.enqueue("bob", b"new".to_vec(), 150).unwrap();

        // TTL is 60s: at t=161 the first message is expired, the second is not.
        assert_eq!(q.sweep_expired(161), 1);
        assert_eq!(q.pending_count("bob"), 1);
        assert_eq!(q.sweep_expired(211), 1);
        assert_eq!(q.pending_count("bob"), 0);
    }

    #[test]
    fn per_device_queue_is_bounded() {
        let q = queue();
        for _ in 0..3 {
            q.enqueue("bob", b"ct".to_vec(), 100).unwrap();
        }
        assert_eq!(
            q.enqueue("bob", b"ct".to_vec(), 100),
            Err(EnqueueError::QueueFull)
        );
        // Other recipients are unaffected.
        assert!(q.enqueue("alice", b"ct".to_vec(), 100).is_ok());
    }
}
