use crate::{
    error::GameErrors, player::Surrogate, user_card::encode_cards, Card, CardParameters,
    CardProtocol, MaskedCard, ProofShuffle, Scalar,
};
use ark_std::One;
use barnett::BarnettSmartProtocol;
use rand::thread_rng;
use std::collections::HashMap;

type ZkCardGameInstance = u8;

pub struct ZkCardGame {
    config: ZkGameConfig,
    players: Vec<Surrogate>,
    basic: Option<ZkCardGameInitInfo>,
    instance: Option<ZkCardGameInstance>,
}

struct ZkCardGameInitInfo {
    parameters: CardParameters,
    initial_cards: HashMap<Card, Vec<u8>>,
    initial_deck: Vec<MaskedCard>,
    shuffled_decks: Vec<(Vec<MaskedCard>, u32 /*player index*/, ProofShuffle)>,
}

impl ZkCardGameInitInfo {
    pub fn new(
        parameters: CardParameters,
        initial_cards: HashMap<Card, Vec<u8>>,
        initial_deck: Vec<MaskedCard>,
    ) -> Self {
        Self {
            parameters,
            initial_cards,
            initial_deck,
            shuffled_decks: vec![],
        }
    }

    pub fn add_shuffled_deck(&mut self, player: u32, proof: ProofShuffle, deck: Vec<MaskedCard>) {
        self.shuffled_decks.push((deck, player, proof));
    }
}

#[derive(Eq, PartialEq)]
pub struct ZkGameConfig {
    m: usize,
    n: usize,
}

impl ZkGameConfig {
    pub fn new(m: usize, n: usize) -> Self {
        Self { m, n }
    }

    pub fn m(&self) -> usize {
        self.m
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn num_of_cards(&self) -> usize {
        self.m.saturating_mul(self.n)
    }
}

impl ZkCardGame {
    pub fn new(config: ZkGameConfig) -> Option<Self> {
        match config.m.checked_mul(config.n) {
            Some(res) if res <= 52 => Some(Self {
                config,
                players: vec![],
                basic: None,
                instance: None,
            }),
            _ => None,
        }
    }

    pub fn setup(&mut self) -> anyhow::Result<(), GameErrors> {
        // check if the game is ready to setup
        if !self.is_ready() {
            return Err(GameErrors::NotReady);
        }

        let rng = &mut thread_rng();
        let parameters = CardProtocol::setup(rng, self.config.m(), self.config.n())?;
        let initial_cards = encode_cards(rng, self.config.num_of_cards());

        let key_proof_info = self
            .players
            .iter()
            .map(|p| (p.pk, p.proof_key, p.name.clone()))
            .collect::<Vec<_>>();

        // Each player should run this computation. Alternatively, it can be ran by a smart contract
        let joint_pk = CardProtocol::compute_aggregate_key(&parameters, &key_proof_info)?;

        let deck = initial_cards
            .keys()
            .map(|card| {
                CardProtocol::mask(rng, &parameters, &joint_pk, &card, &Scalar::one()).map(|x| x.0)
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.basic = Some(ZkCardGameInitInfo::new(parameters, initial_cards, deck));

        Ok(())
    }

    pub fn register_players(&mut self, mut players: Vec<Surrogate>) {
        self.players.append(&mut players);
    }

    fn is_ready(&self) -> bool {
        !self.players.is_empty()
    }
}
