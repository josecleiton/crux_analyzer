//! Finds Cores: `impl App for X` blocks and their associated event and
//! effect enums.

use std::collections::BTreeMap;
use std::path::Path;

use crate::index::{CrateIndex, EnumDecl};

pub(crate) struct CoreInfo {
    /// Name of the type implementing `App` (the Core's name).
    pub name: String,
    /// Every enum the Core's events can be expressed in, keyed by the name
    /// code actually uses (declared ident or import alias): the root `Event`
    /// associated type plus enums reachable through variant fields
    /// (`Event::Recording(RecordingEvent)` → both).
    pub event_enums: BTreeMap<String, EnumDecl>,
    /// Same closure over the `Effect` associated type: the effect enum plus
    /// the operation enums its variants wrap (`Effect::Audio(AudioOperation)`).
    pub effect_enums: BTreeMap<String, EnumDecl>,
}

impl CoreInfo {
    pub fn is_event_enum(&self, name: &str) -> bool {
        self.event_enums.contains_key(name)
    }

    pub fn is_effect_enum(&self, name: &str) -> bool {
        self.effect_enums.contains_key(name)
    }
}

pub(crate) fn find_cores(index: &CrateIndex) -> Vec<CoreInfo> {
    index
        .trait_impls
        .iter()
        .filter(|imp| imp.trait_name == "App")
        .map(|imp| CoreInfo {
            name: imp.self_ty.clone(),
            event_enums: enum_closure(index, associated_type(imp.item, "Event"), imp.file),
            effect_enums: enum_closure(index, associated_type(imp.item, "Effect"), imp.file),
        })
        .collect()
}

/// Resolves `type <name> = X;` inside the impl block to the ident `X`.
fn associated_type(item: &syn::ItemImpl, name: &str) -> Option<String> {
    item.items.iter().find_map(|impl_item| {
        if let syn::ImplItem::Type(assoc) = impl_item {
            if assoc.ident == name {
                if let syn::Type::Path(type_path) = &assoc.ty {
                    return type_path.path.segments.last().map(|s| s.ident.to_string());
                }
            }
        }
        None
    })
}

/// Transitive closure over variant field types, resolving each name against
/// the file that references it (so `Event` binds to the Core's own `Event`,
/// not a same-named enum in another module).
fn enum_closure(
    index: &CrateIndex,
    root: Option<String>,
    core_file: &Path,
) -> BTreeMap<String, EnumDecl> {
    let mut found: BTreeMap<String, EnumDecl> = BTreeMap::new();
    let mut queue: Vec<(String, std::path::PathBuf)> = root
        .map(|name| (name, core_file.to_path_buf()))
        .into_iter()
        .collect();

    while let Some((name, referenced_from)) = queue.pop() {
        if found.contains_key(&name) {
            continue;
        }
        let Some(decl) = index.resolve_enum(&name, &referenced_from) else {
            continue;
        };
        let decl = decl.clone();
        for field_types in &decl.variant_field_types {
            for field_type in field_types {
                if !found.contains_key(field_type) && !index.enum_decls(field_type).is_empty() {
                    queue.push((field_type.clone(), decl.file.clone()));
                }
            }
        }
        found.insert(name, decl);
    }

    found
}
