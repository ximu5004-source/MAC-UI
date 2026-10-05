use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{RecvTimeoutError, SyncSender, TryRecvError, TrySendError, sync_channel},
};
use std::time::Duration;

/// Keep Explorer's synchronous hook callback independent of icon decoding,
/// filesystem access and frontend event delivery. One worker preserves order.
pub(super) struct EventQueue<T> {
    sender: SyncSender<T>,
    overflowed: Arc<AtomicBool>,
}

impl<T: Send + 'static> EventQueue<T> {
    pub(super) fn start(
        capacity: usize,
        mut process: impl FnMut(T) + Send + 'static,
        mut recover: impl FnMut() + Send + 'static,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = sync_channel(capacity);
        let overflowed = Arc::new(AtomicBool::new(false));
        let worker_overflowed = overflowed.clone();
        std::thread::Builder::new()
            .name("TrayEvents".into())
            .spawn(move || {
                loop {
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Ok(event) => process(event),
                        Err(RecvTimeoutError::Disconnected) => return,
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                    loop {
                        match receiver.try_recv() {
                            Ok(event) => process(event),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => return,
                        }
                    }
                    if worker_overflowed.swap(false, Ordering::AcqRel) {
                        // An add/remove may have been dropped. Re-enumerate once
                        // the burst is drained instead of leaving stale icons.
                        recover();
                    }
                }
            })?;
        Ok(Self { sender, overflowed })
    }

    pub(super) fn push(&self, event: T) -> bool {
        match self.sender.try_send(event) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.overflowed.store(true, Ordering::Release);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc::channel, time::Duration};

    #[test]
    fn slow_consumer_does_not_block_producer_and_preserves_order() {
        let (entered_tx, entered_rx) = channel();
        let (resume_tx, resume_rx) = channel();
        let (processed_tx, processed_rx) = channel();
        let queue = EventQueue::start(
            2,
            move |value| {
                if value == 1 {
                    entered_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                }
                processed_tx.send(value).unwrap();
            },
            || panic!("No overflow expected"),
        )
        .unwrap();
        assert!(queue.push(1));
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(queue.push(2));
        assert!(queue.push(3));
        resume_tx.send(()).unwrap();
        for expected in 1..=3 {
            assert_eq!(
                processed_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn overflow_is_nonblocking_and_recovers_once_after_the_burst() {
        let (entered_tx, entered_rx) = channel();
        let (resume_tx, resume_rx) = channel();
        let (recovered_tx, recovered_rx) = channel();
        let queue = EventQueue::start(
            1,
            move |value| {
                if value == 1 {
                    entered_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                }
            },
            move || {
                recovered_tx.send(()).unwrap();
            },
        )
        .unwrap();
        assert!(queue.push(1));
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(queue.push(2));
        assert!(!queue.push(3));
        assert!(!queue.push(4));
        resume_tx.send(()).unwrap();
        recovered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(recovered_rx.try_recv().is_err());
    }
}
