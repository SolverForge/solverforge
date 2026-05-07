fn resolve_root(root: &LitStr) -> Result<PathBuf> {
    let value = root.value();
    let root_path = PathBuf::from(&value);
    if root_path.is_absolute() {
        return Ok(root_path);
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        Error::new_spanned(root, "planning_model! could not resolve CARGO_MANIFEST_DIR")
    })?;
    let manifest_root = PathBuf::from(&manifest_dir).join(&root_path);
    if manifest_root.exists() {
        return Ok(manifest_root);
    }
    for ancestor in PathBuf::from(&manifest_dir).ancestors() {
        let candidate = ancestor.join(&root_path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let current_root = current_dir.join(&root_path);
        if current_root.exists() {
            return Ok(current_root);
        }
        for ancestor in current_dir.ancestors() {
            let candidate = ancestor.join(&root_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Ok(PathBuf::from(manifest_dir).join(root_path))
}

fn read_modules(items: &[ManifestItem], root: &Path) -> Result<Vec<ModuleSource>> {
    let mut modules = Vec::new();
    for item in items {
        let ManifestItem::Mod(item_mod) = item else {
            continue;
        };
        let ident = item_mod.ident.clone();
        let path = module_path(root, &ident).ok_or_else(|| {
            Error::new_spanned(
                item_mod,
                format!(
                    "planning_model! module `{ident}` must resolve to `{}/{ident}.rs` or `{}/{ident}/mod.rs`",
                    root.display(),
                    root.display(),
                ),
            )
        })?;
        let source = std::fs::read_to_string(&path).map_err(|err| {
            Error::new_spanned(
                item_mod,
                format!(
                    "planning_model! could not read module `{ident}` at `{}`: {err}",
                    path.display(),
                ),
            )
        })?;
        let file = syn::parse_file(&source).map_err(|err| {
            Error::new_spanned(
                item_mod,
                format!(
                    "planning_model! could not parse module `{ident}` at `{}`: {err}",
                    path.display(),
                ),
            )
        })?;
        modules.push(ModuleSource { ident, path, file });
    }
    Ok(modules)
}

fn module_path(root: &Path, ident: &Ident) -> Option<PathBuf> {
    let file_path = root.join(format!("{ident}.rs"));
    if file_path.exists() {
        return Some(file_path);
    }
    let mod_path = root.join(ident.to_string()).join("mod.rs");
    mod_path.exists().then_some(mod_path)
}

fn collect_model_metadata(
    items: &[ManifestItem],
    modules: &[ModuleSource],
) -> Result<ModelMetadata> {
    let mut solution: Option<SolutionMetadata> = None;
    let mut entities = BTreeMap::new();
    let mut facts = BTreeSet::new();
    let mut raw_aliases = BTreeMap::new();

    for item in items {
        if let ManifestItem::Use(item_use) = item {
            collect_use_aliases(item_use, &mut raw_aliases)?;
        }
    }

    for module in modules {
        for item in &module.file.items {
            match item {
                Item::Struct(item_struct) => {
                    if has_attribute(&item_struct.attrs, "planning_solution") {
                        if let Some(existing) = &solution {
                            return Err(Error::new_spanned(
                                item_struct,
                                format!(
                                    "planning_model! found duplicate #[planning_solution]; `{}` is already the model solution",
                                    existing.ident
                                ),
                            ));
                        }
                        solution = Some(parse_solution(module, item_struct)?);
                    }
                    if has_attribute(&item_struct.attrs, "planning_entity") {
                        let metadata = parse_entity(module, item_struct)?;
                        entities.insert(metadata.type_name.clone(), metadata);
                    }
                    if has_attribute(&item_struct.attrs, "problem_fact") {
                        if let Some(attr) = get_attribute(&item_struct.attrs, "problem_fact") {
                            validate_problem_fact_attribute(attr)?;
                        }
                        validate_problem_fact_fields(item_struct)?;
                        facts.insert(item_struct.ident.to_string());
                    }
                }
                Item::Type(item_type) => {
                    if let Some(target) = alias_target_name(item_type) {
                        insert_alias(
                            &mut raw_aliases,
                            item_type.ident.to_string(),
                            target,
                            item_type,
                        )?;
                    }
                }
                Item::Use(item_use) if matches!(item_use.vis, Visibility::Public(_)) => {
                    collect_use_aliases(item_use, &mut raw_aliases)?;
                }
                _ => {}
            }
        }
    }

    let Some(solution) = solution else {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "planning_model! requires exactly one #[planning_solution] in the listed modules",
        ));
    };

    let aliases = resolve_aliases(&raw_aliases)?;
    validate_collections(&solution, &entities, &facts, &aliases)?;
    validate_list_element_sources(&solution, &entities, &aliases)?;

    Ok(ModelMetadata {
        solution,
        entities,
        aliases,
    })
}

fn alias_target_name(item_type: &ItemType) -> Option<String> {
    type_name(&item_type.ty)
}

fn collect_use_aliases(item_use: &ItemUse, aliases: &mut BTreeMap<String, String>) -> Result<()> {
    fn walk(
        tree: &UseTree,
        aliases: &mut BTreeMap<String, String>,
        span: &impl ToTokens,
    ) -> Result<()> {
        match tree {
            UseTree::Path(path) => walk(&path.tree, aliases, span),
            UseTree::Rename(rename) => insert_alias(
                aliases,
                rename.rename.to_string(),
                rename.ident.to_string(),
                span,
            ),
            UseTree::Group(group) => {
                for item in &group.items {
                    walk(item, aliases, span)?;
                }
                Ok(())
            }
            UseTree::Name(_) | UseTree::Glob(_) => Ok(()),
        }
    }

    walk(&item_use.tree, aliases, item_use)
}

fn insert_alias(
    aliases: &mut BTreeMap<String, String>,
    alias: String,
    target: String,
    span: &impl ToTokens,
) -> Result<()> {
    if alias == target {
        return Ok(());
    }
    if let Some(existing) = aliases.get(&alias) {
        if existing != &target {
            return Err(Error::new_spanned(
                span,
                format!(
                    "planning_model! alias `{alias}` points to both `{existing}` and `{target}`",
                ),
            ));
        }
        return Ok(());
    }
    aliases.insert(alias, target);
    Ok(())
}

fn resolve_aliases(raw_aliases: &BTreeMap<String, String>) -> Result<BTreeMap<String, String>> {
    fn resolve_one(
        name: &str,
        raw_aliases: &BTreeMap<String, String>,
        stack: &mut Vec<String>,
    ) -> Result<String> {
        let Some(target) = raw_aliases.get(name) else {
            return Ok(name.to_string());
        };
        if stack.iter().any(|seen| seen == name) {
            stack.push(name.to_string());
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "planning_model! alias cycle detected: {}",
                    stack.join(" -> "),
                ),
            ));
        }
        stack.push(name.to_string());
        let resolved = resolve_one(target, raw_aliases, stack)?;
        stack.pop();
        Ok(resolved)
    }

    let mut resolved = BTreeMap::new();
    for alias in raw_aliases.keys() {
        resolved.insert(
            alias.clone(),
            resolve_one(alias, raw_aliases, &mut Vec::new())?,
        );
    }
    Ok(resolved)
}

fn canonical_type_name<'a>(aliases: &'a BTreeMap<String, String>, type_name: &'a str) -> &'a str {
    aliases
        .get(type_name)
        .map(String::as_str)
        .unwrap_or(type_name)
}
