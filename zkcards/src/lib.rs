pub mod error;
pub mod player;
pub mod server;
pub mod user_card;

use barnett::discrete_log_cards;
use proof_essentials::{
    homomorphic_encryption::el_gamal::ElGamal,
    vector_commitment::pedersen::PedersenCommitment,
    zkp::{
        arguments::shuffle,
        proofs::{chaum_pedersen_dl_equality, schnorr_identification},
    },
};

// Choose elliptic curve setting
// And instantiate concrete type for our card protocol
type Curve = starknet_curve::Projective;
type Scalar = starknet_curve::Fr;

type CardProtocol<'a> = discrete_log_cards::DLCards<'a, Curve>;
type Enc = ElGamal<Curve>;
type Comm = PedersenCommitment<Curve>;

type CardParameters = discrete_log_cards::Parameters<Curve>;
type PublicKey = discrete_log_cards::PublicKey<Curve>;
type SecretKey = discrete_log_cards::PlayerSecretKey<Curve>;
type AggregatePublicKey = discrete_log_cards::PublicKey<Curve>;

type Card = discrete_log_cards::Card<Curve>;
type MaskedCard = discrete_log_cards::MaskedCard<Curve>;
type RevealToken = discrete_log_cards::RevealToken<Curve>;

type ProofKeyOwnership = schnorr_identification::proof::Proof<Curve>;
type ProofReveal = chaum_pedersen_dl_equality::proof::Proof<Curve>;
type ProofShuffle = shuffle::proof::Proof<Scalar, Enc, Comm>;
