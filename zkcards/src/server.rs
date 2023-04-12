use crate::{
    error::GameErrors, player::Surrogate, user_card::encode_cards, AggregatePublicKey, Card,
    CardParameters, CardProtocol, MaskedCard, ProofShuffle, Scalar,
};
use ark_std::One;
use barnett::BarnettSmartProtocol;
use rand::thread_rng;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

type ZkCardGameInstance = u8;

pub struct ZkCardGame {
    config: ZkGameConfig,
    parameters: CardParameters,
    players: Vec<(u32, Surrogate)>,
    basic: Option<ZkCardGameInitInfo>,
    #[allow(dead_code)]
    instance: Option<ZkCardGameInstance>,
}

struct ZkCardGameInitInfo {
    shared_key: AggregatePublicKey,
    #[allow(dead_code)]
    initial_cards: HashMap<Card, Vec<u8>>,
    initial_deck: Vec<MaskedCard /*, MaskedProof*/>,
    next_shuffle_player: Option<u32>,
    shuffled_decks: Vec<(
        Vec<MaskedCard>,
        Option<(u32 /*player index*/, ProofShuffle)>,
    )>,
}

impl ZkCardGameInitInfo {
    pub fn new(
        shared_key: AggregatePublicKey,
        initial_cards: HashMap<Card, Vec<u8>>,
        initial_deck: Vec<MaskedCard>,
    ) -> Self {
        Self {
            shared_key,
            initial_cards,
            initial_deck,
            next_shuffle_player: None,
            shuffled_decks: vec![],
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ZkGameConfig {
    m: usize,
    n: usize,
    players_num: usize,
}

impl ZkGameConfig {
    pub fn new(m: usize, n: usize, players: usize) -> Self {
        Self {
            m,
            n,
            players_num: players,
        }
    }

    pub fn m(&self) -> usize {
        self.m
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn players(&self) -> usize {
        self.players_num
    }

    pub fn num_of_cards(&self) -> usize {
        self.m.saturating_mul(self.n)
    }
}

impl ZkCardGame {
    pub fn new(config: ZkGameConfig) -> anyhow::Result<Self, GameErrors> {
        match config.m.checked_mul(config.n) {
            Some(res) if res <= 52 && config.players_num > 0 => {
                let rng = &mut thread_rng();
                let parameters = CardProtocol::setup(rng, config.m, config.n)?;
                Ok(Self {
                    config,
                    parameters: parameters.into(),
                    players: vec![],
                    basic: None,
                    instance: None,
                })
            }
            _ => Err(GameErrors::InvalidParameters),
        }
    }

    pub fn setup(&mut self) -> anyhow::Result<(), GameErrors> {
        // check if the game is ready to setup
        if !self.is_ready() {
            return Err(GameErrors::NotReady);
        }

        let rng = &mut thread_rng();
        let initial_cards = encode_cards(rng, self.config.num_of_cards());

        let key_proof_info = self
            .players
            .iter()
            .map(|(_, p)| (p.pk, p.proof_key.into(), p.name.clone()))
            .collect::<Vec<_>>();

        // Each player should run this computation. Alternatively, it can be ran by a smart contract
        let joint_pk =
            CardProtocol::compute_aggregate_key((&self.parameters).into(), &key_proof_info)?;

        let deck = initial_cards
            .keys()
            .map(|card| {
                let inner_card = card.clone().into();
                CardProtocol::mask(
                    rng,
                    (&self.parameters).into(),
                    &joint_pk,
                    &inner_card,
                    &Scalar::one(),
                )
                .map(|x| x.0.into())
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.basic = Some(ZkCardGameInitInfo::new(joint_pk, initial_cards, deck));

        Ok(())
    }

    pub fn initial_deck(&self) -> anyhow::Result<Vec<MaskedCard>, GameErrors> {
        if let Some(basic) = self.basic.as_ref() {
            Ok(basic.initial_deck.clone())
        } else {
            Err(GameErrors::NotReady)
        }
    }

    pub fn register_players(&mut self, mut players: Vec<(u32, Surrogate)>) {
        self.players.append(&mut players);
    }

    pub fn register_shuffled_deck(
        &mut self,
        deck: Vec<MaskedCard>,
        proof: Option<ProofShuffle>,
        next_shuffle_player: u32,
    ) -> anyhow::Result<(), GameErrors> {
        // The proof could be None for the initial deck
        if let Some(basic) = self.basic.as_mut() {
            let with_player = proof.map(|p| (basic.next_shuffle_player.unwrap(), p));
            basic.shuffled_decks.push((deck, with_player));
            basic.next_shuffle_player = Some(next_shuffle_player);
            Ok(())
        } else {
            Err(GameErrors::NotReady)
        }
    }

    pub fn next_shuffle_player(&mut self) -> anyhow::Result<u32, GameErrors> {
        if self.is_all_shuffled() {
            return Err(GameErrors::AllShuffled);
        } else if !self.is_ready() || !self.ready_to_shuffle() {
            return Err(GameErrors::NotReady);
        } else {
            todo!()
        }
    }

    pub fn current_shuffle_player(&self) -> anyhow::Result<u32, GameErrors> {
        todo!()
    }

    pub fn next_card(&self) -> anyhow::Result<u32, GameErrors> {
        todo!()
    }

    pub fn is_all_shuffled(&self) -> bool {
        let players_num = self.players.len();
        if players_num == 0 {
            return false;
        }
        let expect_shuffled_decks = players_num + 1;
        self.basic
            .as_ref()
            .map(|b| b.shuffled_decks.len() == expect_shuffled_decks)
            .unwrap_or_default()
    }

    pub fn parameters(&self) -> Vec<u8> {
        serde_json::to_vec(&self.parameters).unwrap()
    }

    pub fn joint_pk(&self) -> anyhow::Result<AggregatePublicKey, GameErrors> {
        if let Some(basic) = self.basic.as_ref() {
            Ok(basic.shared_key.clone())
        } else {
            Err(GameErrors::NotReady)
        }
    }

    fn is_ready(&self) -> bool {
        !self.players.is_empty()
    }

    pub fn ready_to_shuffle(&self) -> bool {
        self.players.len() == self.config.players_num
    }
}
