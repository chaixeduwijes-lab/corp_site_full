use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use zeroize::Zeroizing;

/// A queued encrypted envelope. The relay never inspects `ciphertext`; it is
/// opaque bytes produced by the sender's E2EE layer.
///
/// `ciphertext` is wrapped in [`Zeroizing`] so that dropping the message —
/// on ACK, on TTL expiry, or on process teardown — overwrites the bytes in
/// place instead of merely freeing them. This is a best-effort measure (it
/// does not defeat a live RAM capture, and true crypto-erasure is the
/// clients' ratchet discarding message keys), but it bounds how long a
/// delivered envelope lingers in reusable heap. See
/// docs/security-audit-messenger-relay.md (F2).
#[derive(Clone)]
pub struct QueuedMessage {
    pub id: String,
    pub ciphertext: Zeroizing<Vec<u8>>,
    pub queued_at: u64,
    pub expires_at: u64,
    /// True once the message was pushed over an open WebSocket connection.
    /// Cleared on disconnect so unacknowledged messages are redelivered.
    pub delivered: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnqueueError {
    /// The recipient's per-device queue is full.
    QueueFull,
    /// The relay-wide RAM budget for queued ciphertext is exhausted.
    BudgetExceeded,
}

struct QueueState {
    queues: HashMap<String, VecDeque<QueuedMessage>>,
    /// Sum of `ciphertext.len()` across every queued message.
    total_bytes: usize,
}

/// In-memory, per-recipient message queue.
///
/// Messages live only in RAM by design: the relay never writes queued
/// ciphertext to disk, so a compromise of the host's storage yields no
/// message data — *provided* the process memory cannot be paged to swap or
/// dumped to a core file (enforced separately, see `harden.rs` and F1). A
/// message is removed on ACK or when its TTL expires, whichever comes first.
///
/// Two independent caps bound memory: `max_per_device` (fairness — one
/// recipient cannot starve others) and `max_total_bytes` (a relay-wide
/// ceiling so the whole queue cannot exceed the host's RAM, see F6).
pub struct MessageQueue {
    inner: Mutex<QueueState>,
    ttl: Duration,
    max_per_device: usize,
    max_total_bytes: usize,
}

impl MessageQueue {
    pub fn new(ttl: Duration, max_per_device: usize, max_total_bytes: usize) -> Self {
        Self {
            inner: Mutex::new(QueueState {
                queues: HashMap::new(),
                total_bytes: 0,
            }),
            ttl,
            max_per_device,
            max_total_bytes,
        }
    }

    /// Queue a ciphertext for `to`. Returns the message id and expiry time.
    pub fn enqueue(
        &self,
        to: &str,
        ciphertext: Vec<u8>,
        now: u64,
    ) -> Result<(String, u64), EnqueueError> {
        let len = ciphertext.len();
        let mut inner = self.inner.lock().unwrap();
        if inner.total_bytes + len > self.max_total_bytes {
            return Err(EnqueueError::BudgetExceeded);
        }
        let queue = inner.queues.entry(to.to_string()).or_default();
        if queue.len() >= self.max_per_device {
            return Err(EnqueueError::QueueFull);
        }
        let msg = QueuedMessage {
            id: uuid::Uuid::new_v4().to_string(),
            ciphertext: Zeroizing::new(ciphertext),
            queued_at: now,
            expires_at: now + self.ttl.as_secs(),
            delivered: false,
        };
        let result = (msg.id.clone(), msg.expires_at);
        queue.push_back(msg);
        inner.total_bytes += len;
        Ok(result)
    }

    /// Return all messages not yet pushed on the current connection, marking
    /// them as delivered. They stay queued until ACKed or expired.
    pub fn take_undelivered(&self, device: &str) -> Vec<QueuedMessage> {
        let mut inner = self.inner.lock().unwrap();
        let Some(queue) = inner.queues.get_mut(device) else {
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
        let Some(queue) = inner.queues.get_mut(device) else {
            return false;
        };
        let mut freed = 0;
        let before = queue.len();
        queue.retain(|m| {
            let keep = m.id != id;
            if !keep {
                freed += m.ciphertext.len();
            }
            keep
        });
        let removed = queue.len() < before;
        if queue.is_empty() {
            inner.queues.remove(device);
        }
        inner.total_bytes -= freed;
        removed
    }

    /// Called when a device's connection closes: everything unACKed becomes
    /// eligible for redelivery on the next connection.
    pub fn reset_delivered(&self, device: &str) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(queue) = inner.queues.get_mut(device) {
            for m in queue.iter_mut() {
                m.delivered = false;
            }
        }
    }

