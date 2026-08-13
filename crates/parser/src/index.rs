//! Flat index of the crate's items: enums and functions, across all files.
//!
//! Module trees are not resolved; instead the index keeps every declaration
//! per name (names can collide across modules) plus the `use ... as ...`
//! aliases, so lookups can prefer the declaration in the referencing file
//! and follow the alias a file actually uses in its patterns.
//!
//! `#[cfg(test)]` modules are skipped so test helpers never contribute
//! spurious states or transitions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::annotations::{doc_block, DocBlock};
use crate::loader::SourceFile;

/// A field of an enum variant: its name (for named fields) and the last
/// segment of its type.
#[derive(Debug, Clone)]
pub(crate) struct VariantField {
    pub name: Option<String>,
    pub type_name: String,
}

/// An enum declaration.
#[derive(Debug, Clone)]
pub(crate) struct EnumDecl {
    pub file: PathBuf,
    /// Variant names in declaration order.
    pub variants: Vec<String>,
    /// For each variant, its fields (used to follow nested event enums like
    /// `Event::Recording(RecordingEvent)`, event payload bindings, and
    /// composite states like `State::Active(ActiveState)`).
    pub variant_fields: Vec<Vec<VariantField>>,
    /// Documentation authored on the enum item itself.
    pub docs: DocBlock,
    /// Documentation authored on each variant. Parallel to `variants`.
    pub variant_docs: Vec<DocBlock>,
    /// The `#[default]` variant, when the enum derives `Default`.
    pub default_variant: Option<String>,
}

impl EnumDecl {
    /// Last-segment type names of a variant's fields.
    pub fn field_types(&self, variant_index: usize) -> impl Iterator<Item = &str> {
        self.variant_fields[variant_index]
            .iter()
            .map(|f| f.type_name.as_str())
    }

    /// Documentation of a variant by position — companion to [`field_types`].
    pub fn docs_of(&self, variant_index: usize) -> &DocBlock {
        &self.variant_docs[variant_index]
    }
}

/// A named field of a struct, carrying two readings of its type because two
/// analyses need different ones.
#[derive(Debug, Clone)]
pub(crate) struct StructField {
    pub name: String,
    /// The last path segment as written: `Vec` for `Vec<Draft>`, `Option` for
    /// `Option<E>`. What a `T::default()` reset actually assigns, and so the
    /// only sound reading there — `default()` on an `Option<E>` field yields
    /// `None`, not a variant of `E`, and unwrapping would invent a transition.
    pub declared: String,
    /// The type looked through collections and smart pointers: `Draft` for
    /// `Vec<Draft>`. What model reachability follows, so that state held
    /// per-entity inside a collection the model owns is still reachable.
    pub reachable: String,
}

/// A struct declaration: its named fields, in declaration order.
#[derive(Debug, Clone)]
pub(crate) struct StructDecl {
    pub fields: Vec<StructField>,
}

impl EnumDecl {
    pub fn has_variant(&self, name: &str) -> bool {
        self.variants.iter().any(|v| v == name)
    }
}

/// A function or method body, addressable by `(self type, name)`.
pub(crate) struct FnInfo<'a> {
    /// `Some("App")` for `impl App { fn f }`, `None` for free functions.
    pub self_ty: Option<String>,
    pub name: String,
    /// Parameter binding names, in order (`self` receivers excluded;
    /// non-ident patterns become `"_"`).
    pub params: Vec<String>,
    pub block: &'a syn::Block,
    pub file: &'a Path,
}

/// A trait impl block (the core finder looks for `impl App for X`).
pub(crate) struct TraitImplInfo<'a> {
    pub trait_name: String,
    pub self_ty: String,
    pub item: &'a syn::ItemImpl,
    pub file: &'a Path,
}

/// `use path::Original as Alias;`
struct UseRename {
    /// Path segments before the renamed ident (e.g. `["crate", "recording"]`).
    prefix: Vec<String>,
    original: String,
    alias: String,
}

pub(crate) struct CrateIndex<'a> {
    /// Every declaration known by a given name — declared idents plus aliases.
    pub enums: HashMap<String, Vec<EnumDecl>>,
    /// Struct declarations by name (used to resolve `T::default()` resets).
    pub structs: HashMap<String, StructDecl>,
    pub fns: Vec<FnInfo<'a>>,
    pub trait_impls: Vec<TraitImplInfo<'a>>,
}

