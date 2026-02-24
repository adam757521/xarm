#![cfg_attr(not(test), no_std)]
#![feature(adt_const_params)]

pub mod common;
// TODO: Maybe move this into a prelude. 
pub use common::*;

pub mod mmu;

pub mod pmm;
pub mod vmm;
