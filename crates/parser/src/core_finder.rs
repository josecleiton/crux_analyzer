//! Finds Cores: `impl App for X` blocks and their associated event and
//! effect enums.

use std::collections::{BTreeMap, BTreeSet};
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
    /// The `Effect` associated type itself — the root of that closure, and the
    /// enum whose variants name the Core's capabilities.
    pub effect_root: Option<String>,
}

impl CoreInfo {
    pub fn is_event_enum(&self, name: &str) -> bool {
        self.event_enums.contains_key(name)
    }

    pub fn is_effect_enum(&self, name: &str) -> bool {
        self.effect_enums.contains_key(name)
    }

    /// The capability an operation enum travels through: the root effect
    /// variant that wraps it (`Effect::Audio(AudioOperation)` → `Audio`).
    ///
    /// Structure, not inference — the declaration says which variant carries
    /// this operation. `None` when the operation *is* the root enum (crux's
    /// own `Render` arrives that way) or when no variant wraps it.
    pub fn capability_of(&self, operation_enum: &str) -> Option<String> {
        let root = self.effect_root.as_deref()?;
        if root == operation_enum {
            return None;
        }
        let decl = self.effect_enums.get(root)?;
        let position = (0..decl.variants.len()).find(|index| {
            decl.field_types(*index)
                .any(|field| field == operation_enum)
        })?;
        Some(decl.variants[position].clone())
    }
}

/// `excluded` holds the state-machine enums, `dispatched` the enums whose
/// variants appear in match patterns somewhere, and `delegating` the
/// `(enum, variant)` pairs whose arm hands its payload straight on.
///
/// An enum carried as an event payload (`Event::Sync(State)`,
/// `Event::SignInRequested(Provider)`) must not join the event closure — its
/// wrapping variant would be mistaken for a delegating wrapper, and the arm
/// would silently lose its own label along with every transition it performs.
/// Dispatch alone does not tell the two apart: a `From` impl over a two-variant
/// payload is a match like any other. Delegation does.
pub(crate) fn find_cores(
    index: &CrateIndex,
    excluded: &BTreeSet<String>,
    dispatched: &BTreeSet<String>,
    delegating: &BTreeSet<(String, String)>,
) -> Vec<CoreInfo> {
    index
        .trait_impls
        .iter()
        .filter(|imp| imp.trait_name == "App")
        .map(|imp| {
            let effect_root = associated_type(imp.item, "Effect");
            CoreInfo {
                name: imp.self_ty.clone(),
                event_enums: enum_closure(
                    index,
                    associated_type(imp.item, "Event"),
                    imp.file,
                    excluded,
                    Some(dispatched),
                    Some(delegating),
                ),
                effect_enums: enum_closure(
                    index,
                    effect_root.clone(),
                    imp.file,
                    excluded,
                    // Effect operations are constructed, never dispatched on.
                    None,
                    None,
                ),
                effect_root,
            }
        })
        .collect()
}

/// Resolves `type <name> = X;` inside the impl block to the ident `X`.
pub(crate) fn associated_type(item: &syn::ItemImpl, name: &str) -> Option<String> {
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
    excluded: &BTreeSet<String>,
    dispatched: Option<&BTreeSet<String>>,
    delegating: Option<&BTreeSet<(String, String)>>,
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
        for (position, fields) in decl.variant_fields.iter().enumerate() {
            let variant = decl.variants.get(position).cloned().unwrap_or_default();
            let delegates =
                delegating.is_none_or(|set| set.contains(&(name.clone(), variant.clone())));
            for field in fields {
                let qualifies = delegates
                    && !found.contains_key(&field.type_name)
                    && !excluded.contains(&field.type_name)
                    && dispatched.is_none_or(|set| set.contains(&field.type_name))
                    && !index.enum_decls(&field.type_name).is_empty();
                if qualifies {
                    queue.push((field.type_name.clone(), decl.file.clone()));
                }
            }
        }
        found.insert(name, decl);
    }

    found
}
