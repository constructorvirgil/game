use server::rooms::{PlayerConn, RoomError, RoomManager};
use std::collections::HashSet;

fn join_three(manager: &mut RoomManager, room_id: &str) -> [u64; 3] {
    let ids = [10u64, 11u64, 12u64];
    for id in ids.iter() {
        manager
            .join_room(room_id, PlayerConn { id: *id, tx: None })
            .unwrap();
    }
    ids
}

fn current_turn(manager: &RoomManager, room_id: &str, player_id: u64) -> u64 {
    manager
        .snapshot_for(room_id, player_id)
        .map(|s| s.turn)
        .unwrap()
}

fn hand_for(manager: &RoomManager, room_id: &str, player_id: u64) -> Vec<String> {
    manager
        .snapshot_for(room_id, player_id)
        .map(|s| s.your_hand)
        .unwrap()
}

fn rank_weight_from_view(rank: &str) -> u8 {
    match rank {
        "Three" => 3,
        "Four" => 4,
        "Five" => 5,
        "Six" => 6,
        "Seven" => 7,
        "Eight" => 8,
        "Nine" => 9,
        "Ten" => 10,
        "Jack" => 11,
        "Queen" => 12,
        "King" => 13,
        "Ace" => 14,
        "Two" => 16,
        "BlackJoker" => 17,
        "RedJoker" => 18,
        other => panic!("unexpected rank view: {other}"),
    }
}

fn rank_weight_from_code(card_code: &str) -> u8 {
    game_core::Card::from_code(card_code)
        .map(|card| card.rank as u8)
        .unwrap()
}

fn simulate_until_game_over(manager: &mut RoomManager, room_id: &str, observer: u64) -> u64 {
    for _step in 0..800 {
        let snapshot = manager.snapshot_for(room_id, observer).unwrap();
        let turn = snapshot.turn;
        let hand = hand_for(manager, room_id, turn);
        assert!(
            !hand.is_empty(),
            "active player {turn} has no hand cards before winner is emitted"
        );

        let maybe_card = if let Some(last_play) = snapshot.last_play.as_ref() {
            if snapshot.last_player == Some(turn) {
                Some(hand[0].clone())
            } else if last_play.kind == "Single" {
                let prev_rank = rank_weight_from_view(&last_play.main_rank);
                hand.into_iter()
                    .find(|code| rank_weight_from_code(code) > prev_rank)
            } else {
                None
            }
        } else {
            Some(hand[0].clone())
        };

        if let Some(card_code) = maybe_card {
            let card = game_core::Card::from_code(&card_code).unwrap();
            if let Some(winner_id) = manager.apply_play(room_id, turn, vec![card]).unwrap() {
                return winner_id;
            }
        } else {
            manager.pass_turn(room_id, turn).unwrap();
        }
    }
    panic!("simulation did not finish within step limit");
}

#[test]
fn create_room_generates_code() {
    let mut manager = RoomManager::with_seed(1);
    let room_id = manager.create_room();
    assert_eq!(room_id.len(), 6);
    assert!(room_id
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
}

#[test]
fn join_room_happy_path() {
    let mut manager = RoomManager::with_seed(2);
    let room_id = manager.create_room();
    let result = manager.join_room(&room_id, PlayerConn { id: 1, tx: None });
    assert!(result.is_ok());
}

#[test]
fn join_room_rejects_duplicates() {
    let mut manager = RoomManager::with_seed(3);
    let room_id = manager.create_room();
    let player = PlayerConn { id: 1, tx: None };
    manager.join_room(&room_id, player.clone()).unwrap();
    let result = manager.join_room(&room_id, player);
    assert_eq!(result.err(), Some(RoomError::AlreadyJoined));
}

#[test]
fn join_room_rejects_when_full() {
    let mut manager = RoomManager::with_seed(4);
    let room_id = manager.create_room();
    join_three(&mut manager, &room_id);
    let result = manager.join_room(&room_id, PlayerConn { id: 99, tx: None });
    assert_eq!(result.err(), Some(RoomError::Full));
}

#[test]
fn start_game_requires_three_players() {
    let mut manager = RoomManager::with_seed(5);
    let room_id = manager.create_room();
    manager
        .join_room(&room_id, PlayerConn { id: 1, tx: None })
        .unwrap();
    let result = manager.start_if_ready(&room_id, 42);
    assert_eq!(result.err(), Some(RoomError::NotReady));
}

#[test]
fn start_game_after_three_players() {
    let mut manager = RoomManager::with_seed(6);
    let room_id = manager.create_room();
    join_three(&mut manager, &room_id);
    let result = manager.start_if_ready(&room_id, 42);
    assert!(result.is_ok());
    assert!(manager.room_state_exists(&room_id));
}

#[test]
fn snapshot_contains_hand() {
    let mut manager = RoomManager::with_seed(7);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 101).unwrap();
    let snapshot = manager.snapshot_for(&room_id, ids[0]).unwrap();
    assert_eq!(snapshot.your_hand.len(), 17);
}

