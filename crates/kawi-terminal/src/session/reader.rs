use std::io::{self, Read};
use std::sync::mpsc::SyncSender;
use std::thread::{self, JoinHandle};

use super::PtyEvent;
use super::wake::Wake;

const READ_BUFFER: usize = 8_192;

pub(super) fn spawn(
    mut reader: Box<dyn Read + Send>,
    sender: SyncSender<PtyEvent>,
    wake: Wake,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("kawi-terminal-reader".to_string())
        .spawn(move || read_loop(&mut reader, &sender, &wake))
}

fn read_loop(reader: &mut dyn Read, sender: &SyncSender<PtyEvent>, wake: &Wake) {
    let mut buffer = [0u8; READ_BUFFER];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                let data = PtyEvent::Data(buffer[..read].to_vec());
                if sender.send(data).is_err() {
                    return;
                }
                wake.notify();
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    let _ = sender.send(PtyEvent::Closed);
    wake.notify();
}
