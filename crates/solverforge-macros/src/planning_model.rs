use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Error, Fields, Ident, Item, ItemMod, ItemStruct, ItemType, ItemUse, LitStr, Result,
    Token, Type, UseTree, Visibility,
};

use crate::attr_parse::{
    get_attribute, has_attribute, parse_attribute_bool, parse_attribute_list,
    parse_attribute_string,
};
use crate::attr_validation::{
    validate_list_element_collection_attribute, validate_no_attribute_args,
    validate_planning_entity_attribute, validate_planning_list_variable_attribute,
    validate_planning_solution_attribute, validate_planning_variable_attribute,
    validate_problem_fact_attribute, validate_shadow_updates_attribute,
    validate_shadow_variable_attribute,
};

include!("planning_model/manifest.rs");
include!("planning_model/modules.rs");
include!("planning_model/metadata.rs");
include!("planning_model/support_groups.rs");
include!("planning_model/support.rs");
include!("planning_model/shadows.rs");
include!("planning_model/tests.rs");