#[test]
fn apply_play_updates_state() {
    let mut manager = RoomManager::with_seed(8);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 77).unwrap();
    let turn = current_turn(&manager, &room_id, ids[0]);
    let hand = hand_for(&manager, &room_id, turn);
    let card_code = hand[0].clone();
    let card = game_core::Card::from_code(&card_code).unwrap();
    let result = manager.apply_play(&room_id, turn, vec![card]);
    assert!(result.is_ok());
}

#[test]
fn pass_requires_previous_play() {
    let mut manager = RoomManager::with_seed(9);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 88).unwrap();
    let turn = current_turn(&manager, &room_id, ids[0]);
    let result = manager.pass_turn(&room_id, turn);
    assert_eq!(result.err(), Some(RoomError::CannotPass));
}

#[test]
fn remove_player_deletes_room_when_empty() {
    let mut manager = RoomManager::with_seed(10);
    let room_id = manager.create_room();
    manager
        .join_room(&room_id, PlayerConn { id: 1, tx: None })
        .unwrap();
    let result = manager.remove_connection(&room_id, 1).unwrap();
    assert!(result.room_deleted);
    assert!(manager.room_ids().is_empty());
}

#[test]
fn remove_player_during_active_game_interrupts_round() {
    let mut manager = RoomManager::with_seed(100);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 1234).unwrap();

    let result = manager.remove_connection(&room_id, ids[1]).unwrap();
    assert!(!result.room_deleted);
    assert!(result.game_interrupted);
    assert_eq!(result.player_count, 2);
    assert!(!manager.room_state_exists(&room_id));

    manager
        .join_room(&room_id, PlayerConn { id: 99, tx: None })
        .unwrap();
    manager.start_if_ready(&room_id, 1235).unwrap();
    assert!(manager.room_state_exists(&room_id));
}

#[test]
fn apply_play_rejects_invalid_cards() {
    let mut manager = RoomManager::with_seed(11);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 99).unwrap();
    let turn = current_turn(&manager, &room_id, ids[0]);
    let hand = hand_for(&manager, &room_id, turn);
    let hand_set: HashSet<String> = hand.into_iter().collect();
    let missing = game_core::standard_deck()
        .iter()
        .map(|c| c.code())
        .find(|code| !hand_set.contains(code))
        .unwrap();
    let card = game_core::Card::from_code(&missing).unwrap();
    let result = manager.apply_play(&room_id, turn, vec![card]);
    assert_eq!(result.err(), Some(RoomError::CardsNotOwned));
}

#[test]
fn full_game_simulation_reaches_game_over() {
    let mut manager = RoomManager::with_seed(12);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 2026).unwrap();

    let winner_id = simulate_until_game_over(&mut manager, &room_id, ids[0]);
    let winner_snapshot = manager.snapshot_for(&room_id, winner_id).unwrap();

    assert_eq!(winner_snapshot.your_hand.len(), 0);
}

#[test]
fn restart_game_resets_round_after_game_over() {
    let mut manager = RoomManager::with_seed(13);
    let room_id = manager.create_room();
    let ids = join_three(&mut manager, &room_id);
    manager.start_if_ready(&room_id, 2027).unwrap();

    let _winner = simulate_until_game_over(&mut manager, &room_id, ids[0]);
    manager.restart_game(&room_id, ids[0], 2028).unwrap();

    for id in ids {
        let snapshot = manager.snapshot_for(&room_id, id).unwrap();
        assert!(!snapshot.your_hand.is_empty());
    }
}
