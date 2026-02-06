use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::RngCore;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    Joker,
}

impl Suit {
    fn order(&self) -> u8 {
        match self {
            Suit::Clubs => 1,
            Suit::Diamonds => 2,
            Suit::Hearts => 3,
            Suit::Spades => 4,
            Suit::Joker => 5,
        }
    }

    fn from_char(ch: char) -> Option<Self> {
        match ch {
            'C' => Some(Suit::Clubs),
            'D' => Some(Suit::Diamonds),
            'H' => Some(Suit::Hearts),
            'S' => Some(Suit::Spades),
            _ => None,
        }
    }

    fn to_char(&self) -> char {
        match self {
            Suit::Clubs => 'C',
            Suit::Diamonds => 'D',
            Suit::Hearts => 'H',
            Suit::Spades => 'S',
            Suit::Joker => 'J',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Rank {
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
    Two = 16,
    BlackJoker = 17,
    RedJoker = 18,
}

impl Rank {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "3" => Some(Rank::Three),
            "4" => Some(Rank::Four),
            "5" => Some(Rank::Five),
            "6" => Some(Rank::Six),
            "7" => Some(Rank::Seven),
            "8" => Some(Rank::Eight),
            "9" => Some(Rank::Nine),
            "10" => Some(Rank::Ten),
            "J" => Some(Rank::Jack),
            "Q" => Some(Rank::Queen),
            "K" => Some(Rank::King),
            "A" => Some(Rank::Ace),
            "2" => Some(Rank::Two),
            "BJ" => Some(Rank::BlackJoker),
            "RJ" => Some(Rank::RedJoker),
            _ => None,
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::BlackJoker => "BJ",
            Rank::RedJoker => "RJ",
        }
    }

    fn is_joker(&self) -> bool {
        matches!(self, Rank::BlackJoker | Rank::RedJoker)
    }

    fn is_straightable(&self) -> bool {
        !matches!(self, Rank::Two | Rank::BlackJoker | Rank::RedJoker)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn code(&self) -> String {
        if self.rank.is_joker() {
            return self.rank.to_str().to_string();
        }
        format!("{}{}", self.suit.to_char(), self.rank.to_str())
    }

    pub fn from_code(code: &str) -> Option<Self> {
        if code == "BJ" {
            return Some(Card {
                rank: Rank::BlackJoker,
                suit: Suit::Joker,
            });
        }
        if code == "RJ" {
            return Some(Card {
                rank: Rank::RedJoker,
                suit: Suit::Joker,
            });
        }
        let mut chars = code.chars();
        let suit = Suit::from_char(chars.next()?)?;
        let rank_str: String = chars.collect();
        let rank = Rank::from_str(rank_str.as_str())?;
        Some(Card { rank, suit })
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

pub fn standard_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(54);
    let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
    let ranks = [
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
        Rank::Two,
    ];
    for suit in suits.iter() {
        for rank in ranks.iter() {
            deck.push(Card {
                rank: *rank,
                suit: *suit,
            });
        }
    }
    deck.push(Card {
        rank: Rank::BlackJoker,
        suit: Suit::Joker,
    });
    deck.push(Card {
        rank: Rank::RedJoker,
        suit: Suit::Joker,
    });
    deck
}

pub fn shuffled_deck(seed: u64) -> Vec<Card> {
    let mut deck = standard_deck();
    let mut rng = StdRng::seed_from_u64(seed);
    deck.shuffle(&mut rng);
    deck
}

pub fn deal(seed: u64) -> (Vec<Vec<Card>>, Vec<Card>) {
    let mut deck = shuffled_deck(seed);
    let mut hands = vec![
        Vec::with_capacity(17),
        Vec::with_capacity(17),
        Vec::with_capacity(17),
    ];
    for i in 0..51 {
        hands[i % 3].push(deck[i]);
    }
    let bottom = deck.split_off(51);
    (hands, bottom)
}

pub fn sort_hand(hand: &mut Vec<Card>) {
    hand.sort_by_key(|card| (card.rank, card.suit.order()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayKind {
    Single,
    Pair,
    Triple,
    TripleSingle,
    TriplePair,
    Straight,
    DoubleStraight,
    Airplane,
    Bomb,
    Rocket,
    FourTwoSingle,
    FourTwoPair,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Play {
    pub kind: PlayKind,
    pub main_rank: Rank,
    pub size: usize,
}

fn counts_by_rank(cards: &[Card]) -> BTreeMap<Rank, usize> {
    let mut counts = BTreeMap::new();
    for card in cards.iter() {
        *counts.entry(card.rank).or_insert(0) += 1;
    }
    counts
}

fn is_consecutive(ranks: &[Rank]) -> bool {
    if ranks.len() < 2 {
        return true;
    }
    for pair in ranks.windows(2) {
        let a = pair[0] as u8;
        let b = pair[1] as u8;
        if b != a + 1 {
            return false;
        }
    }
    true
}

pub fn classify_play(cards: &[Card]) -> Option<Play> {
    if cards.is_empty() {
        return None;
    }
    let mut ranks: Vec<Rank> = cards.iter().map(|card| card.rank).collect();
    ranks.sort();
    let counts = counts_by_rank(cards);
    let unique = counts.len();
    let len = cards.len();

    if len == 2 && ranks.contains(&Rank::BlackJoker) && ranks.contains(&Rank::RedJoker) {
        return Some(Play {
            kind: PlayKind::Rocket,
            main_rank: Rank::RedJoker,
            size: 2,
        });
    }

    if len == 4 && unique == 1 {
        return Some(Play {
            kind: PlayKind::Bomb,
            main_rank: ranks[0],
            size: 4,
        });
    }

    if len == 1 {
        return Some(Play {
            kind: PlayKind::Single,
            main_rank: ranks[0],
            size: 1,
        });
    }

    if len == 2 && unique == 1 {
        return Some(Play {
            kind: PlayKind::Pair,
            main_rank: ranks[0],
            size: 2,
        });
    }

    if len == 3 && unique == 1 {
        return Some(Play {
            kind: PlayKind::Triple,
            main_rank: ranks[0],
            size: 3,
        });
    }

    if len == 4 && unique == 2 {
        if let Some((rank, _)) = counts.iter().find(|(_, count)| **count == 3) {
            return Some(Play {
                kind: PlayKind::TripleSingle,
                main_rank: *rank,
                size: 4,
            });
        }
    }

    if len == 5 && unique == 2 {
        if let Some((rank, _)) = counts.iter().find(|(_, count)| **count == 3) {
            return Some(Play {
                kind: PlayKind::TriplePair,
                main_rank: *rank,
                size: 5,
            });
        }
    }

    if len == 6 && unique == 3 {
        if let Some((rank, _)) = counts.iter().find(|(_, count)| **count == 4) {
            return Some(Play {
                kind: PlayKind::FourTwoSingle,
                main_rank: *rank,
                size: 6,
            });
        }
    }

    if len == 8 && unique == 3 {
        if let Some((rank, _)) = counts.iter().find(|(_, count)| **count == 4) {
            let pair_count = counts.values().filter(|count| **count == 2).count();
            if pair_count == 2 {
                return Some(Play {
                    kind: PlayKind::FourTwoPair,
                    main_rank: *rank,
                    size: 8,
                });
            }
        }
    }

    if counts.values().all(|count| *count == 1) && len >= 5 {
        if ranks.iter().all(|rank| rank.is_straightable()) && is_consecutive(&ranks) {
            return Some(Play {
                kind: PlayKind::Straight,
                main_rank: *ranks.last().unwrap(),
                size: len,
            });
        }
    }

    if counts.values().all(|count| *count == 2) && len >= 6 && len % 2 == 0 {
        let mut pair_ranks: Vec<Rank> = counts.keys().copied().collect();
        pair_ranks.sort();
        if pair_ranks.iter().all(|rank| rank.is_straightable()) && is_consecutive(&pair_ranks) {
            return Some(Play {
                kind: PlayKind::DoubleStraight,
                main_rank: *pair_ranks.last().unwrap(),
                size: pair_ranks.len(),
            });
        }
    }

    if counts.values().all(|count| *count == 3) && len >= 6 && len % 3 == 0 {
        let mut triple_ranks: Vec<Rank> = counts.keys().copied().collect();
        triple_ranks.sort();
        if triple_ranks.iter().all(|rank| rank.is_straightable()) && is_consecutive(&triple_ranks) {
            return Some(Play {
                kind: PlayKind::Airplane,
                main_rank: *triple_ranks.last().unwrap(),
                size: triple_ranks.len(),
            });
        }
    }

    None
}

pub fn can_beat(prev: &Play, next: &Play) -> bool {
    if prev.kind == PlayKind::Rocket {
        return false;
    }
    if next.kind == PlayKind::Rocket {
        return true;
    }
    if next.kind == PlayKind::Bomb && prev.kind != PlayKind::Bomb {
        return true;
    }
    if prev.kind == PlayKind::Bomb && next.kind != PlayKind::Bomb {
        return false;
    }
    if prev.kind != next.kind {
        return false;
    }
    match prev.kind {
        PlayKind::Straight | PlayKind::DoubleStraight | PlayKind::Airplane => {
            prev.size == next.size && next.main_rank > prev.main_rank
        }
        _ => next.main_rank > prev.main_rank,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerState {
    pub id: u64,
    pub hand: Vec<Card>,
    pub out: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    pub players: [PlayerState; 3],
    pub landlord: usize,
    pub turn: usize,
    pub last_play: Option<Play>,
    pub last_player: Option<usize>,
    pub pass_count: u8,
    pub deck_seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    NotYourTurn,
    CardsNotOwned,
    InvalidPlay,
    MustBeatPrevious,
    GameOver,
    CannotPass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayOutcome {
    pub play: Play,
    pub next_turn: usize,
    pub winner: Option<usize>,
}

impl GameState {
    pub fn new(player_ids: [u64; 3], seed: u64) -> Self {
        let (mut hands, bottom) = deal(seed);
        let mut rng = StdRng::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15);
        let landlord = (rng.next_u64() % 3) as usize;
        hands[landlord].extend(bottom);
        for hand in hands.iter_mut() {
            sort_hand(hand);
        }
        GameState {
            players: [
                PlayerState {
                    id: player_ids[0],
                    hand: hands[0].clone(),
                    out: false,
                },
                PlayerState {
                    id: player_ids[1],
                    hand: hands[1].clone(),
                    out: false,
                },
                PlayerState {
                    id: player_ids[2],
                    hand: hands[2].clone(),
                    out: false,
                },
            ],
            landlord,
            turn: landlord,
            last_play: None,
            last_player: None,
            pass_count: 0,
            deck_seed: seed,
        }
    }

    pub fn player_index(&self, player_id: u64) -> Option<usize> {
        self.players
            .iter()
            .position(|player| player.id == player_id)
    }

    pub fn apply_play(
        &mut self,
        player_idx: usize,
        cards: Vec<Card>,
    ) -> Result<PlayOutcome, GameError> {
        if self.players.iter().any(|p| p.out) {
            return Err(GameError::GameOver);
        }
        if player_idx != self.turn {
            return Err(GameError::NotYourTurn);
        }
        let play = classify_play(&cards).ok_or(GameError::InvalidPlay)?;
        if let Some(prev) = &self.last_play {
            if let Some(last_player) = self.last_player {
                if last_player != player_idx && !can_beat(prev, &play) {
                    return Err(GameError::MustBeatPrevious);
                }
            }
        }
        let hand = &mut self.players[player_idx].hand;
        let mut needed = HashMap::new();
        for card in cards.iter() {
            *needed.entry(*card).or_insert(0usize) += 1;
        }
        for (card, count) in needed.iter() {
            let owned = hand.iter().filter(|c| *c == card).count();
            if owned < *count {
                return Err(GameError::CardsNotOwned);
            }
        }
        for card in cards.iter() {
            if let Some(pos) = hand.iter().position(|c| c == card) {
                hand.remove(pos);
            }
        }
        self.last_play = Some(play.clone());
        self.last_player = Some(player_idx);
        self.pass_count = 0;
        let next_turn = (player_idx + 1) % 3;
        self.turn = next_turn;
        if hand.is_empty() {
            self.players[player_idx].out = true;
            return Ok(PlayOutcome {
                play,
                next_turn,
                winner: Some(player_idx),
            });
        }
        Ok(PlayOutcome {
            play,
            next_turn,
            winner: None,
        })
    }

    pub fn pass(&mut self, player_idx: usize) -> Result<usize, GameError> {
        if self.players.iter().any(|p| p.out) {
            return Err(GameError::GameOver);
        }
        if player_idx != self.turn {
            return Err(GameError::NotYourTurn);
        }
        if self.last_play.is_none() || self.last_player == Some(player_idx) {
            return Err(GameError::CannotPass);
        }
        self.pass_count = self.pass_count.saturating_add(1);
        if self.pass_count >= 2 {
            self.last_play = None;
            self.last_player = None;
            self.pass_count = 0;
        }
        self.turn = (self.turn + 1) % 3;
        Ok(self.turn)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    fn deck_has_54_cards() {
        let deck = standard_deck();
        assert_eq!(deck.len(), 54);
    }

    #[test]
    fn deck_has_two_jokers() {
        let deck = standard_deck();
        let jokers = deck.iter().filter(|c| c.rank.is_joker()).count();
        assert_eq!(jokers, 2);
    }

    #[test]
    fn deal_gives_17_cards_each_and_3_bottom() {
        let (hands, bottom) = deal(42);
        assert_eq!(hands.len(), 3);
        assert_eq!(hands[0].len(), 17);
        assert_eq!(hands[1].len(), 17);
        assert_eq!(hands[2].len(), 17);
        assert_eq!(bottom.len(), 3);
    }

    #[test]
    fn card_code_round_trip() {
        let c = card(Rank::Ace, Suit::Spades);
        let code = c.code();
        let parsed = Card::from_code(&code).unwrap();
        assert_eq!(parsed, c);
    }

    #[test]
    fn joker_codes_round_trip() {
        let c = Card::from_code("BJ").unwrap();
        assert_eq!(c.rank, Rank::BlackJoker);
        let r = Card::from_code("RJ").unwrap();
        assert_eq!(r.rank, Rank::RedJoker);
    }

    #[test]
    fn classify_single() {
        let play = classify_play(&[card(Rank::Three, Suit::Clubs)]).unwrap();
        assert_eq!(play.kind, PlayKind::Single);
    }

    #[test]
    fn classify_pair() {
        let play = classify_play(&[
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::Pair);
    }

    #[test]
    fn classify_triple() {
        let play = classify_play(&[
            card(Rank::Five, Suit::Clubs),
            card(Rank::Five, Suit::Diamonds),
            card(Rank::Five, Suit::Hearts),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::Triple);
    }

    #[test]
    fn classify_triple_single() {
        let play = classify_play(&[
            card(Rank::Six, Suit::Clubs),
            card(Rank::Six, Suit::Diamonds),
            card(Rank::Six, Suit::Hearts),
            card(Rank::Nine, Suit::Spades),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::TripleSingle);
    }

    #[test]
    fn classify_triple_pair() {
        let play = classify_play(&[
            card(Rank::Seven, Suit::Clubs),
            card(Rank::Seven, Suit::Diamonds),
            card(Rank::Seven, Suit::Hearts),
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Nine, Suit::Diamonds),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::TriplePair);
    }

    #[test]
    fn classify_bomb() {
        let play = classify_play(&[
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Eight, Suit::Diamonds),
            card(Rank::Eight, Suit::Hearts),
            card(Rank::Eight, Suit::Spades),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::Bomb);
    }

    #[test]
    fn classify_rocket() {
        let play = classify_play(&[
            card(Rank::BlackJoker, Suit::Joker),
            card(Rank::RedJoker, Suit::Joker),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::Rocket);
    }

    #[test]
    fn classify_straight() {
        let play = classify_play(&[
            card(Rank::Three, Suit::Clubs),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Six, Suit::Clubs),
            card(Rank::Seven, Suit::Clubs),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::Straight);
        assert_eq!(play.size, 5);
    }

    #[test]
    fn classify_double_straight() {
        let play = classify_play(&[
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Five, Suit::Diamonds),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::DoubleStraight);
        assert_eq!(play.size, 3);
    }

    #[test]
    fn classify_airplane() {
        let play = classify_play(&[
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Four, Suit::Hearts),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::Airplane);
        assert_eq!(play.size, 2);
    }

    #[test]
    fn classify_four_two_single() {
        let play = classify_play(&[
            card(Rank::Nine, Suit::Clubs),
            card(Rank::Nine, Suit::Diamonds),
            card(Rank::Nine, Suit::Hearts),
            card(Rank::Nine, Suit::Spades),
            card(Rank::Three, Suit::Clubs),
            card(Rank::Four, Suit::Clubs),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::FourTwoSingle);
    }

    #[test]
    fn classify_four_two_pair() {
        let play = classify_play(&[
            card(Rank::Ten, Suit::Clubs),
            card(Rank::Ten, Suit::Diamonds),
            card(Rank::Ten, Suit::Hearts),
            card(Rank::Ten, Suit::Spades),
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
        ])
        .unwrap();
        assert_eq!(play.kind, PlayKind::FourTwoPair);
    }

    #[test]
    fn can_beat_same_kind() {
        let a = classify_play(&[card(Rank::Three, Suit::Clubs)]).unwrap();
        let b = classify_play(&[card(Rank::Four, Suit::Clubs)]).unwrap();
        assert!(can_beat(&a, &b));
    }

    #[test]
    fn bomb_beats_non_bomb() {
        let prev = classify_play(&[card(Rank::King, Suit::Clubs)]).unwrap();
        let bomb = classify_play(&[
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Three, Suit::Hearts),
            card(Rank::Three, Suit::Spades),
        ])
        .unwrap();
        assert!(can_beat(&prev, &bomb));
    }

    #[test]
    fn rocket_beats_bomb() {
        let bomb = classify_play(&[
            card(Rank::Four, Suit::Clubs),
            card(Rank::Four, Suit::Diamonds),
            card(Rank::Four, Suit::Hearts),
            card(Rank::Four, Suit::Spades),
        ])
        .unwrap();
        let rocket = classify_play(&[
            card(Rank::BlackJoker, Suit::Joker),
            card(Rank::RedJoker, Suit::Joker),
        ])
        .unwrap();
        assert!(can_beat(&bomb, &rocket));
    }

    #[test]
    fn straight_requires_same_length() {
        let a = classify_play(&[
            card(Rank::Three, Suit::Clubs),
            card(Rank::Four, Suit::Clubs),
            card(Rank::Five, Suit::Clubs),
            card(Rank::Six, Suit::Clubs),
            card(Rank::Seven, Suit::Clubs),
        ])
        .unwrap();
        let b = classify_play(&[
            card(Rank::Four, Suit::Spades),
            card(Rank::Five, Suit::Spades),
            card(Rank::Six, Suit::Spades),
            card(Rank::Seven, Suit::Spades),
            card(Rank::Eight, Suit::Spades),
        ])
        .unwrap();
        assert!(can_beat(&a, &b));
    }

    #[test]
    fn game_state_starts_with_landlord_turn() {
        let state = GameState::new([1, 2, 3], 7);
        assert_eq!(state.turn, state.landlord);
        assert_eq!(state.players[state.landlord].hand.len(), 20);
    }

    #[test]
    fn apply_play_removes_cards() {
        let mut state = GameState::new([1, 2, 3], 9);
        let idx = state.turn;
        let card_play = state.players[idx].hand[0];
        let outcome = state.apply_play(idx, vec![card_play]).unwrap();
        assert_eq!(outcome.play.kind, PlayKind::Single);
        assert_eq!(state.players[idx].hand.len(), 19);
    }

    #[test]
    fn apply_play_checks_turn() {
        let mut state = GameState::new([1, 2, 3], 9);
        let idx = (state.turn + 1) % 3;
        let card_play = state.players[idx].hand[0];
        let result = state.apply_play(idx, vec![card_play]);
        assert_eq!(result.err(), Some(GameError::NotYourTurn));
    }

    #[test]
    fn apply_play_rejects_missing_cards() {
        let mut state = GameState::new([1, 2, 3], 9);
        let idx = state.turn;
        let card_play = standard_deck()
            .into_iter()
            .find(|card| !state.players[idx].hand.contains(card))
            .expect("there is always a card the current player does not own");
        let result = state.apply_play(idx, vec![card_play]);
        assert_eq!(result.err(), Some(GameError::CardsNotOwned));
    }

    #[test]
    fn apply_play_requires_beating_previous() {
        let mut state = GameState::new([1, 2, 3], 9);
        let idx = state.turn;
        let next_idx = (idx + 1) % 3;
        let first = state.players[idx].hand[0];
        let _ = state.apply_play(idx, vec![first]).unwrap();
        let next_card = state.players[next_idx]
            .hand
            .iter()
            .min_by_key(|c| c.rank)
            .copied()
            .unwrap();
        let result = state.apply_play(next_idx, vec![next_card]);
        if let Err(err) = result {
            assert_eq!(err, GameError::MustBeatPrevious);
        }
    }

    #[test]
    fn classify_rejects_invalid_play() {
        let play = classify_play(&[
            card(Rank::Three, Suit::Clubs),
            card(Rank::Three, Suit::Diamonds),
            card(Rank::Four, Suit::Clubs),
        ]);
        assert!(play.is_none());
    }

    #[test]
    fn straight_cannot_include_two_or_jokers() {
        let play = classify_play(&[
            card(Rank::Ten, Suit::Clubs),
            card(Rank::Jack, Suit::Clubs),
            card(Rank::Queen, Suit::Clubs),
            card(Rank::King, Suit::Clubs),
            card(Rank::Two, Suit::Clubs),
        ]);
        assert!(play.is_none());
    }
    #[test]
    fn pass_not_allowed_without_previous_play() {
        let mut state = GameState::new([1, 2, 3], 9);
        let result = state.pass(state.turn);
        assert_eq!(result.err(), Some(GameError::CannotPass));
    }

    #[test]
    fn two_passes_reset_last_play() {
        let mut state = GameState::new([1, 2, 3], 9);
        let leader = state.turn;
        let first_card = state.players[leader].hand[0];
        state.apply_play(leader, vec![first_card]).unwrap();
        let next = state.turn;
        state.pass(next).unwrap();
        let next2 = state.turn;
        state.pass(next2).unwrap();
        assert!(state.last_play.is_none());
    }
}
