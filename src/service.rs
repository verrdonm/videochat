use anyhow::{anyhow, Error};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, stream::SplitSink};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use crate::handlers::WsMessage;

pub struct P2pRoomService {
    // RWLock Map of rooms by string id
    room_map: RwLock<HashMap<String, P2pRoom>>,
}

impl P2pRoomService {
    pub fn new() -> Self {
        Self {room_map: RwLock::new(HashMap::new())}
    }

    pub async fn join_room(&self, room_code: &str, participant: Participant) -> Result<(), Error> {
        tracing::info!("getting writer lock for {room_code}");
        let mut writer = self.room_map.write().await;
        tracing::info!("got writer lock for {room_code}");
        // create room is does not exist
        if !writer.contains_key(room_code) {
            writer.insert(room_code.to_owned(), P2pRoom::new(room_code.to_owned()));
        }

        tracing::info!("adding participant {room_code}");
        // add the participant
        if let Some(room) = writer.get_mut(room_code) {

            tracing::info!("Found room {room:?}");
            room.add_participant(participant).await;
        }

        tracing::info!("added participant {room_code}");
        Ok(())
    }

    pub async fn handle_sdp_offer(&self, room: &str, name: &str, sdp: String) -> Result<(), Error> {
        // set my offer, send other offers back to me
        let mut writer = self.room_map.write().await;
        tracing::debug!("handle sdp offer player {:?} in room {:?}", name, room);
        if let Some(r) = writer.get_mut(room) {
            r.set_sdp_offer(name, sdp).await;
            for (other_name, participant) in r.participants.read().await.iter() {

                if other_name != name {
                    tracing::debug!("found other player {:?} in room {:?}", other_name, room);
                    if let Some(other_sdp) = &participant.sdp {

                        tracing::debug!("other player {:?} has sdp {:?}", other_name, room);
                        r.message_participant(name, WsMessage::Offer { name: participant.name.clone(), sdp: other_sdp.clone() }).await;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn handle_sdp_answer(&self, room: &str, sender_name: &str, recipient_name: &str, sdp: String) -> Result<(), Error> {
        let mut writer = self.room_map.write().await;
        tracing::debug!("handle sdp answer player {:?} in room {:?}, to player {:?}", sender_name, room, recipient_name);
        if let Some(r) = writer.get_mut(room) {
            // send answer to the corresponding offer's connections
            r.message_participant(recipient_name, WsMessage::Answer { 
                sender_name: sender_name.to_string(),
                recipient_name: recipient_name.to_string(),
                sdp }).await;

            // pull down candidates from that offer
            if let Some(recipient) = r.participants.read().await.get(recipient_name) {
                for c in recipient.candidates.iter() {
                    let message = WsMessage::Candidate { 
                        sender_name: sender_name.to_string(),
                        candidate: c.to_string(),};
                    r.message_participant(sender_name, message).await;
                }
            }
        }
        Ok(())
    }

    pub async fn handle_candidate(&self, room: &str, sender_name: String, candidate: String) -> Result<(), Error> {
        let mut writer = self.room_map.write().await;

        if let Some(r) = writer.get_mut(room) {
            let mut writable_participants = r.participants.write().await;
            // save my candidates on my participant
            if let Some(p) = writable_participants.get_mut(&sender_name) {
                p.candidates.push(candidate.clone());
            }

            // send my candidate messages to other person (eventually people) in the room.
            for (other_name, participant) in writable_participants.iter() {
                if other_name != &sender_name {
                    tracing::debug!("found other player {:?} in room {:?}", other_name, room);
                    if participant.sdp.is_some() {
                        tracing::debug!("other player {:?} has sdp {:?}", other_name, room);
                        let message = WsMessage::Candidate { 
                            sender_name: sender_name.clone(),
                            candidate: candidate.clone(),};
                        participant.send_message(message).await;
                    }
                }
            }
        }
        Ok(())
    }

}

#[derive(Debug)]
pub struct P2pRoom {
    pub room_code: String,
    participants: RwLock<HashMap<String, Participant>>,
}

impl P2pRoom {
    pub fn new(room_code: String) -> Self {
        Self{
            room_code,
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

    pub async fn set_sdp_offer(&mut self, name: &str, sdp: String) {
        let mut write_lock = self.participants.write().await;
        tracing::debug!("got write lock to set sdp for {name:?}: {sdp:?}");
        if let Some(p) = write_lock.get_mut(name) {
            tracing::debug!("setting sdp for {name:?}");
            p.sdp = Some(sdp);
        }
    }

    pub async fn message_participant(&self, name: &str, message: WsMessage) {
        let read_lock = self.participants.read().await;
        if let Some(p) = read_lock.get(name) {
            p.send_message(message).await;
        }
    }

    // pub async fn find_other_participants(&self, my_name: &str) -> Vec<Participant> {
    //     self.participants.iter().filter(|p| p.name != my_name).cloned().collect()
    // }
}

#[derive(Debug)]
pub struct Participant {
    pub name: String,
    pub sdp: Option<String>,
    pub candidates: Vec<String>,
    sender: Mutex<SplitSink<WebSocket, Message>>,
}

impl Participant {
    pub fn new(name: String, sdp: Option<String>, sender: SplitSink<WebSocket, Message>) -> Self {
        Self {
            name, sdp, candidates: Vec::new(), sender: Mutex::new(sender),
        }
    }

    async fn send_message(&self, message: WsMessage) {
        tracing::debug!("Sending to player {:?} message {:?}", self.name, message);
        let mut sender = self.sender.lock().await;
        let msg = serde_json::to_string(&message).unwrap_or("{}".to_string());
        let _ = sender.send(Message::Text(msg)).await;
    }
}