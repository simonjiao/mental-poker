use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::task::yield_now;
use zkcards::{
    ark_de, ark_se,
    error::GameErrors,
    player::{Player, Surrogate},
    server::{ZkCardGame, ZkGameConfig},
    AggregatePublicKey, Card, CardParameters, MaskedCard, ProofRemasking, ProofReveal,
    ProofShuffle, RevealToken, RevealedToken,
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
                        let initial_cards = instance
                            .as_ref()
                            .unwrap()
                            .card_mappings()
                            .unwrap()
                            .into_iter()
                            .map(|c| InitialOrMaskedCard::InitialCard(c.0, c.1))
                            .collect::<Vec<_>>();

                        let first_shuffle_player = {
                            let mut rng = thread_rng();
                            let num: u32 = rng.gen();
                            num % PLAYERS_NUM
                        };
                        for player in &player_txs {
                            let msg = serde_json::to_vec(&S2COp::NextShuffle(
                                Some(first_shuffle_player),
                                initial_cards.clone(),
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
                            .register_shuffled_deck(deck, None, Some(first_shuffle_player))
                            .unwrap();
                    }
                }

                C2SOp::ShuffledCards(original, deck, proof) => {
                    // FixMe: avoid clone operation
                    let raw_proof = serde_json::to_vec(&proof).unwrap();
                    // TODO: verify proof shuffle

                    let original = original
                        .into_iter()
                        .map(|o| InitialOrMaskedCard::MaskedCard(o))
                        .collect::<Vec<_>>();
                    let next_shuffle_player = instance.as_mut().unwrap().next_shuffle_player().ok();
                    instance
                        .as_mut()
                        .unwrap()
                        .register_shuffled_deck(
                            deck.clone(),
                            // FixMe: remove this deserialization
                            Some(serde_json::from_slice(raw_proof.as_slice()).unwrap()),
                            next_shuffle_player,
                        )
                        .unwrap();
                    let msg = S2COp::NextShuffle(
                        next_shuffle_player,
                        original,
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

                C2SOp::PeekCard(player_idx, card_idx) => {
                    let revealed_tokens = loop {
                        match instance
                            .as_ref()
                            .unwrap()
                            .revealed_tokens(player_idx, card_idx)
                        {
                            Err(GameErrors::NotEnoughRevealedTokens(c)) => {
                                println!("revealed player count {c}");
                                yield_now().await;
                            }
                            Err(_) => {
                                panic!("nothing we can do");
                            }
                            Ok(tokens) => break tokens,
                        }
                    };

                    let msg = serde_json::to_vec(&S2COp::RevealedCard(card_idx, revealed_tokens))
                        .unwrap();
                    let player = player_txs.get(player_idx as usize).unwrap();
                    player.send(msg).await.unwrap();
                }

                C2SOp::RevealingCard(index, token, proof, player_idx) => {
                    instance
                        .as_mut()
                        .unwrap()
                        .register_revealed_token(index, token, proof, player_idx)
                        .unwrap();
                }

                C2SOp::OpenCard(index, token, proof, player_idx) => {
                    println!("player {player_idx} open his/her card {index}");
                    instance
                        .as_mut()
                        .unwrap()
                        .register_revealed_token(index, token, proof, player_idx)
                        .unwrap();
                }

                C2SOp::RequestCards(index, _start, _num) => {
                    let _ = player_txs.get(index as usize).unwrap();
                    if instance.as_ref().unwrap().is_all_shuffled() {
                        // next_card() must update the index
                        let card = if let Ok(next_card) = instance.as_mut().unwrap().next_card() {
                            S2COp::ReceiveCard(Some((index, next_card)))
                        } else {
                            // no more cards
                            S2COp::ReceiveCard(None)
                        };

                        for player in &player_txs {
                            let msg = serde_json::to_vec(&card).unwrap();
                            player.send(msg).await.unwrap();
                        }
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
            let mut card_mappings = HashMap::new();
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
                            let original = original
                                .into_iter()
                                .map(|o| match o {
                                    InitialOrMaskedCard::MaskedCard(card) => card,
                                    _ => panic!(),
                                })
                                .collect::<Vec<_>>();

                            player
                                .as_ref()
                                .unwrap()
                                .verify_shuffle(
                                    param.as_ref().unwrap(),
                                    joint_pk.as_ref().unwrap(),
                                    original.as_ref(),
                                    &deck,
                                    &proof,
                                )
                                .unwrap();
                        } else {
                            card_mappings = original
                                .into_iter()
                                .map(|o| match o {
                                    InitialOrMaskedCard::InitialCard(card, classic_card) => {
                                        (card, classic_card)
                                    }
                                    _ => panic!(),
                                })
                                .collect::<HashMap<_, _>>();
                        }

                        // If I am the chosen one, do the shuffle.
                        // Then send the result back.
                        if index == Some(i as u32) {
                            println!("player {i} shuffling the deck");
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
                            println!("player {i} request cards");
                            // Everyone have already shuffle cards, and received shuffledCards
                            // Player can request cards
                            let msg =
                                serde_json::to_vec(&C2SOp::RequestCards(i as u32, None, 1_u32))
                                    .unwrap();
                            srv_tx.send(msg).await.unwrap();
                        }
                    }

                    S2COp::RevealedCard(idx, mut tokens) => {
                        let (card, token, proof, pk) = async {
                            let mut rng = thread_rng();
                            let card = final_deck.get(idx as usize).unwrap();
                            // make sure this card is belonged to current player
                            let (token, proof, pk) = player
                                .as_ref()
                                .unwrap()
                                .compute_reveal_token(&mut rng, param.as_ref().unwrap(), card)
                                .unwrap();
                            (card, token, proof, pk)
                        }
                        .await;

                        tokens.push(RevealedToken {
                            token,
                            proof,
                            player: pk,
                        });
                        player
                            .as_mut()
                            .unwrap()
                            .peek_at_card(
                                param.as_ref().unwrap(),
                                &mut tokens,
                                &card_mappings,
                                &card,
                            )
                            .unwrap();
                        println!("player {i} peek card {idx}");

                        // submit private revealed token
                        let msg = serde_json::to_vec(&C2SOp::OpenCard(idx, token, proof, i as u32))
                            .unwrap();
                        srv_tx.send(msg).await.unwrap();
                    }

                    S2COp::ReceiveCard(card) => {
                        if let Some((player_idx, index)) = card {
                            let card = final_deck.get(index as usize).unwrap();

                            // not mine card
                            if player_idx != i as u32 {
                                let (token, proof) = async {
                                    let mut rng = thread_rng();
                                    let (token, proof, _pk) = player
                                        .as_ref()
                                        .unwrap()
                                        .compute_reveal_token(
                                            &mut rng,
                                            param.as_ref().unwrap(),
                                            card,
                                        )
                                        .unwrap();
                                    (token, proof)
                                }
                                .await;
                                let msg = serde_json::to_vec(&C2SOp::RevealingCard(
                                    index, token, proof, i as u32,
                                ))
                                .unwrap();
                                srv_tx.send(msg).await.unwrap();
                            } else {
                                player.as_mut().unwrap().receive_card(*card);

                                let msg = serde_json::to_vec(&C2SOp::PeekCard(player_idx, index))
                                    .unwrap();
                                srv_tx.send(msg).await.unwrap();
                            }
                        } else {
                            // no more cards
                        }
                    }
                    S2COp::OpenedCard(..) => {
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
    PeekCard(u32, u32),
    RevealingCard(u32, RevealToken, ProofReveal, u32),
    OpenCard(u32, RevealToken, ProofReveal, u32),
}

#[derive(Serialize, Deserialize)]
enum ProofOrPk {
    #[allow(unused)]
    ProofOne(ProofRemasking),
    ProofTwo(ProofShuffle),
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    JointPk(AggregatePublicKey),
}

#[derive(Clone, Serialize, Deserialize)]
enum InitialOrMaskedCard {
    InitialCard(Card, Vec<u8>),
    MaskedCard(MaskedCard),
}

#[derive(Serialize, Deserialize)]
enum S2COp {
    GameParam(Vec<u8>),
    NextShuffle(
        Option<u32>,
        Vec<InitialOrMaskedCard>,
        Vec<MaskedCard>,
        ProofOrPk,
    ),
    ReceiveCard(Option<(u32, u32)>),
    RevealedCard(u32, Vec<RevealedToken>),
    OpenedCard(RevealToken),
}
