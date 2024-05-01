use openbrush::contracts::access_control::RoleType;

pub const LOTTO_MANAGER_ROLE: RoleType = ink::selector_id!("LOTTO_MANAGER");

pub type RaffleId = u32;
pub type Number = u16;

pub mod config;
pub mod error;
pub mod raffle;
pub mod reward;