impl<'a> CrateIndex<'a> {
    /// Looks up a function body by self type and name.
    pub fn find_fn(&self, self_ty: Option<&str>, name: &str) -> Option<&FnInfo<'a>> {
        self.fns
            .iter()
            .find(|f| f.self_ty.as_deref() == self_ty && f.name == name)
    }

    /// All declarations known by `name` (declared ident or alias).
    pub fn enum_decls(&self, name: &str) -> &[EnumDecl] {
        self.enums.get(name).map_or(&[], Vec::as_slice)
    }

    /// The declaration for `name`, preferring one in `file` (same-file
    /// declarations shadow same-named enums from other modules).
    pub fn resolve_enum(&self, name: &str, file: &Path) -> Option<&EnumDecl> {
        let decls = self.enum_decls(name);
        decls
            .iter()
            .find(|d| d.file == file)
            .or_else(|| decls.first())
    }
}

pub(crate) fn build_index<'a>(sources: &'a [SourceFile]) -> CrateIndex<'a> {
    let mut index = CrateIndex {
        enums: HashMap::new(),
        structs: HashMap::new(),
        fns: Vec::new(),
        trait_impls: Vec::new(),
    };
    let mut renames: Vec<UseRename> = Vec::new();

    for source in sources {
        index_items(&source.ast.items, &source.path, &mut index, &mut renames);
    }
    register_aliases(&mut index, &renames);
    index
}

fn index_items<'a>(
    items: &'a [syn::Item],
    file: &'a Path,
    index: &mut CrateIndex<'a>,
    renames: &mut Vec<UseRename>,
) {
    for item in items {
        match item {
            syn::Item::Enum(item_enum) => {
                index
                    .enums
                    .entry(item_enum.ident.to_string())
                    .or_default()
                    .push(EnumDecl {
                        file: file.to_path_buf(),
                        variants: item_enum
                            .variants
                            .iter()
                            .map(|v| v.ident.to_string())
                            .collect(),
                        variant_fields: item_enum
                            .variants
                            .iter()
                            .map(|v| variant_fields(&v.fields))
                            .collect(),
                        docs: doc_block(&item_enum.attrs),
                        variant_docs: item_enum
                            .variants
                            .iter()
                            .map(|v| doc_block(&v.attrs))
                            .collect(),
                        default_variant: item_enum
                            .variants
                            .iter()
                            .find(|v| v.attrs.iter().any(|a| a.path().is_ident("default")))
                            .map(|v| v.ident.to_string()),
                    });
            }
            syn::Item::Struct(item_struct) => {
                index.structs.insert(
                    item_struct.ident.to_string(),
                    StructDecl {
                        fields: item_struct
                            .fields
                            .iter()
                            .filter_map(|field| {
                                let name = field.ident.as_ref()?.to_string();
                                let syn::Type::Path(path) = &field.ty else {
                                    return None;
                                };
                                let declared = path.path.segments.last()?.ident.to_string();
                                Some(StructField {
                                    name,
                                    reachable: reachable_type_name(&field.ty, 0)
                                        .unwrap_or_else(|| declared.clone()),
                                    declared,
                                })
                            })
                            .collect(),
                    },
                );
            }
            syn::Item::Use(item_use) => {
                collect_renames(&item_use.tree, &mut Vec::new(), renames);
            }
            syn::Item::Fn(item_fn) => {
                index.fns.push(FnInfo {
                    self_ty: None,
                    name: item_fn.sig.ident.to_string(),
                    params: param_names(&item_fn.sig),
                    block: &item_fn.block,
                    file,
                });
            }
            syn::Item::Impl(item_impl) => {
                let Some(self_ty) = type_name(&item_impl.self_ty) else {
                    continue;
                };
                if let Some((trait_path, _)) = &item_impl.trait_ {
                    if let Some(segment) = trait_path.segments.last() {
                        index.trait_impls.push(TraitImplInfo {
                            trait_name: segment.ident.to_string(),
                            self_ty: self_ty.clone(),
                            item: item_impl,
                            file,
                        });
                    }
                }
                for impl_item in &item_impl.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        index.fns.push(FnInfo {
                            self_ty: Some(self_ty.clone()),
                            name: method.sig.ident.to_string(),
                            params: param_names(&method.sig),
                            block: &method.block,
                            file,
                        });
                    }
                }
            }
            syn::Item::Mod(item_mod) => {
                if is_cfg_test(&item_mod.attrs) {
                    continue;
                }
                if let Some((_, items)) = &item_mod.content {
                    index_items(items, file, index, renames);
                }
            }
            _ => {}
        }
    }
}

/// Registers `use path::X as Y` aliases as extra entries under `Y`, resolving
/// `X` against the module hinted by the path (`crate::recording::Event as
/// RecordingEvent` prefers the `Event` declared in `recording.rs`).
fn register_aliases(index: &mut CrateIndex, renames: &[UseRename]) {
    for rename in renames {
        let decls = index.enum_decls(&rename.original);
        if decls.is_empty() {
            continue;
        }
        let module_hint = rename.prefix.last().map(String::as_str);
        let chosen = decls
            .iter()
            .find(|decl| module_hint.is_some_and(|hint| file_matches_module(&decl.file, hint)))
            .or_else(|| decls.first())
            .cloned();
        if let Some(decl) = chosen {
            index
                .enums
                .entry(rename.alias.clone())
                .or_default()
                .push(decl);
        }
    }
}

