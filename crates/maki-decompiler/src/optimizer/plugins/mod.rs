//! Built-in optimizer plugins
//!
//! This module contains concrete optimizer implementations that improve
//! FSH output quality by removing redundancy and combining related rules.

pub mod cardinality;
pub mod combine_assignments;
pub mod simplify_cardinality;
pub mod remove_duplicates;
pub mod add_reference_keyword;
pub mod combine_card_and_flag;
pub mod remove_zero_zero_card;
pub mod combine_contains;
pub mod remove_generated_text;
pub mod remove_extension_url;
pub mod simplify_array_index;
pub mod remove_choice_slicing;

pub use cardinality::*;
pub use combine_assignments::*;
pub use simplify_cardinality::*;
pub use remove_duplicates::*;
pub use add_reference_keyword::*;
pub use combine_card_and_flag::*;
pub use remove_zero_zero_card::*;
pub use combine_contains::*;
pub use remove_generated_text::*;
pub use remove_extension_url::*;
pub use simplify_array_index::*;
pub use remove_choice_slicing::*;
