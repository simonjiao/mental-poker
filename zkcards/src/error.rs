use barnett::error::CardProtocolError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum GameErrors {
    #[error("No such card in hand")]
    CardNotFound,

    #[error("Invalid card")]
    InvalidCard,

    #[error("Game not Ready")]
    NotReady,

    #[error("Protocol Error")]
    ProtocolError(CardProtocolError),
}

impl From<CardProtocolError> for GameErrors {
    fn from(value: CardProtocolError) -> Self {
        Self::ProtocolError(value)
    }
}
