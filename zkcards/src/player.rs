use crate::{
    error::GameErrors, user_card::ClassicPlayingCard, Card, CardParameters, CardProtocol,
    MaskedCard, ProofKeyOwnership, ProofReveal, PublicKey, RevealToken, SecretKey,
};
use barnett::BarnettSmartProtocol;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Player {
    name: Vec<u8>,
    sk: SecretKey,
    pk: PublicKey,
    proof_key: ProofKeyOwnership,
    cards: Vec<MaskedCard>,
    opened_cards: Vec<Option<ClassicPlayingCard>>,
}

pub struct Surrogate {
    pub name: Vec<u8>,
    pub pk: PublicKey,
    pub proof_key: ProofKeyOwnership,
}

impl Surrogate {
    pub fn verify(&self, pp: &CardParameters) -> bool {
        CardProtocol::verify_key_ownership(pp, &self.pk, &self.name, &self.proof_key).is_ok()
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
            proof_key,
            cards: vec![],
            opened_cards: vec![],
        })
    }

    pub fn surrogate(&self, pp: &CardParameters) -> Surrogate {
        let rng = &mut thread_rng();
        let proof_key =
            CardProtocol::prove_key_ownership(rng, pp, &self.pk, &self.sk, &self.name).unwrap();

        Surrogate {
            name: self.name.clone(),
            pk: self.pk,
            proof_key,
        }
    }

    pub fn receive_card(&mut self, card: MaskedCard) {
        self.cards.push(card);
        self.opened_cards.push(None);
    }

    pub fn peek_at_card(
        &mut self,
        parameters: &CardParameters,
        reveal_tokens: &mut Vec<(RevealToken, ProofReveal, PublicKey)>,
        card_mappings: &HashMap<Card, ClassicPlayingCard>,
        card: &MaskedCard,
    ) -> Result<(), anyhow::Error> {
        let i = self.cards.iter().position(|&x| x == *card);

        let i = i.ok_or(GameErrors::CardNotFound)?;

        //TODO add function to create that without the proof
        let rng = &mut thread_rng();
        let own_reveal_token = self.compute_reveal_token(rng, parameters, card)?;
        reveal_tokens.push(own_reveal_token);

        let unmasked_card = CardProtocol::unmask(&parameters, reveal_tokens, card)?;
        let opened_card = card_mappings.get(&unmasked_card);
        let opened_card = opened_card.ok_or(GameErrors::InvalidCard)?;

        self.opened_cards[i] = Some(*opened_card);
        Ok(())
    }

    pub fn compute_reveal_token<R: Rng>(
        &self,
        rng: &mut R,
        pp: &CardParameters,
        card: &MaskedCard,
    ) -> anyhow::Result<(RevealToken, ProofReveal, PublicKey)> {
        let (reveal_token, reveal_proof) =
            CardProtocol::compute_reveal_token(rng, &pp, &self.sk, &self.pk, card)?;

        Ok((reveal_token, reveal_proof, self.pk))
    }
}
