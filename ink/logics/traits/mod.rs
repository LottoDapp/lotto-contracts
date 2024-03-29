use openbrush::contracts::access_control::RoleType;

pub const LOTTO_MANAGER_ROLE: RoleType = ink::selector_id!("LOTTO_MANAGER");

pub mod config;
pub mod error;
pub mod raffle;
