mod ftoken;
pub use ftoken::*;

mod common;
pub use common::*;

pub mod prelude;

pub const FOREIGN_USER: u64 = 12345678;
pub const PROGRAMS: &[u64] = &[1, 2, 3];
pub const USERS: &[u64] = &[1, 2, 3, 4, 5, 6, 7, 8];