    /// Drop every message past its TTL. Returns how many were dropped.
    pub fn sweep_expired(&self, now: u64) -> usize {
        let mut inner = self.inner.lock().unwrap();
        let mut dropped = 0;
        let mut freed = 0;
        inner.queues.retain(|_, queue| {
            let before = queue.len();
            queue.retain(|m| {
                let keep = m.expires_at > now;
                if !keep {
                    freed += m.ciphertext.len();
                }
                keep
            });
            dropped += before - queue.len();
            !queue.is_empty()
        });
        inner.total_bytes -= freed;
        dropped
    }

    pub fn pending_count(&self, device: &str) -> usize {
        self.inner
            .lock()
            .unwrap()
            .queues
            .get(device)
            .map_or(0, VecDeque::len)
    }

    /// Total bytes of ciphertext currently held across all queues.
    pub fn total_bytes(&self) -> usize {
        self.inner.lock().unwrap().total_bytes
    }

    /// Read-only snapshot of the ciphertexts queued for a device, without
    /// marking them delivered. Intended for metrics and tests that assert the
    /// relay only ever holds opaque bytes.
    pub fn peek(&self, device: &str) -> Vec<Vec<u8>> {
        self.inner
            .lock()
            .unwrap()
            .queues
            .get(device)
            .map(|q| q.iter().map(|m| m.ciphertext.to_vec()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue() -> MessageQueue {
        // Generous global budget so per-device behaviour is what's exercised.
        MessageQueue::new(Duration::from_secs(60), 3, 1 << 20)
    }

    #[test]
    fn enqueue_deliver_ack_removes_message() {
        let q = queue();
        let (id, expires_at) = q.enqueue("bob", b"ct".to_vec(), 100).unwrap();
        assert_eq!(expires_at, 160);
        assert_eq!(q.pending_count("bob"), 1);
        assert_eq!(q.total_bytes(), 2);

        let delivered = q.take_undelivered("bob");
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].id, id);
        assert_eq!(&delivered[0].ciphertext[..], b"ct");
        // Delivered but not ACKed: still queued, not re-pushed.
        assert_eq!(q.pending_count("bob"), 1);
        assert!(q.take_undelivered("bob").is_empty());

        assert!(q.ack("bob", &id));
        assert_eq!(q.pending_count("bob"), 0);
        assert_eq!(q.total_bytes(), 0);
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
        assert_eq!(q.total_bytes(), 6);

        // TTL is 60s: at t=161 the first message is expired, the second is not.
        assert_eq!(q.sweep_expired(161), 1);
        assert_eq!(q.pending_count("bob"), 1);
        assert_eq!(q.total_bytes(), 3);
        assert_eq!(q.sweep_expired(211), 1);
        assert_eq!(q.pending_count("bob"), 0);
        assert_eq!(q.total_bytes(), 0);
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

    #[test]
    fn global_budget_is_enforced_across_devices() {
        // Budget of 10 bytes; each message is 4 bytes ("msg0"…).
        let q = MessageQueue::new(Duration::from_secs(60), 100, 10);
        assert!(q.enqueue("a", b"msg0".to_vec(), 100).is_ok()); // 4
        assert!(q.enqueue("b", b"msg1".to_vec(), 100).is_ok()); // 8
                                                                // Third would reach 12 > 10: rejected regardless of per-device room.
        assert_eq!(
            q.enqueue("c", b"msg2".to_vec(), 100),
            Err(EnqueueError::BudgetExceeded)
        );
        // Freeing space lets a new message in again.
        let id = {
            let m = q.take_undelivered("a");
            m[0].id.clone()
        };
        assert!(q.ack("a", &id));
        assert_eq!(q.total_bytes(), 4);
        assert!(q.enqueue("c", b"msg2".to_vec(), 100).is_ok());
    }
}
