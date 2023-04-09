pub mod error;
pub mod player;
pub mod server;
pub mod user_card;

use barnett::discrete_log_cards::{self, ark_de, ark_se};
use proof_essentials::{
    homomorphic_encryption::el_gamal::ElGamal,
    vector_commitment::pedersen::PedersenCommitment,
    zkp::{
        arguments::shuffle,
        proofs::{chaum_pedersen_dl_equality, schnorr_identification},
    },
};
use serde::{Deserialize, Serialize};

// Choose elliptic curve setting
// And instantiate concrete type for our card protocol
type Curve = starknet_curve::Projective;
type Scalar = starknet_curve::Fr;

type CardProtocol<'a> = discrete_log_cards::DLCards<'a, Curve>;
type Enc = ElGamal<Curve>;
type Comm = PedersenCommitment<Curve>;

pub type CardParameters = discrete_log_cards::Parameters<Curve>;
pub type PlayerPublicKey = discrete_log_cards::PublicKey<Curve>;
pub type PlayerSecretKey = discrete_log_cards::PlayerSecretKey<Curve>;
pub type AggregatePublicKey = discrete_log_cards::PublicKey<Curve>;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct Card(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    discrete_log_cards::Card<Curve>,
);
impl From<discrete_log_cards::Card<Curve>> for Card {
    fn from(value: discrete_log_cards::Card<Curve>) -> Self {
        Self(value)
    }
}
impl From<Card> for discrete_log_cards::Card<Curve> {
    fn from(value: Card) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct MaskedCard(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    discrete_log_cards::MaskedCard<Curve>,
);
impl From<discrete_log_cards::MaskedCard<Curve>> for MaskedCard {
    fn from(value: discrete_log_cards::MaskedCard<Curve>) -> Self {
        Self(value)
    }
}
impl From<MaskedCard> for discrete_log_cards::MaskedCard<Curve> {
    fn from(value: MaskedCard) -> Self {
        value.0
    }
}
impl<'a> From<&'a MaskedCard> for &'a discrete_log_cards::MaskedCard<Curve> {
    fn from(value: &'a MaskedCard) -> Self {
        &value.0
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct RevealToken(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    discrete_log_cards::RevealToken<Curve>,
);
impl From<discrete_log_cards::RevealToken<Curve>> for RevealToken {
    fn from(value: discrete_log_cards::RevealToken<Curve>) -> Self {
        Self(value)
    }
}
impl From<RevealToken> for discrete_log_cards::RevealToken<Curve> {
    fn from(value: RevealToken) -> Self {
        value.0
    }
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct ProofKeyOwnership(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    schnorr_identification::proof::Proof<Curve>,
);
impl From<schnorr_identification::proof::Proof<Curve>> for ProofKeyOwnership {
    fn from(value: schnorr_identification::proof::Proof<Curve>) -> Self {
        Self(value)
    }
}
impl From<ProofKeyOwnership> for schnorr_identification::proof::Proof<Curve> {
    fn from(value: ProofKeyOwnership) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Deserialize, Serialize)]
pub struct ProofReveal(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    chaum_pedersen_dl_equality::proof::Proof<Curve>,
);
impl From<chaum_pedersen_dl_equality::proof::Proof<Curve>> for ProofReveal {
    fn from(value: chaum_pedersen_dl_equality::proof::Proof<Curve>) -> Self {
        Self(value)
    }
}
impl From<ProofReveal> for chaum_pedersen_dl_equality::proof::Proof<Curve> {
    fn from(value: ProofReveal) -> Self {
        value.0
    }
}

#[derive(Deserialize, Serialize)]
pub struct ProofShuffle(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    shuffle::proof::Proof<Scalar, Enc, Comm>,
);
impl From<shuffle::proof::Proof<Scalar, Enc, Comm>> for ProofShuffle {
    fn from(value: shuffle::proof::Proof<Scalar, Enc, Comm>) -> Self {
        Self(value)
    }
}
impl From<ProofShuffle> for shuffle::proof::Proof<Scalar, Enc, Comm> {
    fn from(value: ProofShuffle) -> Self {
        value.0
    }
}
impl<'a> From<&'a ProofShuffle> for &'a shuffle::proof::Proof<Scalar, Enc, Comm> {
    fn from(value: &'a ProofShuffle) -> Self {
        &value.0
    }
}

//pub struct ProofMasking(chaum_pedersen_dl_equality::proof::Proof<Curve>);
#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug, Deserialize, Serialize)]
pub struct ProofRemasking(
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    chaum_pedersen_dl_equality::proof::Proof<Curve>,
);
impl From<chaum_pedersen_dl_equality::proof::Proof<Curve>> for ProofRemasking {
    fn from(value: chaum_pedersen_dl_equality::proof::Proof<Curve>) -> Self {
        Self(value)
    }
}
impl From<ProofRemasking> for chaum_pedersen_dl_equality::proof::Proof<Curve> {
    fn from(value: ProofRemasking) -> Self {
        value.0
    }
}
