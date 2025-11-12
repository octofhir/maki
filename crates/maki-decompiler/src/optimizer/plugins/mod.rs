//! Built-in optimizer plugins
//!
//! This module contains concrete optimizer implementations that improve
//! FSH output quality by removing redundancy and combining related rules.

pub mod add_reference_keyword;
pub mod cardinality;
pub mod combine_assignments;
pub mod combine_card_and_flag;
pub mod combine_contains;
pub mod remove_choice_slicing;
pub mod remove_duplicates;
pub mod remove_extension_url;
pub mod remove_generated_text;
pub mod remove_zero_zero_card;
pub mod simplify_array_index;
pub mod simplify_cardinality;

pub use add_reference_keyword::*;
pub use cardinality::*;
pub use combine_assignments::*;
pub use combine_card_and_flag::*;
pub use combine_contains::*;
pub use remove_choice_slicing::*;
pub use remove_duplicates::*;
pub use remove_extension_url::*;
pub use remove_generated_text::*;
pub use remove_zero_zero_card::*;
pub use simplify_array_index::*;
pub use simplify_cardinality::*;
