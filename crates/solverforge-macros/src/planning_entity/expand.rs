use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Error, Fields};

use crate::attr_parse::{
    attribute_argument_names, get_attribute, has_attribute, has_attribute_argument,
    parse_attribute_bool, parse_attribute_string,
};

use super::list_variable::{generate_list_metadata, generate_list_trait_impl};
use super::scalar_variable::generate_scalar_helpers;
use super::utils::{field_is_option_usize, field_option_inner_type};

include!("expand/derive.rs");
include!("expand/validation.rs");
