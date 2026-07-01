// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, OverloadedError> {
        let (response_sender, response_receiver) = sync_channel(1);
        let command = Command::Insert {
            draft,
            response_sender,
        };

        self.sender.try_send(command).map_err(|_| OverloadedError)?;
        Ok(response_receiver.recv().unwrap())
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, OverloadedError> {
        let (response_sender, response_receiver) = sync_channel(1);
        let command = Command::Get {
            id,
            response_sender,
        };
        self.sender.try_send(command).map_err(|_| OverloadedError)?;
        Ok(response_receiver.recv().unwrap())
    }
}

pub fn launch(bound: usize) -> TicketStoreClient {
    let (sender, receiver) = sync_channel(bound);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient { sender }
}

#[derive(Debug, thiserror::Error)]
#[error("The store is overloaded")]
pub struct OverloadedError;

enum Command {
    Insert {
        draft: TicketDraft,
        response_sender: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_sender: SyncSender<Option<Ticket>>,
    },
}

fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_sender,
            }) => {
                let id = store.add_ticket(draft);
                let _ = response_sender.send(id);
            }
            Ok(Command::Get {
                id,
                response_sender,
            }) => {
                let ticket = store.get(id);
                let _ = response_sender.send(ticket.cloned());
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
