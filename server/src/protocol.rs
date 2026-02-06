use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    CreateRoom,
    JoinRoom { room_id: String },
    ListRooms,
    Play { cards: Vec<String> },
    Pass,
    RestartGame,
    Ping,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Welcome {
        user_id: u64,
        user_name: String,
    },
    RoomCreated {
        room_id: String,
    },
    Joined {
        room_id: String,
        you: u64,
        you_name: String,
        player_count: usize,
        started: bool,
    },
    RoomsList {
        rooms: Vec<RoomSummary>,
    },
    RoomState(RoomSnapshot),
    PlayRejected {
        reason: String,
    },
    GameOver {
        room_id: String,
        winner_id: u64,
    },
    RoomInterrupted {
        room_id: String,
        leaver_id: u64,
        player_count: usize,
    },
    GameRestarted {
        room_id: String,
    },
    Error {
        message: String,
    },
    Pong,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u64,
    pub name: String,
    pub hand_count: usize,
    pub is_landlord: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomSummary {
    pub room_id: String,
    pub player_count: usize,
    pub started: bool,
    pub can_join: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayView {
    pub kind: String,
    pub main_rank: String,
    pub size: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub room_id: String,
    pub players: Vec<PlayerInfo>,
    pub turn: u64,
    pub last_player: Option<u64>,
    pub last_play: Option<PlayView>,
    pub your_hand: Vec<String>,
}
