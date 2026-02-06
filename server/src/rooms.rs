use crate::protocol::{PlayView, PlayerInfo, RoomSnapshot, RoomSummary};
use game_core::{GameError, GameState, Play};
use rand::distributions::Alphanumeric;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Debug)]
pub struct PlayerConn {
    pub id: u64,
    pub tx: Option<UnboundedSender<crate::protocol::ServerMessage>>,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub players: Vec<PlayerConn>,
    pub state: Option<GameState>,
}

#[derive(Clone, Debug)]
pub struct RoomManager {
    rooms: HashMap<String, Room>,
    rng: StdRng,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveConnectionResult {
    pub room_deleted: bool,
    pub game_interrupted: bool,
    pub player_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomError {
    NotFound,
    Full,
    AlreadyJoined,
    InvalidPlay,
    NotYourTurn,
    MustBeatPrevious,
    CardsNotOwned,
    GameOver,
    NotReady,
    CannotPass,
    RestartNotAllowed,
}

impl RoomManager {
    pub fn new() -> Self {
        Self::with_seed(rand::random())
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            rooms: HashMap::new(),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn create_room(&mut self) -> String {
        let id = self.new_room_id();
        let room = Room {
            players: Vec::new(),
            state: None,
        };
        self.rooms.insert(id.clone(), room);
        id
    }

    pub fn join_room(&mut self, room_id: &str, player: PlayerConn) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.players.iter().any(|p| p.id == player.id) {
            return Err(RoomError::AlreadyJoined);
        }
        if room.players.len() >= 3 {
            return Err(RoomError::Full);
        }
        room.players.push(player);
        Ok(())
    }

    pub fn remove_connection(
        &mut self,
        room_id: &str,
        user_id: u64,
    ) -> Option<RemoveConnectionResult> {
        let room = self.rooms.get_mut(room_id)?;
        let had_state = room.state.is_some();
        let before_len = room.players.len();
        room.players.retain(|p| p.id != user_id);
        let player_count = room.players.len();
        let removed = player_count < before_len;
        let game_interrupted = had_state && removed && player_count < 3;

        if game_interrupted {
            room.state = None;
        }

        if player_count == 0 {
            self.rooms.remove(room_id);
            return Some(RemoveConnectionResult {
                room_deleted: true,
                game_interrupted,
                player_count: 0,
            });
        }

        Some(RemoveConnectionResult {
            room_deleted: false,
            game_interrupted,
            player_count,
        })
    }

    pub fn start_if_ready(&mut self, room_id: &str, seed: u64) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.players.len() < 3 {
            return Err(RoomError::NotReady);
        }
        if room.state.is_some() {
            return Ok(());
        }
        let player_ids = [room.players[0].id, room.players[1].id, room.players[2].id];
        room.state = Some(GameState::new(player_ids, seed));
        Ok(())
    }

    pub fn apply_play(
        &mut self,
        room_id: &str,
        player_id: u64,
        cards: Vec<game_core::Card>,
    ) -> Result<Option<u64>, RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        let state = room.state.as_mut().ok_or(RoomError::NotReady)?;
        let player_idx = state.player_index(player_id).ok_or(RoomError::NotFound)?;
        let outcome = state
            .apply_play(player_idx, cards)
            .map_err(map_game_error)?;
        let winner_id = outcome.winner.map(|idx| state.players[idx].id);
        Ok(winner_id)
    }

