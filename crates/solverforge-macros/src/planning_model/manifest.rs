struct PlanningModelInput {
    root: LitStr,
    items: Vec<ManifestItem>,
}

enum ManifestItem {
    Mod(ItemMod),
    Use(ItemUse),
}

impl Parse for PlanningModelInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let root_ident: Ident = input.parse()?;
        if root_ident != "root" {
            return Err(Error::new_spanned(root_ident, "expected `root`"));
        }
        input.parse::<Token![=]>()?;
        let root = input.parse::<LitStr>()?;
        input.parse::<Token![;]>()?;

        let mut items = Vec::new();
        while !input.is_empty() {
            let item = input.parse::<Item>()?;
            match item {
                Item::Mod(item_mod) => {
                    if item_mod.content.is_some() {
                        return Err(Error::new_spanned(
                            item_mod,
                            "planning_model! only accepts file-backed `mod name;` declarations",
                        ));
                    }
                    items.push(ManifestItem::Mod(item_mod));
                }
                Item::Use(item_use) => {
                    if !matches!(item_use.vis, Visibility::Public(_)) {
                        return Err(Error::new_spanned(
                            item_use,
                            "planning_model! only accepts public use exports",
                        ));
                    }
                    items.push(ManifestItem::Use(item_use));
                }
                other => {
                    return Err(Error::new_spanned(
                        other,
                        "planning_model! accepts only `mod name;` and `pub use ...;` items",
                    ));
                }
            }
        }

        Ok(Self { root, items })
    }
}

impl ToTokens for ManifestItem {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Mod(item) => item.to_tokens(tokens),
            Self::Use(item) => item.to_tokens(tokens),
        }
    }
}

struct ModuleSource {
    ident: Ident,
    path: PathBuf,
    file: syn::File,
}

#[derive(Clone)]
struct HookPaths {
    candidate_values: Option<syn::Path>,
    nearby_value_candidates: Option<syn::Path>,
    nearby_entity_candidates: Option<syn::Path>,
    nearby_value_distance_meter: Option<syn::Path>,
    nearby_entity_distance_meter: Option<syn::Path>,
    construction_entity_order_key: Option<syn::Path>,
    construction_value_order_key: Option<syn::Path>,
}

#[derive(Clone)]
struct ScalarVariableMetadata {
    field_name: String,
    hooks: HookPaths,
}

#[derive(Clone)]
struct EntityMetadata {
    type_name: String,
    scalar_variables: Vec<ScalarVariableMetadata>,
    list_variable_name: Option<String>,
    list_element_collection: Option<String>,
}

struct SolutionCollection {
    field_ident: Ident,
    field_name: String,
    type_name: String,
    descriptor_index: Option<usize>,
}

struct SolutionMetadata {
    module_ident: Ident,
    ident: Ident,
    collections: Vec<SolutionCollection>,
    collection_field_names: BTreeSet<String>,
    shadow_config: ShadowConfig,
}

struct ModelMetadata {
    solution: SolutionMetadata,
    entities: BTreeMap<String, EntityMetadata>,
    aliases: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
struct ShadowConfig {
    list_owner: Option<String>,
    inverse_field: Option<String>,
    previous_field: Option<String>,
    next_field: Option<String>,
    cascading_listener: Option<String>,
    post_update_listener: Option<String>,
    entity_aggregates: Vec<String>,
    entity_computes: Vec<String>,
}

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let manifest: PlanningModelInput = syn::parse2(input)?;
    let root = resolve_root(&manifest.root)?;
    let modules = read_modules(&manifest.items, &root)?;
    let model = collect_model_metadata(&manifest.items, &modules)?;
    let support_impl = generate_support_impl(&model)?;
    let module_dependency_paths = modules.iter().map(|module| {
        LitStr::new(
            &module.path.to_string_lossy(),
            proc_macro2::Span::call_site(),
        )
    });

    let items = manifest.items.iter();
    Ok(quote! {
        #(#items)*
        const _: &[&str] = &[
            #(include_str!(#module_dependency_paths)),*
        ];
        #support_impl
    })
}
