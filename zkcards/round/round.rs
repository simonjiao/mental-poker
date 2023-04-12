use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use zkcards::{
    ark_de, ark_se,
    player::{Player, Surrogate},
    server::{ZkCardGame, ZkGameConfig},
    AggregatePublicKey, CardParameters, MaskedCard, ProofRemasking, ProofShuffle, RevealToken,
};

const PLAYERS_NUM: u32 = 4;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (srv_tx, mut srv_rx) = tokio::sync::mpsc::channel(100);

    let mut player_txs = vec![];
    let mut player_rxs = vec![];
    for _ in 0..PLAYERS_NUM {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        player_txs.push(tx);
        player_rxs.push(rx);
    }

    // create a game instance
    let config = ZkGameConfig::new(13, 4, 4);
    let msg = C2SOp::CreateInstance(config);
    let raw_msg = serde_json::to_vec(&msg).unwrap();
    srv_tx.send(raw_msg).await.unwrap();

    let handle = tokio::spawn(async move {
        let mut instance = None;
        while let Some(msg) = srv_rx.recv().await {
            let op = serde_json::from_slice::<C2SOp>(msg.as_slice()).unwrap();
            match op {
                C2SOp::CreateInstance(config) => {
                    println!("create game {config:?}");
                    instance = Some(ZkCardGame::new(config).unwrap());
                    let pp = instance.as_ref().unwrap().parameters();

                    for player in &player_txs {
                        let msg = S2COp::GameParam(pp.clone());
                        let raw_msg = serde_json::to_vec(&msg).unwrap();
                        player.send(raw_msg).await.unwrap();
                    }
                }
                C2SOp::CheckIn(index, player) => {
                    println!("player {index} registered");
                    // TODO: verify public key proof
                    instance
                        .as_mut()
                        .unwrap()
                        .register_players(vec![(index, player)]);

                    // If all players have check-in, we setup the game, and request players to shuffle cards
                    if instance.as_ref().unwrap().ready_to_shuffle() {
                        instance
                            .as_mut()
                            .unwrap()
                            .setup()
                            .expect("failed to setup a new game");

                        let deck = instance.as_ref().unwrap().initial_deck().unwrap();
                        let first_shuffle_player = {
                            let mut rng = thread_rng();
                            let num: u32 = rng.gen();
                            num % PLAYERS_NUM
                        };
                        for player in &player_txs {
                            let msg = serde_json::to_vec(&S2COp::NextShuffle(
                                Some(first_shuffle_player),
                                None,
                                deck.clone(),
                                ProofOrPk::JointPk(instance.as_ref().unwrap().joint_pk().unwrap()),
                            ))
                            .unwrap();
                            player.send(msg).await.unwrap();
                        }

                        // This deck has not been shuffled by any players.
                        instance
                            .as_mut()
                            .unwrap()
                            .register_shuffled_deck(deck, None, first_shuffle_player)
                            .unwrap();
                    }
                }

                C2SOp::ShuffledCards(original, deck, proof) => {
                    let current_shuffle_player =
                        instance.as_ref().unwrap().current_shuffle_player().unwrap();
                    // TODO: verify proof shuffle
                    instance
                        .as_mut()
                        .unwrap()
                        .register_shuffled_deck(
                            deck.clone(),
                            Some(proof.clone()),
                            current_shuffle_player,
                        )
                        .unwrap();

                    let next_shuffle_player = instance.as_mut().unwrap().next_shuffle_player().ok();
                    let msg = S2COp::NextShuffle(
                        next_shuffle_player,
                        Some(original),
                        deck,
                        ProofOrPk::ProofTwo(proof),
                    );
                    let raw_msg = serde_json::to_vec(&msg).unwrap();
                    for player in &player_txs {
                        player.send(raw_msg.clone()).await.unwrap();
                    }
                }

                C2SOp::CheckOut(_, _) => {
                    todo!()
                }

                C2SOp::PeekCard(_) => {
                    todo!()
                }

                C2SOp::RevealedCard(_) => {
                    todo!()
                }

                C2SOp::OpenCard(_) => {
                    todo!()
                }

                C2SOp::RequestCards(index, _start, _num) => {
                    let player = player_txs.get(index);
                    if instance.as_ref().unwrap().is_all_shuffled() && player.is_some() {
                        // next_card() must update the index
                        let card = if let Ok(next_card) = instance.as_mut().unwrap().next_card() {
                            S2COp::ReceiveCard(vec![next_card])
                        } else {
                            // no more cards
                            S2COp::ReceiveCard(vec![])
                        };
                        let msg = serde_json::to_vec(&card).unwrap();
                        player.send(msg).unwrap();
                    }
                }
            }
        }
    });

    let new_player = |i, pp: Vec<u8>| async move {
        let param = serde_json::from_slice::<CardParameters>(pp.as_slice()).unwrap();
        let name = format!("player-{i}");
        let rng = &mut thread_rng();
        (
            Some(Player::new(rng, &param, &name.as_bytes().to_vec()).unwrap()),
            Some(param),
        )
    };

    for (i, mut rx) in player_rxs.into_iter().enumerate() {
        let srv_tx = srv_tx.clone();
        tokio::spawn(async move {
            let mut player = None;
            let mut param = None;
            let mut joint_pk = None;
            let mut final_deck = vec![];
            while let Some(msg) = rx.recv().await {
                let msg = serde_json::from_slice::<S2COp>(msg.as_slice()).unwrap();
                match msg {
                    S2COp::GameParam(pp) => {
                        (player, param) = new_player(i, pp).await;

                        let msg = C2SOp::CheckIn(i as u32, player.as_ref().unwrap().surrogate());
                        let raw_msg = serde_json::to_vec(&msg).unwrap();
                        srv_tx.send(raw_msg).await.unwrap();
                    }

                    S2COp::NextShuffle(index, original, deck, proof_or_pk) => {
                        assert!(player.is_some() && param.is_some());

                        let mut proof_shuffle = None;
                        match proof_or_pk {
                            ProofOrPk::ProofOne(_remaksing) => {
                                todo!()
                            }
                            ProofOrPk::ProofTwo(proof) => {
                                proof_shuffle = Some(proof);
                            }
                            ProofOrPk::JointPk(pk) => {
                                assert!(joint_pk.is_none());
                                joint_pk = Some(pk);
                            }
                        }

                        if let Some(proof) = proof_shuffle {
                            player
                                .as_ref()
                                .unwrap()
                                .verify_shuffle(
                                    param.as_ref().unwrap(),
                                    joint_pk.as_ref().unwrap(),
                                    original.as_ref().unwrap(),
                                    &deck,
                                    &proof,
                                )
                                .unwrap();
                        }

                        // If I am the chosen one, do the shuffle.
                        // Then send the result back.
                        if index == Some(i as u32) {
                            let card_nums = param.as_ref().unwrap().card_nums();
                            let (shuffled, proof) = player
                                .as_ref()
                                .unwrap()
                                .shuffle(
                                    param.as_ref().unwrap(),
                                    &deck,
                                    joint_pk.as_ref().unwrap(),
                                    card_nums.0 * card_nums.1,
                                )
                                .unwrap();

                            let msg = C2SOp::ShuffledCards(deck, shuffled, proof);
                            let raw_msg = serde_json::to_vec(&msg).unwrap();
                            srv_tx.send(raw_msg).await.unwrap();
                        } else if index.is_none() {
                            // save the final deck
                            final_deck = deck;
                            // Everyone have already shuffle cards, and received shuffledCards
                            // Player can request cards
                            let msg = serde_json::to_vec(&C2SOp::RequestCards(
                                i as u32,
                                None,
                                u32::one(),
                            ))
                            .unwrap();
                            srv_tx.send(msg).await.unwrap();
                        }
                    }

                    S2COp::RevealingCard(_) => {
                        todo!()
                    }

                    S2COp::ReceiveCard(cards) => {
                        todo!()
                    }
                    S2COp::OpenedCard(_) => {
                        todo!()
                    }
                }
            }
        });
    }

    handle.await?;

    println!("Game finished!");
    Ok(())
}

#[derive(Serialize, Deserialize)]
enum C2SOp {
    CreateInstance(ZkGameConfig),
    CheckIn(u32, Surrogate),
    CheckOut(u32, Surrogate),
    ShuffledCards(Vec<MaskedCard>, Vec<MaskedCard>, ProofShuffle),
    RequestCards(u32, Option<u32>, u32),
    PeekCard(Vec<u32>),
    RevealedCard(RevealToken),
    OpenCard(RevealToken),
}

#[derive(Serialize, Deserialize)]
enum ProofOrPk {
    #[allow(unused)]
    ProofOne(ProofRemasking),
    ProofTwo(ProofShuffle),
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    JointPk(AggregatePublicKey),
}

#[derive(Serialize, Deserialize)]
enum S2COp {
    GameParam(Vec<u8>),
    NextShuffle(
        Option<u32>,
        Option<Vec<MaskedCard>>,
        Vec<MaskedCard>,
        ProofOrPk,
    ),
    ReceiveCard(Vec<u32>),
    RevealingCard(RevealToken),
    OpenedCard(RevealToken),
}
