//! A single serialized reader, independent of the UI event loop.
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::model::BatteryStatus;

enum Command {
    Refresh,
    Visible(bool),
    Stop,
    #[cfg(test)]
    Barrier(Sender<()>),
}

pub struct BatteryService {
    sender: Sender<Command>,
}

impl BatteryService {
    pub fn start(
        read: impl FnMut() -> BatteryStatus + Send + 'static,
        publish: impl FnMut(BatteryStatus) -> bool + Send + 'static,
    ) -> std::io::Result<Self> {
        Self::start_with_intervals(
            read,
            publish,
            Duration::from_secs(60),
            Duration::from_secs(1),
        )
    }

    fn start_with_intervals(
        mut read: impl FnMut() -> BatteryStatus + Send + 'static,
        mut publish: impl FnMut(BatteryStatus) -> bool + Send + 'static,
        hidden_interval: Duration,
        visible_interval: Duration,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("ninjutso-battery".into())
            .spawn(move || {
                let mut visible = false;
                let mut deadline = Instant::now();
                loop {
                    let delay = deadline.saturating_duration_since(Instant::now());
                    match receiver.recv_timeout(delay) {
                        Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        #[cfg(test)]
                        Ok(Command::Barrier(ack)) => {
                            let _ = ack.send(());
                        }
                        Ok(Command::Refresh) => deadline = Instant::now(),
                        Ok(Command::Visible(value)) => {
                            visible = value;
                            deadline = if visible {
                                Instant::now()
                            } else {
                                Instant::now() + hidden_interval
                            };
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if !publish(read()) {
                                break;
                            }
                            deadline = Instant::now()
                                + if visible {
                                    visible_interval
                                } else {
                                    hidden_interval
                                };
                        }
                    }
                }
            })?;
        Ok(Self { sender })
    }

    pub fn refresh(&self) {
        let _ = self.sender.send(Command::Refresh);
    }
    pub fn set_visible(&self, visible: bool) {
        let _ = self.sender.send(Command::Visible(visible));
    }
}

impl Drop for BatteryService {
    fn drop(&mut self) {
        // A slow driver must not hold the UI or application shutdown hostage.
        let _ = self.sender.send(Command::Stop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConnectionKind, MouseBattery};
    use std::sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    };

    #[test]
    fn refresh_delivers_new_data_and_visible_polling_stops_after_hiding() {
        let percent = Arc::new(AtomicU8::new(72));
        let source = percent.clone();
        let (tx, rx) = mpsc::channel();
        let worker = BatteryService::start_with_intervals(
            move || {
                BatteryStatus::Ready(MouseBattery {
                    percent: source.load(Ordering::Relaxed),
                    online: true,
                    charging: false,
                    connection: ConnectionKind::Receiver,
                    polling_rate: None,
                    dpi: None,
                })
            },
            move |status| tx.send(status).is_ok(),
            Duration::from_secs(60),
            Duration::from_millis(25),
        )
        .unwrap();
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), BatteryStatus::Ready(b) if b.percent==72)
        );
        percent.store(12, Ordering::Relaxed);
        worker.refresh();
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), BatteryStatus::Ready(b) if b.percent==12)
        );
        worker.set_visible(true);
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.set_visible(false);
        let (ack_tx, ack_rx) = mpsc::channel();
        worker.sender.send(Command::Barrier(ack_tx)).unwrap();
        ack_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        while rx.try_recv().is_ok() {}
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
    }
}
