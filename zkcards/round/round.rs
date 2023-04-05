use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use zkcards::{
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
                    instance
                        .as_mut()
                        .unwrap()
                        .register_players(vec![(index, player)]);

                    if instance.as_ref().unwrap().ready_to_shuffle() {
                        instance
                            .as_ref()
                            .unwrap()
                            .setup()
                            .expect("failed to setup a new game");

                        let deck = instance.as_ref().unwrap().initial_deck().unwrap();
                        let first_shuffle_player = {
                            let mut rng = thread_rng();
                            let num = rng.gen();
                            num % PLAYERS_NUM
                        };
                        for player in &player_txs {
                            let msg = serde_json::to_vec(&S2COp::NextShuffle(
                                first_shuffle_player,
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

                C2SOp::ShuffledCards(deck, proof) => {
                    let instance = instance.as_mut().unwrap();
                    let next_shuffle_player = instance.next_shuffle_player().unwrap();
                    instance
                        .register_shuffled_deck(deck, Some(proof), next_shuffle_player)
                        .unwrap();

                    // If the deck has been shuffled by all players
                    // Players can receive cards
                    if instacne.is_all_shuffled() {
                        todo!()
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
            }
        }
    });

    let new_player = |i, pp| async move {
        let name = format!("player-{i}");
        let rng = &mut thread_rng();
        Player::new(rng, pp, &name.as_bytes().to_vec()).unwrap()
    };

    for (i, mut rx) in player_rxs.into_iter().enumerate() {
        let srv_tx = srv_tx.clone();
        tokio::spawn(async move {
            let mut player = None;
            let mut param = None;
            let mut joint_pk = None;
            while let Some(msg) = rx.recv().await {
                let msg = serde_json::from_slice::<S2COp>(msg.as_slice()).unwrap();
                match msg {
                    S2COp::GameParam(pp) => {
                        param =
                            Some(serde_json::from_slice::<CardParameters>(pp.as_slice()).unwrap());
                        player = Some(new_player(i, param.as_ref().unwrap()).await);

                        let msg = C2SOp::CheckIn(i as u32, player.surrogate());
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
                                jioint_pk = Some(pk);
                            }
                        }

                        if let Some(proof) = proof_shuffle {
                            player
                                .as_ref()
                                .unwrap()
                                .verify_shuffle(
                                    pp,
                                    joint_pk.as_ref().unwrap(),
                                    original.as_ref().unwrap(),
                                    &card,
                                    &proof,
                                )
                                .unwrap();
                        }

                        // If I am the chosen one, do the shuffle.
                        // Then send the result back.
                        if index == i {
                            let card_nums = param.as_ref().unwrap().card_nums();
                            let (deck, proof) = player
                                .as_ref()
                                .unwrap()
                                .shuffle(
                                    pp,
                                    &deck,
                                    joint_pk.as_ref().unwrap(),
                                    card_nums.0 * card_nums.1,
                                )
                                .unwrap();

                            let msg = C2SOp::ShuffledCards(deck, proof);
                            let raw_msg = serde_json::to_vec(&msg).unwrap();
                            srv_tx.send(raw_msg).await.unwrap();
                        }
                    }

                    S2COp::RevealingCard(_) => {
                        todo!()
                    }

                    S2COp::ReceiveCard(_) => {
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
    ShuffledCards(Vec<MaskedCard>, ProofShuffle),
    PeekCard(Vec<u32>),
    RevealedCard(RevealToken),
    OpenCard(RevealToken),
}

enum ProofOrPk {
    #[allow(unused)]
    ProofOne(ProofRemasking),
    ProofTwo(ProofShuffle),
    JointPk(AggregatePublicKey),
}

#[derive(Serialize, Deserialize)]
enum S2COp {
    GameParam(Vec<u8>),
    NextShuffle(u32, Option<Vec<MaskedCard>>, Vec<MaskedCard>, ProofOrPk),
    ReceiveCard(Vec<u32>),
    RevealingCard(RevealToken),
    OpenedCard(RevealToken),
}
