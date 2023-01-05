use io::StakingMetadata;
use gmeta::{metawasm, Metadata};
use gstd::{prelude::*, ActorId};

#[metawasm]
pub trait Metawasm {
    type State = <StakingMetadata as Metadata>::State;
}
