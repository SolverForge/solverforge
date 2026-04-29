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

include!("planning_model/manifest.rs");
include!("planning_model/modules.rs");
include!("planning_model/metadata.rs");
include!("planning_model/support_groups.rs");
include!("planning_model/support.rs");
include!("planning_model/shadows.rs");
include!("planning_model/tests.rs");