/// Whether a file plausibly hosts module `hint`: `recording.rs`,
/// `recording/mod.rs` or a `mod recording { .. }` in scope (approximated by
/// the file stem or parent directory name).
fn file_matches_module(file: &Path, hint: &str) -> bool {
    let stem = file.file_stem().and_then(|s| s.to_str());
    if stem == Some(hint) {
        return true;
    }
    stem == Some("mod")
        && file
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            == Some(hint)
}

fn collect_renames(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<UseRename>) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_renames(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_renames(item, prefix, out);
            }
        }
        syn::UseTree::Rename(rename) => out.push(UseRename {
            prefix: prefix.clone(),
            original: rename.ident.to_string(),
            alias: rename.rename.to_string(),
        }),
        syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
    }
}

/// A variant's fields with names (if any) and last-segment type names
/// (`Recording(RecordingEvent)` and `Recording(recorder::RecordingEvent)`
/// both yield the type `RecordingEvent`). Single-argument smart pointers
/// (`Box<T>`, `Rc<T>`, `Arc<T>`) are looked through; other generic types
/// are skipped.
fn variant_fields(fields: &syn::Fields) -> Vec<VariantField> {
    fields
        .iter()
        .filter_map(|field| {
            let type_name = unwrapped_type_name(&field.ty)?;
            Some(VariantField {
                name: field.ident.as_ref().map(|i| i.to_string()),
                type_name,
            })
        })
        .collect()
}

/// Generic arguments nest without limit — `Box<Box<Box<…>>>` is valid Rust —
/// and both type walkers below follow that nesting, so hostile input would
/// recurse until the stack ran out. The loader's bracket pre-check does not
/// help here: it counts `(`, `[` and `{`, never `<`. Past the cap the type
/// simply yields no name, which both callers already handle. See
/// `docs/security.md`.
const MAX_TYPE_DEPTH: usize = 64;

fn unwrapped_type_name(ty: &syn::Type) -> Option<String> {
    unwrapped_type_name_at(ty, 0)
}

fn unwrapped_type_name_at(ty: &syn::Type, depth: usize) -> Option<String> {
    if depth >= MAX_TYPE_DEPTH {
        return None;
    }
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    match &segment.arguments {
        syn::PathArguments::None => Some(segment.ident.to_string()),
        syn::PathArguments::AngleBracketed(args)
            if matches!(segment.ident.to_string().as_str(), "Box" | "Rc" | "Arc") =>
        {
            let [syn::GenericArgument::Type(inner)] = args.args.iter().collect::<Vec<_>>()[..]
            else {
                return None;
            };
            unwrapped_type_name_at(inner, depth + 1)
        }
        _ => None,
    }
}

/// The type a struct field ultimately holds, looking through the containers
/// that state can sit inside: smart pointers, interior mutability, `Option`,
/// and the collections a model uses to hold one entry per entity. Maps yield
/// their *value* type — a key is not where state lives.
///
/// Deliberately separate from [`unwrapped_type_name`] rather than a widening of
/// it: that one also feeds composite-state detection, where looking through a
/// `Vec` would change what can be read as a sub-state.
fn reachable_type_name(ty: &syn::Type, depth: usize) -> Option<String> {
    if depth >= MAX_TYPE_DEPTH {
        return None;
    }
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    let args = match &segment.arguments {
        syn::PathArguments::None => return Some(segment.ident.to_string()),
        syn::PathArguments::AngleBracketed(args) => args,
        syn::PathArguments::Parenthesized(_) => return None,
    };
    let types: Vec<&syn::Type> = args
        .args
        .iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect();
    let inner = match (segment.ident.to_string().as_str(), types.as_slice()) {
        (
            "Box" | "Rc" | "Arc" | "RefCell" | "Cell" | "Mutex" | "RwLock" | "Option" | "Vec"
            | "VecDeque" | "BTreeSet" | "HashSet",
            [inner],
        ) => *inner,
        ("HashMap" | "BTreeMap", [_key, value]) => *value,
        _ => return None,
    };
    reachable_type_name(inner, depth + 1)
}

/// Parameter binding names of a function signature.
fn param_names(sig: &syn::Signature) -> Vec<String> {
    sig.inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(typed) => Some(match &*typed.pat {
                syn::Pat::Ident(ident) => ident.ident.to_string(),
                _ => "_".to_string(),
            }),
            syn::FnArg::Receiver(_) => None,
        })
        .collect()
}

fn type_name(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && matches!(&attr.meta, syn::Meta::List(list) if list.tokens.to_string().contains("test"))
    })
}
