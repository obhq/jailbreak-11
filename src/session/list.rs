use std::collections::HashMap;
use std::num::NonZeroU16;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

/// Active PPPoE sessions.
///
/// Lock order of the members are the same as their definition order.
#[derive(Default)]
pub struct Sessions {
    list: Mutex<HashMap<NonZeroU16, UnboundedSender<()>>>,
    free: Mutex<Vec<NonZeroU16>>,
}

impl Sessions {
    pub fn spawn(self: &Arc<Self>) -> Option<Session> {
        // Get session ID.
        let mut list = self.list.lock().unwrap();
        let mut free = self.free.lock().unwrap();
        let id = match free.pop() {
            Some(v) => v,
            None => (list.len() + 1)
                .try_into()
                .ok()
                .map(|v| unsafe { NonZeroU16::new_unchecked(v) })?,
        };

        // Allocate a session.
        let (tx, rx) = unbounded_channel();

        assert!(list.insert(id, tx).is_none());

        Some(Session {
            slot: Slot {
                list: self.clone(),
                id,
            },
            rx,
        })
    }

    fn free(&self, id: NonZeroU16) {
        let mut list = self.list.lock().unwrap();
        let mut free = self.free.lock().unwrap();

        if Into::<usize>::into(id.get()) != list.len() {
            free.push(id);
        }

        list.remove(&id).unwrap();
    }
}

/// Active PPPoE session.
pub struct Session {
    slot: Slot, // Drop first.
    rx: UnboundedReceiver<()>,
}

impl Session {
    pub fn id(&self) -> NonZeroU16 {
        self.slot.id
    }

    pub async fn run(self) {}
}

/// RAII struct to remove a session from active list.
struct Slot {
    list: Arc<Sessions>,
    id: NonZeroU16,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.list.free(self.id);
    }
}
