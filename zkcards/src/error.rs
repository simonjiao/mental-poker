use barnett::error::CardProtocolError;
use proof_essentials::error::CryptoError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum GameErrors {
    #[error("Invalid config parameters")]
    InvalidParameters,

    #[error("No such card in hand")]
    CardNotFound,

    #[error("Invalid card")]
    InvalidCard,

    #[error("Game not Ready")]
    NotReady,

    #[error("All players have been shuffled")]
    AllShuffled,

    #[error("Protocol Error")]
    ProtocolError(CardProtocolError),

    #[error("Crypto Error")]
    CryptoError(CryptoError),

    #[error("No more cards")]
    NoMoreCards,

    #[error("No enough revealed tokens")]
    NotEnoughRevealedTokens(u32),
}

impl From<CardProtocolError> for GameErrors {
    fn from(value: CardProtocolError) -> Self {
        Self::ProtocolError(value)
    }
}
