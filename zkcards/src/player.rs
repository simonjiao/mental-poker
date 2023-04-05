use crate::{
    error::GameErrors, user_card::ClassicPlayingCard, AggregatePublicKey, Card, CardParameters,
    CardProtocol, MaskedCard, PlayerPublicKey, PlayerSecretKey, ProofKeyOwnership, ProofReveal,
    ProofShuffle, RevealToken, Scalar,
};
use barnett::{
    discrete_log_cards::{ark_de, ark_se},
    BarnettSmartProtocol, Mask,
};
use proof_essentials::utils::{permutation::Permutation, rand::sample_vector};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Player {
    name: Vec<u8>,
    sk: PlayerSecretKey,
    pk: PlayerPublicKey,
    proof_key: ProofKeyOwnership,
    cards: Vec<MaskedCard>,
    opened_cards: Vec<Option<ClassicPlayingCard>>,
}

#[derive(Serialize, Deserialize)]
pub struct Surrogate {
    pub name: Vec<u8>,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub pk: PlayerPublicKey,
    pub proof_key: ProofKeyOwnership,
}

impl Surrogate {
    pub fn verify(&self, pp: &CardParameters) -> bool {
        CardProtocol::verify_key_ownership(pp, &self.pk, &self.name, &self.proof_key.into()).is_ok()
    }
}

impl Player {
    pub fn new<R: Rng>(rng: &mut R, pp: &CardParameters, name: &Vec<u8>) -> anyhow::Result<Self> {
        let (pk, sk) = CardProtocol::player_keygen(rng, pp)?;
        let proof_key = CardProtocol::prove_key_ownership(rng, pp, &pk, &sk, name)?;
        Ok(Self {
            name: name.clone(),
            sk,
            pk,
            proof_key: proof_key.into(),
            cards: vec![],
            opened_cards: vec![],
        })
    }

    pub fn new_surrogate(&self, pp: &CardParameters) -> Surrogate {
        let rng = &mut thread_rng();
        let proof_key =
            CardProtocol::prove_key_ownership(rng, pp, &self.pk, &self.sk, &self.name).unwrap();

        Surrogate {
            name: self.name.clone(),
            pk: self.pk,
            proof_key: proof_key.into(),
        }
    }

    pub fn surrogate(&self) -> Surrogate {
        Surrogate {
            name: self.name.clone(),
            pk: self.pk,
            proof_key: self.proof_key,
        }
    }

    pub fn shuffle(
        &self,
        parameters: &CardParameters,
        deck: &Vec<MaskedCard>,
        joint_pk: &AggregatePublicKey,
        nums_of_cards: usize,
    ) -> anyhow::Result<(Vec<MaskedCard>, ProofShuffle), GameErrors> {
        let &mut rng = thread_rng();
        let permutation = Permutation::new(rng, nums_of_cards);
        let masking_factors: Vec<Scalar> = sample_vector(rng, nums_of_cards);

        let (shuffled_deck, shuffle_proof) = CardProtocol::shuffle_and_remask(
            rng,
            parameters,
            joint_pk,
            deck.into(),
            &masking_factors,
            &permutation,
        )?;

        Ok((shuffled_deck.into(), shuffle_proof.into()))
    }

    pub fn verify_shuffle(
        &self,
        parameters: &CardParameters,
        joint_pk: &AggregatePublicKey,
        original_deck: &Vec<MaskedCard>,
        shuffled_deck: &Vec<MaskedCard>,
        proof_shuffle: &ProofShuffle,
    ) -> anyhow::Result<(), GameErrors> {
        CardProtocol::verify_shuffle(
            &parameters,
            joint_pk,
            original_deck.into(),
            shuffled_deck.into(),
            proof_shuffle.into(),
        )
        .into()
    }

    pub fn receive_card(&mut self, card: MaskedCard) {
        self.cards.push(card);
        self.opened_cards.push(None);
    }

    pub fn peek_at_card(
        &mut self,
        parameters: &CardParameters,
        reveal_tokens: &mut Vec<(RevealToken, ProofReveal, PlayerPublicKey)>,
        card_mappings: &HashMap<Card, ClassicPlayingCard>,
        card: &MaskedCard,
    ) -> Result<(), anyhow::Error> {
        let i = self.cards.iter().position(|&x| x == *card);

        let i = i.ok_or(GameErrors::CardNotFound)?;

        //TODO add function to create that without the proof
        let rng = &mut thread_rng();
        let own_reveal_token = self.compute_reveal_token(rng, parameters, card)?;
        reveal_tokens.push(own_reveal_token);

        let raw_reveal_tokens = reveal_tokens
            .iter()
            .map(|t| (t.0.into(), t.1.into(), t.2))
            .collect::<Vec<_>>();

        let unmasked_card = CardProtocol::unmask(&parameters, &raw_reveal_tokens, card.into())?;
        let opened_card = card_mappings.get(&unmasked_card.into());
        let opened_card = opened_card.ok_or(GameErrors::InvalidCard)?;

        self.opened_cards[i] = Some(*opened_card);
        Ok(())
    }

    pub fn compute_reveal_token<R: Rng>(
        &self,
        rng: &mut R,
        pp: &CardParameters,
        card: &MaskedCard,
    ) -> anyhow::Result<(RevealToken, ProofReveal, PlayerPublicKey)> {
        let (reveal_token, reveal_proof) =
            CardProtocol::compute_reveal_token(rng, &pp, &self.sk, &self.pk, card.into())?;

        Ok((reveal_token.into(), reveal_proof.into(), self.pk))
    }
}