    pub fn pass_turn(&mut self, room_id: &str, player_id: u64) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        let state = room.state.as_mut().ok_or(RoomError::NotReady)?;
        let player_idx = state.player_index(player_id).ok_or(RoomError::NotFound)?;
        state.pass(player_idx).map_err(map_game_error)?;
        Ok(())
    }

    pub fn restart_game(
        &mut self,
        room_id: &str,
        requester_id: u64,
        seed: u64,
    ) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.players.len() < 3 {
            return Err(RoomError::NotReady);
        }
        if !room.players.iter().any(|player| player.id == requester_id) {
            return Err(RoomError::RestartNotAllowed);
        }
        let previous_state = room.state.as_ref().ok_or(RoomError::NotReady)?;
        if !previous_state.players.iter().any(|player| player.out) {
            return Err(RoomError::RestartNotAllowed);
        }
        let player_ids = [room.players[0].id, room.players[1].id, room.players[2].id];
        room.state = Some(GameState::new(player_ids, seed));
        Ok(())
    }

    pub fn snapshot_for(&self, room_id: &str, player_id: u64) -> Option<RoomSnapshot> {
        let room = self.rooms.get(room_id)?;
        let state = room.state.as_ref()?;
        let players = state
            .players
            .iter()
            .map(|p| PlayerInfo {
                id: p.id,
                name: display_name_for_user(p.id),
                hand_count: p.hand.len(),
                is_landlord: state.player_index(p.id) == Some(state.landlord),
            })
            .collect();
        let your_hand = state
            .players
            .iter()
            .find(|p| p.id == player_id)
            .map(|p| p.hand.iter().map(|c| c.code()).collect())
            .unwrap_or_default();
        let last_play = state.last_play.as_ref().map(play_to_view);
        Some(RoomSnapshot {
            room_id: room_id.to_string(),
            players,
            turn: state.players[state.turn].id,
            last_player: state.last_player.map(|idx| state.players[idx].id),
            last_play,
            your_hand,
        })
    }

    pub fn room_connections(&self, room_id: &str) -> Option<Vec<PlayerConn>> {
        self.rooms.get(room_id).map(|room| room.players.clone())
    }

    pub fn room_state_exists(&self, room_id: &str) -> bool {
        self.rooms
            .get(room_id)
            .and_then(|room| room.state.as_ref())
            .is_some()
    }

    pub fn room_ids(&self) -> Vec<String> {
        self.rooms.keys().cloned().collect()
    }

    pub fn room_player_count(&self, room_id: &str) -> Option<usize> {
        self.rooms.get(room_id).map(|room| room.players.len())
    }

    pub fn room_started(&self, room_id: &str) -> Option<bool> {
        self.rooms.get(room_id).map(|room| room.state.is_some())
    }

    pub fn room_summaries(&self) -> Vec<RoomSummary> {
        let mut rooms = self
            .rooms
            .iter()
            .map(|(room_id, room)| RoomSummary {
                room_id: room_id.clone(),
                player_count: room.players.len(),
                started: room.state.is_some(),
                can_join: room.players.len() < 3,
            })
            .collect::<Vec<_>>();
        rooms.sort_by(|a, b| a.room_id.cmp(&b.room_id));
        rooms
    }

    fn new_room_id(&mut self) -> String {
        (0..6)
            .map(|_| self.rng.sample(Alphanumeric) as char)
            .collect::<String>()
            .to_uppercase()
    }
}

fn play_to_view(play: &Play) -> PlayView {
    PlayView {
        kind: format!("{:?}", play.kind),
        main_rank: format!("{:?}", play.main_rank),
        size: play.size,
    }
}

fn map_game_error(err: GameError) -> RoomError {
    match err {
        GameError::InvalidPlay => RoomError::InvalidPlay,
        GameError::NotYourTurn => RoomError::NotYourTurn,
        GameError::MustBeatPrevious => RoomError::MustBeatPrevious,
        GameError::CardsNotOwned => RoomError::CardsNotOwned,
        GameError::GameOver => RoomError::GameOver,
        GameError::CannotPass => RoomError::CannotPass,
    }
}

pub fn display_name_for_user(user_id: u64) -> String {
    const ADJECTIVES: &[&str] = &[
        "Brave", "Calm", "Swift", "Mighty", "Lucky", "Clever", "Silent", "Fierce", "Nimble",
        "Rapid", "Steady", "Bold", "Witty", "Sunny", "Vivid", "Lively", "Cosmic", "Iron", "Silver",
        "Golden",
    ];
    const NOUNS: &[&str] = &[
        "Panda", "Tiger", "Falcon", "Wolf", "Dragon", "Fox", "Lion", "Eagle", "Shark", "Otter",
        "Hawk", "Bear", "Leopard", "Raven", "Phoenix", "Panther", "Dolphin", "Rhino", "Viper",
        "Cobra",
    ];

    let adjective = ADJECTIVES[(user_id as usize) % ADJECTIVES.len()];
    let noun = NOUNS[(user_id.rotate_left(17) as usize) % NOUNS.len()];
    format!("{}_{}", adjective, noun)
}
