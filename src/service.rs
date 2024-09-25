use anyhow::{anyhow, Error};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, stream::SplitSink};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use crate::handlers::{MessagePayload, WebSocketMessage};

pub struct P2pRoomService {
    room_map: RwLock<HashMap<String, P2pRoom>>,
}

impl P2pRoomService {
    pub fn new() -> Self {
        Self {room_map: RwLock::new(HashMap::new())}
    }

    pub async fn join_room(&self, room_code: &str, participant: Participant) -> Result<(), Error> {
        tracing::debug!("getting writer lock for {room_code}");
        let mut writer = self.room_map.write().await;
        tracing::debug!("got writer lock for {room_code}");
        // create room is does not exist
        if !writer.contains_key(room_code) {
            writer.insert(room_code.to_owned(), P2pRoom::new());
        }

        tracing::info!("adding participant {room_code}");
        // add the participant
        if let Some(room) = writer.get_mut(room_code) {
            tracing::debug!("Found room {room:?}");
            let name = participant.name.clone();
            room.add_participant(participant).await;

            tracing::debug!("added participant {name} to room {room_code}");
            // // send peers message back to the joiner
            
            // let names = room.find_other_participant_names(&name).await;
            // room.message_participant(WebSocketMessage{recipient: name, payload: MessagePayload::Peers { names }}).await;
        }

        Ok(())
    }

    pub async fn leave_room(&self, room_code: &str, name: &str) {
        let mut writer = self.room_map.write().await;
        if let Some(room) = writer.get_mut(room_code) {
            room.remove_participant(name).await;
        }
    }

    pub async fn send_room_peers(&self, room_code: &str, recipient: &str) {
        let reader = self.room_map.read().await;
        if let Some(room) = reader.get(room_code) {
            tracing::debug!("Found room {room:?}");

            // send peers message back to recipient
            
            let names = room.find_other_participant_names(recipient).await;
            room.message_participant(WebSocketMessage{recipient: recipient.to_owned(), payload: MessagePayload::Peers { names }}).await;
        }
    }

    pub async fn relay_message(&self, room_code: &str, message: WebSocketMessage) -> Result<(), Error> {
        let read_lock = self.room_map.read().await;
        if let Some(r) = read_lock.get(room_code) {
            r.message_participant(message).await;
        }
        
        Ok(())
    }

}

#[derive(Debug)]
pub struct P2pRoom {
    participants: RwLock<HashMap<String, Participant>>,
}

impl P2pRoom {
    pub fn new() -> Self {
        Self{
            participants: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_participant(&mut self, participant: Participant) {
        tracing::info!("adding participant in room for participant {participant:?}");
        let mut write_lock = self.participants.write().await;

        tracing::info!("got read lock in room for participant {participant:?}");
        if !write_lock.contains_key(&participant.name) {
            write_lock.insert(participant.name.clone(), participant);
        }
    }

    pub async fn remove_participant(&mut self, name: &str) {
        tracing::info!("adding participant in room for participant {name}");
        let mut write_lock = self.participants.write().await;
        write_lock.remove(name);
    }

    pub async fn message_participant(&self, message: WebSocketMessage) {
        let read_lock = self.participants.read().await;
        if let Some(p) = read_lock.get(&message.recipient) {
            p.send_message(message).await;
        }
    }

    pub async fn find_all_participant_names(&self) -> Vec<String> {
        let read_lock = self.participants.read().await;
        read_lock.iter()
            .map(|(name, _)| name)
            .cloned()
            .collect()
    }

    pub async fn find_other_participant_names(&self, my_name: &str) -> Vec<String> {
        let read_lock = self.participants.read().await;
        read_lock.iter()
            .filter(|(name, _)| *name != my_name)
            .map(|(name, _)| name)
            .cloned()
            .collect()
    }
}


/// A particpant holds the name and a mutex on the sender component of their websocket.
/// In past implementations, this stored more than it needed to, and is now a more strict
/// relay facilitator. PeerConnection state management is moved entirely to the javascript
/// client. This has some points around when ICE Candidates are generated as part of peer 
/// connection setup and then sent to the other end of the connection over the signalling
/// server
#[derive(Debug)]
pub struct Participant {
    pub name: String,
    sender: Mutex<SplitSink<WebSocket, Message>>,
}

impl Participant {
    pub fn new(name: String, sender: SplitSink<WebSocket, Message>) -> Self {
        Self {
            name, sender: Mutex::new(sender),
        }
    }

    async fn send_message(&self, message: WebSocketMessage) {
        let mut sender = self.sender.lock().await;
        let msg = serde_json::to_string(&message).unwrap_or("{}".to_string());
        tracing::debug!("Sending to player {:?} message {:?}", self.name, message);
        match sender.send(Message::Text(msg)).await {
            Ok(()) => (),
            Err(e) => tracing::warn!("Failed to send message to {}, error: {}", self.name, e),
        }
    }
}